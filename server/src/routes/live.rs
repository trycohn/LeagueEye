use axum::extract::State;
use axum::Json;
use std::sync::Arc;

use leagueeye_shared::models::*;
use crate::{AppState, CachedRank, CachedPuuid, RANK_CACHE_TTL, PUUID_CACHE_TTL};

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

fn parse_riot_id(riot_id: &str) -> Option<(&str, &str)> {
    riot_id.split_once('#')
}

fn relative_team_id_from_spectator(spec_team_id: i32, my_spec_team_id: i32) -> i32 {
    if spec_team_id == my_spec_team_id { 100 } else { 200 }
}

fn find_matching_player_index(
    players: &[LivePlayer],
    spec_p: &SpectatorParticipant,
) -> Result<usize, &'static str> {
    if let Some(spec_puuid) = spec_p.puuid.as_deref() {
        let puuid_matches: Vec<_> = players.iter().enumerate()
            .filter(|(_, player)| player.puuid.as_deref() == Some(spec_puuid))
            .map(|(idx, _)| idx)
            .collect();
        match puuid_matches.as_slice() {
            [idx] => return Ok(*idx),
            [] => {},
            _ => return Err("multiple puuid matches"),
        }
    }

    if let Some((game_name, tag_line)) = spec_p.riot_id.as_deref().and_then(parse_riot_id) {
        let riot_id_matches: Vec<_> = players.iter().enumerate()
            .filter(|(_, player)| {
                player.game_name.as_deref() == Some(game_name)
                    && player
                        .tag_line
                        .as_deref()
                        .map(|value| value == tag_line)
                        .unwrap_or(true)
            })
            .map(|(idx, _)| idx)
            .collect();
        match riot_id_matches.as_slice() {
            [idx] => return Ok(*idx),
            [] => {},
            _ => return Err("multiple riot id matches"),
        }
    }

    let champion_matches: Vec<_> = players.iter().enumerate()
        .filter(|(_, player)| player.champion_id == spec_p.champion_id)
        .map(|(idx, _)| idx)
        .collect();
    match champion_matches.as_slice() {
        [idx] => Ok(*idx),
        [] => Err("no matching champion slot"),
        _ => Err("multiple champion matches"),
    }
}

fn hydrate_player_from_spectator(
    players: &mut [LivePlayer],
    spec_p: &SpectatorParticipant,
) -> Result<(), &'static str> {
    let idx = find_matching_player_index(players, spec_p)?;
    let player = &mut players[idx];

    if player.puuid.is_none() {
        player.puuid = spec_p.puuid.clone();
    }

    if let Some((game_name, tag_line)) = spec_p.riot_id.as_deref().and_then(parse_riot_id) {
        player.game_name = Some(game_name.to_string());
        player.tag_line = Some(tag_line.to_string());
    }

    Ok(())
}

