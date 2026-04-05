use axum::extract::{Path, Query, State};
use axum::Json;
use std::sync::Arc;
use std::collections::HashSet;

use leagueeye_shared::models::*;
use crate::AppState;

// GET /api/players/{game_name}/{tag_line}
pub async fn search_player(
    State(state): State<Arc<AppState>>,
    Path((game_name, tag_line)): Path<(String, String)>,
) -> Result<Json<PlayerProfile>, String> {
    let account = state.riot_api.get_account_by_riot_id(&game_name, &tag_line).await?;
    let summoner = state.riot_api.get_summoner_by_puuid(&account.puuid).await?;
    let entries = match &summoner.id {
        Some(id) => state.riot_api.get_league_entries(id).await?,
        None => state.riot_api.get_league_entries_by_puuid(&account.puuid).await.unwrap_or_default(),
    };

    let ranked = build_rank_info(entries);

    let profile = PlayerProfile {
        puuid: account.puuid.clone(),
        game_name: account.game_name.unwrap_or(game_name),
        tag_line: account.tag_line.unwrap_or(tag_line),
        summoner_level: summoner.summoner_level,
        profile_icon_id: summoner.profile_icon_id,
        ranked: ranked.clone(),
    };

    let _ = state.db.save_account(&profile).await;
    let _ = state.db.save_rank_snapshot(&account.puuid, &ranked).await;

    Ok(Json(profile))
}

// GET /api/players/{puuid}/mastery
pub async fn get_mastery(
    State(state): State<Arc<AppState>>,
    Path(puuid): Path<String>,
) -> Result<Json<Vec<MasteryInfo>>, String> {
    let masteries = state.riot_api.get_champion_mastery_top(&puuid, 10).await?;
    let result: Vec<MasteryInfo> = masteries.into_iter().map(|m| MasteryInfo {
        champion_id: m.champion_id,
        champion_level: m.champion_level,
        champion_points: m.champion_points,
    }).collect();
    Ok(Json(result))
}

// GET /api/players/{puuid}/matches?offset=0&limit=15
#[derive(serde::Deserialize)]
pub struct MatchesQuery {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

const INITIAL_MATCHES: usize = 15;
const FETCH_TOTAL: usize = 500;

pub async fn get_matches_and_stats(
    State(state): State<Arc<AppState>>,
    Path(puuid): Path<String>,
    Query(query): Query<MatchesQuery>,
) -> Result<Json<MatchesAndStats>, String> {
    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(INITIAL_MATCHES as i64);

    // If offset > 0, this is a "load more" request
    if offset > 0 {
        let matches = state.db.get_cached_matches_paged(&puuid, offset, limit).await
            .map_err(|e| e.to_string())?;
        return Ok(Json(MatchesAndStats {
            matches,
            champion_stats: vec![],
            total_cached: 0,
            total_wins: 0,
            total_losses: 0,
        }));
    }

    let cached_ids = state.db.get_cached_match_ids(&puuid).await
        .map_err(|e| e.to_string())?;

    let has_cache = !cached_ids.is_empty();

    if has_cache {
        let initial_matches = state.db.get_cached_matches_paged(&puuid, 0, INITIAL_MATCHES as i64).await
            .map_err(|e| e.to_string())?;

        let all_cached = state.db.get_cached_matches_paged(&puuid, 0, 500).await
            .map_err(|e| e.to_string())?;

        let champion_stats = build_champion_stats(&all_cached);
        let total_wins = all_cached.iter().filter(|m| m.win).count() as i32;
        let total_losses = all_cached.len() as i32 - total_wins;

        let total_cached = state.db.count_cached_matches(&puuid).await
            .map_err(|e| e.to_string())?;

        // Background: fetch new matches
        let state_clone = state.clone();
        let puuid_clone = puuid.clone();
        let cached_set: HashSet<String> = cached_ids.into_iter().collect();
        tokio::spawn(async move {
            if let Ok(fresh_ids) = state_clone.riot_api.get_all_match_ids(&puuid_clone, FETCH_TOTAL).await {
                let new_ids: Vec<String> = fresh_ids.iter()
                    .filter(|id| !cached_set.contains(*id))
                    .cloned()
                    .collect();

                for chunk in new_ids.chunks(5) {
                    let dtos = state_clone.riot_api.get_matches_parallel(chunk, 5).await;
                    let summaries: Vec<MatchSummary> = dtos.iter().filter_map(|m| {
                        dto_to_summary(m, &puuid_clone, |puuid, start, end| {
                            // No LP delta for background fetches on server (no blocking DB call in async)
                            let _ = (puuid, start, end);
                            None
                        })
                    }).collect();
                    let _ = state_clone.db.save_matches(&puuid_clone, &summaries).await;
                    for dto in &dtos {
                        let parts = dto_to_participants(dto);
                        let _ = state_clone.db.save_match_participants(&dto.metadata.match_id, &parts).await;
                    }
                }
            }
        });

        return Ok(Json(MatchesAndStats {
            matches: initial_matches,
            champion_stats,
            total_cached,
            total_wins,
            total_losses,
        }));
    }

    // First request: no cache — fetch first 15 synchronously
    let fresh_ids = state.riot_api.get_all_match_ids(&puuid, FETCH_TOTAL).await?;

    let first_batch: Vec<String> = fresh_ids.iter().take(INITIAL_MATCHES).cloned().collect();
    let remaining_ids: Vec<String> = fresh_ids.iter().skip(INITIAL_MATCHES).cloned().collect();

    if !first_batch.is_empty() {
        let dtos = state.riot_api.get_matches_parallel(&first_batch, 5).await;
        let summaries: Vec<MatchSummary> = dtos.iter().filter_map(|m| {
            dto_to_summary(m, &puuid, |_, _, _| None)
        }).collect();
        let _ = state.db.save_matches(&puuid, &summaries).await;
        for dto in &dtos {
            let parts = dto_to_participants(dto);
            let _ = state.db.save_match_participants(&dto.metadata.match_id, &parts).await;
        }
    }

    let initial_matches = state.db.get_cached_matches_paged(&puuid, 0, INITIAL_MATCHES as i64).await
        .map_err(|e| e.to_string())?;

    let all_cached = state.db.get_cached_matches_paged(&puuid, 0, 500).await
        .map_err(|e| e.to_string())?;
    let champion_stats = build_champion_stats(&all_cached);
    let total_wins = all_cached.iter().filter(|m| m.win).count() as i32;
    let total_losses = all_cached.len() as i32 - total_wins;

    let total_cached = state.db.count_cached_matches(&puuid).await
        .map_err(|e| e.to_string())?;

    // Background fetch remaining
    if !remaining_ids.is_empty() {
        let state_clone = state.clone();
        let puuid_clone = puuid.clone();
        tokio::spawn(async move {
            for chunk in remaining_ids.chunks(5) {
                let dtos = state_clone.riot_api.get_matches_parallel(chunk, 5).await;
                let summaries: Vec<MatchSummary> = dtos.iter().filter_map(|m| {
                    dto_to_summary(m, &puuid_clone, |_, _, _| None)
                }).collect();
                let _ = state_clone.db.save_matches(&puuid_clone, &summaries).await;
                for dto in &dtos {
                    let parts = dto_to_participants(dto);
                    let _ = state_clone.db.save_match_participants(&dto.metadata.match_id, &parts).await;
                }
            }
        });
    }

    Ok(Json(MatchesAndStats {
        matches: initial_matches,
        champion_stats,
        total_cached,
        total_wins,
        total_losses,
    }))
}
