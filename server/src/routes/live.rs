use axum::extract::State;
use axum::Json;
use std::sync::Arc;

use leagueeye_shared::models::*;
use crate::AppState;

// Request from client with LCU data
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrichRequest {
    pub phase: String,
    pub players: Vec<EnrichPlayer>,
    pub bans: Vec<LiveBan>,
    pub game_time: Option<i64>,
    pub timer: Option<LiveTimer>,
    pub my_puuid: Option<String>,
    pub queue_id: Option<i32>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrichPlayer {
    pub puuid: Option<String>,
    pub game_name: Option<String>,
    pub tag_line: Option<String>,
    pub champion_id: i64,
    pub assigned_position: Option<String>,
    pub spell1_id: i32,
    pub spell2_id: i32,
    pub team_id: i32,
    pub is_picking: bool,
}

// POST /api/live/enrich
pub async fn enrich_live_game(
    State(state): State<Arc<AppState>>,
    Json(req): Json<EnrichRequest>,
) -> Result<Json<LiveGameData>, String> {
    let mut my_team: Vec<LivePlayer> = Vec::new();
    let mut enemy_team: Vec<LivePlayer> = Vec::new();

    // Determine my team ID
    let my_team_id = req.my_puuid.as_ref()
        .and_then(|my_puuid| req.players.iter().find(|p| p.puuid.as_deref() == Some(my_puuid)))
        .map(|p| p.team_id)
        .unwrap_or(100);

    for p in &req.players {
        let player = LivePlayer {
            puuid: p.puuid.clone(),
            game_name: p.game_name.clone(),
            tag_line: p.tag_line.clone(),
            champion_id: p.champion_id,
            assigned_position: p.assigned_position.clone(),
            spell1_id: p.spell1_id,
            spell2_id: p.spell2_id,
            team_id: p.team_id,
            rank: None,
            is_picking: p.is_picking,
        };
        if p.team_id == my_team_id {
            my_team.push(player);
        } else {
            enemy_team.push(player);
        }
    }

    // Try Spectator API if we have my_puuid and are in-game (to get enemy puuids)
    if req.phase == "in_game" {
        if let Some(ref my_puuid) = req.my_puuid {
            if let Ok(spec_game) = state.riot_api.get_active_game(my_puuid).await {
                // Enrich enemy team with puuids from spectator data
                for spec_p in &spec_game.participants {
                    if spec_p.team_id != my_team_id {
                        if let Some(enemy) = enemy_team.iter_mut().find(|e| e.champion_id == spec_p.champion_id) {
                            enemy.puuid = spec_p.puuid.clone();
                            if let Some(riot_id) = &spec_p.riot_id {
                                if let Some((gn, tl)) = riot_id.split_once('#') {
                                    enemy.game_name = Some(gn.to_string());
                                    enemy.tag_line = Some(tl.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Enrich all players with rank data
    let all_puuids: Vec<Option<String>> = my_team.iter()
        .chain(enemy_team.iter())
        .map(|p| p.puuid.clone())
        .collect();

    let rank_futs: Vec<_> = all_puuids.iter()
        .map(|puuid_opt| {
            let state = state.clone();
            let puuid_opt = puuid_opt.clone();
            async move {
                if let Some(puuid) = &puuid_opt {
                    state.riot_api.get_league_entries_by_puuid(puuid).await.ok()
                } else {
                    None
                }
            }
        })
        .collect();
    let ranks = futures::future::join_all(rank_futs).await;

    let my_count = my_team.len();
    for (i, entries_opt) in ranks.into_iter().enumerate() {
        if let Some(entries) = entries_opt {
            let ri = build_rank_info(entries);
            let rank = ri.iter().find(|r| r.queue_type == "RANKED_SOLO_5x5")
                .or(ri.first())
                .cloned();
            if i < my_count {
                my_team[i].rank = rank;
            } else {
                enemy_team[i - my_count].rank = rank;
            }
        }
    }

    Ok(Json(LiveGameData {
        phase: req.phase,
        queue_id: req.queue_id,
        my_team,
        enemy_team,
        bans: req.bans,
        game_time: req.game_time,
        timer: req.timer,
    }))
}