fn maybe_hydrate_teams_from_spectator(
    my_puuid: &str,
    my_team: &mut [LivePlayer],
    enemy_team: &mut [LivePlayer],
    spec_game: Option<&SpectatorGame>,
) -> Result<(), &'static str> {
    let Some(spec_game) = spec_game else {
        return Ok(());
    };

    let my_spec_team_id = spec_game
        .participants
        .iter()
        .find(|participant| participant.puuid.as_deref() == Some(my_puuid))
        .map(|participant| participant.team_id)
        .ok_or("spectator game missing my puuid")?;

    for spec_p in &spec_game.participants {
        let relative_team_id = relative_team_id_from_spectator(spec_p.team_id, my_spec_team_id);
        let result = if relative_team_id == 100 {
            hydrate_player_from_spectator(my_team, spec_p)
        } else {
            hydrate_player_from_spectator(enemy_team, spec_p)
        };

        if let Err(reason) = result {
            log::warn!(
                "[live] Spectator hydration skipped for team {} champion {}: {}",
                relative_team_id,
                spec_p.champion_id,
                reason
            );
        }
    }

    Ok(())
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

    // Try Spectator API if we have my_puuid and are in-game.
    // Spectator team IDs are actual Riot side IDs, so map them back to our
    // normalized 100=my team / 200=enemy team buckets before hydrating players.
    if req.phase == "in_game" {
        if let Some(ref my_puuid) = req.my_puuid {
            match state.riot_api.get_active_game_fast(my_puuid).await {
                Ok(spec_game) => {
                    if spec_game.is_none() {
                        log::debug!(
                            "[live] Spectator skipped: Riot API did not return summonerId for this puuid"
                        );
                    }
                    if let Err(reason) = maybe_hydrate_teams_from_spectator(
                        my_puuid,
                        &mut my_team,
                        &mut enemy_team,
                        spec_game.as_ref(),
                    ) {
                        log::warn!("[live] Spectator hydration skipped: {}", reason);
                    }
                }
                Err(err) => {
                    log::warn!("[live] Spectator lookup failed: {}", err);
                }
            }
        }
    }

    // Resolve missing puuids via cache, then Riot Account API (game_name + tag_line)
    {
        let all_players_info: Vec<_> = my_team.iter().chain(enemy_team.iter())
            .map(|p| (p.puuid.clone(), p.game_name.clone(), p.tag_line.clone()))
            .collect();

        let resolve_futs: Vec<_> = all_players_info.iter()
            .map(|(puuid, game_name, tag_line)| {
                let state = state.clone();
                let puuid = puuid.clone();
                let gn = game_name.clone();
                let tl = tag_line.clone();
                async move {
                    if puuid.is_some() { return puuid; }
                    if let (Some(gn), Some(tl)) = (gn, tl) {
                        let cache_key = format!("{}#{}", gn.to_lowercase(), tl.to_lowercase());

                        // Check puuid cache first
                        {
                            let cache = state.puuid_cache.lock().await;
                            if let Some(cached) = cache.get(&cache_key) {
                                if cached.fetched_at.elapsed() < PUUID_CACHE_TTL {
                                    return Some(cached.puuid.clone());
                                }
                            }
                        }

                        match state.riot_api.get_account_by_riot_id_fast(&gn, &tl).await {
                            Ok(acc) => {
                                // Store in cache
                                {
                                    let mut cache = state.puuid_cache.lock().await;
                                    cache.insert(cache_key, CachedPuuid {
                                        puuid: acc.puuid.clone(),
                                        fetched_at: std::time::Instant::now(),
                                    });
                                }
                                Some(acc.puuid)
                            }
                            Err(e) => {
                                log::debug!("[live] Riot ID resolve failed for {}#{}: {}", gn, tl, e);
                                None
                            }
                        }
                    } else {
                        None
                    }
                }
            })
            .collect();

        let resolved = futures::future::join_all(resolve_futs).await;
        let my_count = my_team.len();
        for (i, puuid) in resolved.into_iter().enumerate() {
            if let Some(puuid) = puuid {
                if i < my_count {
                    my_team[i].puuid = Some(puuid);
                } else {
                    enemy_team[i - my_count].puuid = Some(puuid);
                }
            }
        }
    }

    // Enrich all players with rank data (cache + Riot league-v4 by puuid).
    let all_puuids: Vec<Option<String>> = my_team.iter()
        .chain(enemy_team.iter())
        .map(|p| p.puuid.clone())
        .collect();

    let rank_futs: Vec<_> = all_puuids.iter()
        .map(|puuid_opt| {
            let state = state.clone();
            let puuid_opt = puuid_opt.clone();
            async move {
                // 1. Try rank cache by puuid
                if let Some(ref puuid) = puuid_opt {
                    {
                        let cache = state.rank_cache.lock().await;
                        if let Some(cached) = cache.get(puuid.as_str()) {
                            if cached.fetched_at.elapsed() < RANK_CACHE_TTL {
                                return cached.rank.clone();
                            }
                        }
                    }

                    // Cache miss — fetch from Riot API
                    if let Ok(entries) = state.riot_api.get_league_entries_by_puuid_fast(puuid).await {
                        let ri = build_rank_info(entries);
                        let rank = ri.iter().find(|r| r.queue_type == "RANKED_SOLO_5x5")
                            .or(ri.first())
                            .cloned();

                        // Store in cache
                        {
                            let mut cache = state.rank_cache.lock().await;
                            cache.insert(puuid.clone(), CachedRank {
                                rank: rank.clone(),
                                fetched_at: std::time::Instant::now(),
                            });
                        }
                        return rank;
                    }
                }

                None
            }
        })
        .collect();
    let ranks = futures::future::join_all(rank_futs).await;

    let my_count = my_team.len();
    for (i, rank) in ranks.into_iter().enumerate() {
        if i < my_count {
            my_team[i].rank = rank;
        } else {
            enemy_team[i - my_count].rank = rank;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn live_player(
        team_id: i32,
        champion_id: i64,
        puuid: Option<&str>,
        game_name: Option<&str>,
        tag_line: Option<&str>,
    ) -> LivePlayer {
        LivePlayer {
            puuid: puuid.map(String::from),
            game_name: game_name.map(String::from),
            tag_line: tag_line.map(String::from),
            champion_id,
            assigned_position: None,
            spell1_id: 0,
            spell2_id: 0,
            team_id,
            rank: None,
            is_picking: false,
        }
    }

    fn spectator_player(
        team_id: i32,
        champion_id: i64,
        puuid: Option<&str>,
        riot_id: Option<&str>,
    ) -> SpectatorParticipant {
        SpectatorParticipant {
            puuid: puuid.map(String::from),
            team_id,
            champion_id,
            spell1_id: None,
            spell2_id: None,
            riot_id: riot_id.map(String::from),
            summoner_id: None,
        }
    }

    #[test]
    fn relative_team_id_uses_actual_spectator_side() {
        assert_eq!(relative_team_id_from_spectator(200, 200), 100);
        assert_eq!(relative_team_id_from_spectator(100, 200), 200);
    }

    #[test]
    fn hydrate_player_from_spectator_prefers_riot_id_match() {
        let mut players = vec![
            live_player(100, 157, None, Some("Tryconn"), None),
            live_player(100, 157, None, None, None),
        ];
        let participant = spectator_player(200, 157, Some("ally-puuid"), Some("Tryconn#EUW"));
        hydrate_player_from_spectator(&mut players, &participant).unwrap();

        assert_eq!(players[0].puuid.as_deref(), Some("ally-puuid"));
        assert_eq!(players[0].game_name.as_deref(), Some("Tryconn"));
        assert_eq!(players[0].tag_line.as_deref(), Some("EUW"));
        assert!(players[1].puuid.is_none());
    }

    #[test]
    fn hydrate_player_from_spectator_rejects_ambiguous_champion_only_match() {
        let mut players = vec![
            live_player(200, 238, None, None, None),
            live_player(200, 238, None, None, None),
        ];
        let participant = spectator_player(100, 238, Some("enemy-puuid"), None);
        let err = hydrate_player_from_spectator(&mut players, &participant).unwrap_err();

        assert_eq!(err, "multiple champion matches");
        assert!(players.iter().all(|player| player.puuid.is_none()));
    }

    fn spectator_game(participants: Vec<SpectatorParticipant>) -> SpectatorGame {
        SpectatorGame {
            game_id: 1,
            game_mode: "CLASSIC".to_string(),
            game_type: Some("MATCHED_GAME".to_string()),
            game_queue_config_id: Some(420),
            participants,
            banned_champions: None,
            game_start_time: Some(0),
            game_length: Some(0),
        }
    }

    #[test]
    fn maybe_hydrate_teams_from_spectator_is_noop_without_game() {
        let mut my_team = vec![live_player(100, 157, Some("me"), Some("Tryconn"), Some("EUW"))];
        let mut enemy_team = vec![live_player(200, 238, None, Some("Enemy"), Some("EUW"))];
        let my_before = my_team[0].clone();
        let enemy_before = enemy_team[0].clone();

        maybe_hydrate_teams_from_spectator("me", &mut my_team, &mut enemy_team, None).unwrap();

        assert_eq!(my_team[0].puuid, my_before.puuid);
        assert_eq!(my_team[0].game_name, my_before.game_name);
        assert_eq!(my_team[0].tag_line, my_before.tag_line);
        assert_eq!(enemy_team[0].puuid, enemy_before.puuid);
        assert_eq!(enemy_team[0].game_name, enemy_before.game_name);
        assert_eq!(enemy_team[0].tag_line, enemy_before.tag_line);
    }

    #[test]
    fn maybe_hydrate_teams_from_spectator_updates_both_teams() {
        let mut my_team = vec![live_player(100, 157, Some("me"), Some("Tryconn"), Some("EUW"))];
        let mut enemy_team = vec![live_player(200, 238, None, None, None)];
        let spec_game = spectator_game(vec![
            spectator_player(200, 157, Some("me"), Some("Tryconn#EUW")),
            spectator_player(100, 238, Some("enemy-puuid"), Some("Enemy#EUW")),
        ]);

        maybe_hydrate_teams_from_spectator("me", &mut my_team, &mut enemy_team, Some(&spec_game)).unwrap();

        assert_eq!(enemy_team[0].puuid.as_deref(), Some("enemy-puuid"));
        assert_eq!(enemy_team[0].game_name.as_deref(), Some("Enemy"));
        assert_eq!(enemy_team[0].tag_line.as_deref(), Some("EUW"));
    }
}
