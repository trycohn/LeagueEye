use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::State;

use crate::db::Db;
use crate::lcu;
use crate::models::*;
use crate::riot_api::RiotApiClient;

type SharedDb = Arc<Mutex<Db>>;

// ─── FetchProgress (фоновая загрузка матчей) ─────────────────────────────────

pub struct FetchProgress {
    pub is_fetching: bool,
    pub generation: u64,
}

impl FetchProgress {
    pub fn new() -> Self {
        Self { is_fetching: false, generation: 0 }
    }
}

// ─── search_player ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn search_player(
    api: State<'_, RiotApiClient>,
    db: State<'_, SharedDb>,
    game_name: String,
    tag_line: String,
) -> Result<PlayerProfile, String> {
    let account = api.get_account_by_riot_id(&game_name, &tag_line).await?;
    let summoner = api.get_summoner_by_puuid(&account.puuid).await?;
    let entries = match &summoner.id {
        Some(id) => api.get_league_entries(id).await?,
        None => api
            .get_league_entries_by_puuid(&account.puuid)
            .await
            .unwrap_or_default(),
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

    if let Ok(db) = db.lock() {
        let _ = db.save_rank_snapshot(&account.puuid, &ranked);
    }

    Ok(profile)
}

// ─── detect_account ──────────────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DetectedAccount {
    pub puuid: String,
    pub game_name: String,
    pub tag_line: String,
    pub profile_icon_id: i64,
    pub summoner_level: i64,
    pub ranked: Vec<RankInfo>,
}

#[tauri::command]
pub async fn detect_account(
    api: State<'_, RiotApiClient>,
    db: State<'_, SharedDb>,
) -> Result<DetectedAccount, String> {
    let creds = lcu::detect_lcu_credentials()
        .ok_or_else(|| "League Client не запущен".to_string())?;

    let identity = lcu::get_lcu_identity(&creds).await?;

    let account = api.get_account_by_riot_id(&identity.game_name, &identity.tag_line).await
        .map_err(|e| format!("Riot account error: {}", e))?;

    let summoner = api.get_summoner_by_puuid(&account.puuid).await?;

    let entries = match &summoner.id {
        Some(id) => api.get_league_entries(id).await?,
        None => api.get_league_entries_by_puuid(&account.puuid).await.unwrap_or_default(),
    };
    let ranked = build_rank_info(entries);

    let profile = PlayerProfile {
        puuid: account.puuid.clone(),
        game_name: identity.game_name.clone(),
        tag_line: identity.tag_line.clone(),
        summoner_level: summoner.summoner_level,
        profile_icon_id: summoner.profile_icon_id,
        ranked: ranked.clone(),
    };

    if let Ok(db) = db.lock() {
        let _ = db.save_account(&profile);
        let _ = db.save_rank_snapshot(&account.puuid, &ranked);
    }

    Ok(DetectedAccount {
        puuid: profile.puuid,
        game_name: profile.game_name,
        tag_line: profile.tag_line,
        profile_icon_id: profile.profile_icon_id,
        summoner_level: profile.summoner_level,
        ranked,
    })
}

// ─── poll_client_status ───────────────────────────────────────────────────────

#[tauri::command]
pub async fn poll_client_status() -> bool {
    lcu::is_lcu_running()
}

// ─── get_cached_profile ───────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_cached_profile(
    db: State<'_, SharedDb>,
) -> Result<Option<DetectedAccount>, String> {
    let db = db.lock().map_err(|e| e.to_string())?;
    let account = db.get_last_account().map_err(|e| e.to_string())?;
    match account {
        Some(acc) => {
            let ranked = db.get_latest_ranks(&acc.puuid).unwrap_or_default();
            Ok(Some(DetectedAccount {
                puuid: acc.puuid,
                game_name: acc.game_name,
                tag_line: acc.tag_line,
                profile_icon_id: acc.profile_icon_id,
                summoner_level: acc.summoner_level,
                ranked,
            }))
        }
        None => Ok(None),
    }
}

// ─── get_mastery ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_mastery(
    api: State<'_, RiotApiClient>,
    puuid: String,
) -> Result<Vec<MasteryInfo>, String> {
    let masteries = api.get_champion_mastery_top(&puuid, 10).await?;
    Ok(masteries
        .into_iter()
        .map(|m| MasteryInfo {
            champion_id: m.champion_id,
            champion_level: m.champion_level,
            champion_points: m.champion_points,
        })
        .collect())
}

// ─── get_matches_and_stats ────────────────────────────────────────────────────

const INITIAL_MATCHES: usize = 15;
const FETCH_TOTAL: usize = 500;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchesAndStats {
    pub matches: Vec<MatchSummary>,
    pub champion_stats: Vec<ChampionStat>,
    pub total_cached: i64,
    pub total_wins: i32,
    pub total_losses: i32,
}

fn dto_to_participants(m: &crate::models::MatchDto) -> Vec<MatchParticipantDetail> {
    m.info.participants.iter().map(|p| {
        let cs = p.total_minions_killed + p.neutral_minions_killed.unwrap_or(0);
        MatchParticipantDetail {
            puuid: p.puuid.clone(),
            riot_id_name: p.riot_id_game_name.clone().unwrap_or_default(),
            riot_id_tagline: p.riot_id_tagline.clone().unwrap_or_default(),
            champion_id: p.champion_id,
            champion_name: p.champion_name.clone(),
            champ_level: p.champ_level,
            team_id: p.team_id,
            win: p.win,
            kills: p.kills,
            deaths: p.deaths,
            assists: p.assists,
            cs,
            gold: p.gold_earned,
            damage: p.total_damage_dealt_to_champions,
            damage_taken: p.total_damage_taken.unwrap_or(0),
            vision_score: p.vision_score.unwrap_or(0),
            wards_placed: p.wards_placed.unwrap_or(0),
            wards_killed: p.wards_killed.unwrap_or(0),
            position: p.team_position.clone()
                .or_else(|| p.individual_position.clone())
                .unwrap_or_else(|| "UNKNOWN".to_string()),
            items: vec![p.item0, p.item1, p.item2, p.item3, p.item4, p.item5, p.item6],
            summoner_spells: vec![p.summoner1_id.unwrap_or(0), p.summoner2_id.unwrap_or(0)],
            double_kills: p.double_kills.unwrap_or(0),
            triple_kills: p.triple_kills.unwrap_or(0),
            quadra_kills: p.quadra_kills.unwrap_or(0),
            penta_kills: p.penta_kills.unwrap_or(0),
        }
    }).collect()
}

fn dto_to_summary(m: &crate::models::MatchDto, puuid: &str, db: &crate::db::Db) -> Option<MatchSummary> {
    let p = m.info.participants.iter().find(|p| p.puuid == puuid)?;
    let cs = p.total_minions_killed + p.neutral_minions_killed.unwrap_or(0);
    let game_end_ms = m.info.game_creation + m.info.game_duration * 1000;
    let lp_delta = db.get_lp_at(puuid, m.info.game_creation, true)
        .and_then(|before| db.get_lp_at(puuid, game_end_ms, false).map(|after| after - before));
    Some(MatchSummary {
        match_id: m.metadata.match_id.clone(),
        champion_name: p.champion_name.clone(),
        champion_id: p.champion_id,
        win: p.win,
        kills: p.kills,
        deaths: p.deaths,
        assists: p.assists,
        cs,
        gold: p.gold_earned,
        damage: p.total_damage_dealt_to_champions,
        vision_score: p.vision_score.unwrap_or(0),
        position: p.team_position.clone()
            .or_else(|| p.individual_position.clone())
            .unwrap_or_else(|| "UNKNOWN".to_string()),
        game_duration: m.info.game_duration,
        game_creation: m.info.game_creation,
        queue_id: m.info.queue_id,
        items: vec![p.item0, p.item1, p.item2, p.item3, p.item4, p.item5, p.item6],
        summoner_spells: vec![p.summoner1_id.unwrap_or(0), p.summoner2_id.unwrap_or(0)],
        lp_delta,
    })
}

fn build_champion_stats(matches: &[MatchSummary]) -> Vec<ChampionStat> {
    struct StatAccum {
        champ_id: i64,
        champ_name: String,
        games: i32,
        wins: i32,
        kills: i32,
        deaths: i32,
        assists: i32,
        cs: i32,
        positions: HashMap<String, i32>,
    }

    let mut stats_map: HashMap<String, StatAccum> = HashMap::new();
    for m in matches {
        let entry = stats_map
            .entry(m.champion_name.clone())
            .or_insert_with(|| StatAccum {
                champ_id: m.champion_id,
                champ_name: m.champion_name.clone(),
                games: 0, wins: 0, kills: 0, deaths: 0, assists: 0, cs: 0,
                positions: HashMap::new(),
            });
        entry.games += 1;
        if m.win { entry.wins += 1; }
        entry.kills += m.kills;
        entry.deaths += m.deaths;
        entry.assists += m.assists;
        entry.cs += m.cs;
        if !m.position.is_empty() && m.position != "UNKNOWN" {
            *entry.positions.entry(m.position.clone()).or_insert(0) += 1;
        }
    }

    let mut stats: Vec<ChampionStat> = stats_map.into_values().map(|s| {
        let g = s.games as f64;
        let top_position = s.positions.iter()
            .max_by_key(|(_, &cnt)| cnt)
            .map(|(pos, _)| pos.clone())
            .unwrap_or_default();
        ChampionStat {
            champion_id: s.champ_id,
            champion_name: s.champ_name,
            games: s.games,
            wins: s.wins,
            winrate: ((s.wins as f64 / g) * 1000.0).round() / 10.0,
            avg_kills: (s.kills as f64 / g * 10.0).round() / 10.0,
            avg_deaths: (s.deaths as f64 / g * 10.0).round() / 10.0,
            avg_assists: (s.assists as f64 / g * 10.0).round() / 10.0,
            avg_cs: (s.cs as f64 / g * 10.0).round() / 10.0,
            position: top_position,
        }
    }).collect();
    stats.sort_by(|a, b| b.games.cmp(&a.games));
    stats
}

#[tauri::command]
pub async fn get_matches_and_stats(
    api: State<'_, RiotApiClient>,
    db: State<'_, SharedDb>,
    fetch_progress: State<'_, Arc<Mutex<FetchProgress>>>,
    puuid: String,
    count: Option<u32>,
) -> Result<MatchesAndStats, String> {
    let _ = count;

    let cached_ids: Vec<String> = db
        .lock().map_err(|e| e.to_string())?
        .get_cached_match_ids(&puuid)
        .unwrap_or_default();

    let has_cache = !cached_ids.is_empty();

    if has_cache {
        let initial_matches = db.lock().map_err(|e| e.to_string())?
            .get_cached_matches_paged(&puuid, 0, INITIAL_MATCHES)
            .unwrap_or_default();

        let all_cached = db.lock().map_err(|e| e.to_string())?
            .get_cached_matches(&puuid, 500)
            .unwrap_or_default();

        let champion_stats = build_champion_stats(&all_cached);
        let total_wins = all_cached.iter().filter(|m| m.win).count() as i32;
        let total_losses = all_cached.len() as i32 - total_wins;

        let total_cached = db.lock().map_err(|e| e.to_string())?
            .count_cached_matches(&puuid)
            .unwrap_or(0);

        // Фоновое обновление: ищем новые матчи и дозагружаем
        {
            let mut p = fetch_progress.lock().unwrap();
            p.generation += 1;
            p.is_fetching = true;
            let my_gen = p.generation;
            let api_clone = api.inner().clone();
            let db_arc: SharedDb = Arc::clone(&*db);
            let progress_arc = Arc::clone(&*fetch_progress);
            let puuid_clone = puuid.clone();
            let cached_set: std::collections::HashSet<String> =
                cached_ids.into_iter().collect();

            tokio::spawn(async move {
                if let Ok(fresh_ids) = api_clone.get_all_match_ids(&puuid_clone, FETCH_TOTAL).await {
                    if progress_arc.lock().unwrap().generation != my_gen { return; }

                    let new_ids: Vec<String> = fresh_ids.iter()
                        .filter(|id| !cached_set.contains(*id))
                        .cloned()
                        .collect();

                    for chunk in new_ids.chunks(5) {
                        if progress_arc.lock().unwrap().generation != my_gen { return; }
                        let dtos = api_clone.get_matches_parallel(chunk, 5).await;
                        let summaries: Vec<MatchSummary> = {
                            if let Ok(db_guard) = db_arc.lock() {
                                dtos.iter().filter_map(|m| dto_to_summary(m, &puuid_clone, &db_guard)).collect()
                            } else { vec![] }
                        };
                        if let Ok(db_guard) = db_arc.lock() {
                            let _ = db_guard.save_matches(&puuid_clone, &summaries);
                            for dto in &dtos {
                                let parts = dto_to_participants(dto);
                                let _ = db_guard.save_match_participants(&dto.metadata.match_id, dto.info.game_duration, &parts);
                            }
                        }
                    }
                }
                let mut p = progress_arc.lock().unwrap();
                if p.generation == my_gen { p.is_fetching = false; }
            });
        }

        return Ok(MatchesAndStats {
            matches: initial_matches,
            champion_stats,
            total_cached,
            total_wins,
            total_losses,
        });
    }

    // Первый запуск: кеша нет — ждём первые 15 матчей синхронно
    let fresh_ids = api.get_all_match_ids(&puuid, FETCH_TOTAL).await?;

    let first_batch: Vec<String> = fresh_ids.iter().take(INITIAL_MATCHES).cloned().collect();
    let remaining_ids: Vec<String> = fresh_ids.iter().skip(INITIAL_MATCHES).cloned().collect();

    if !first_batch.is_empty() {
        let dtos = api.get_matches_parallel(&first_batch, 5).await;
        let summaries: Vec<MatchSummary> = {
            let db_guard = db.lock().map_err(|e| e.to_string())?;
            dtos.iter().filter_map(|m| dto_to_summary(m, &puuid, &db_guard)).collect()
        };
        if let Ok(db_guard) = db.lock() {
            let _ = db_guard.save_matches(&puuid, &summaries);
            for dto in &dtos {
                let parts = dto_to_participants(dto);
                let _ = db_guard.save_match_participants(&dto.metadata.match_id, dto.info.game_duration, &parts);
            }
        }
    }

    let initial_matches = db.lock().map_err(|e| e.to_string())?
        .get_cached_matches_paged(&puuid, 0, INITIAL_MATCHES)
        .unwrap_or_default();

    let all_cached = db.lock().map_err(|e| e.to_string())?
        .get_cached_matches(&puuid, 500)
        .unwrap_or_default();
    let champion_stats = build_champion_stats(&all_cached);
    let total_wins = all_cached.iter().filter(|m| m.win).count() as i32;
    let total_losses = all_cached.len() as i32 - total_wins;

    let total_cached = db.lock().map_err(|e| e.to_string())?
        .count_cached_matches(&puuid)
        .unwrap_or(0);

    // Фоновая загрузка остального
    if !remaining_ids.is_empty() {
        let api_clone = api.inner().clone();
        let db_arc: SharedDb = Arc::clone(&*db);
        let progress_arc = Arc::clone(&*fetch_progress);
        let puuid_clone = puuid.clone();

        let my_gen = {
            let mut p = progress_arc.lock().unwrap();
            p.generation += 1;
            p.is_fetching = true;
            p.generation
        };

        tokio::spawn(async move {
            for chunk in remaining_ids.chunks(5) {
                if progress_arc.lock().unwrap().generation != my_gen { return; }
                let dtos = api_clone.get_matches_parallel(chunk, 5).await;
                let summaries: Vec<MatchSummary> = {
                    if let Ok(db_guard) = db_arc.lock() {
                        dtos.iter().filter_map(|m| dto_to_summary(m, &puuid_clone, &db_guard)).collect()
                    } else { vec![] }
                };
                if let Ok(db_guard) = db_arc.lock() {
                    let _ = db_guard.save_matches(&puuid_clone, &summaries);
                    for dto in &dtos {
                        let parts = dto_to_participants(dto);
                        let _ = db_guard.save_match_participants(&dto.metadata.match_id, dto.info.game_duration, &parts);
                    }
                }
            }
            let mut p = progress_arc.lock().unwrap();
            if p.generation == my_gen { p.is_fetching = false; }
        });
    }

    Ok(MatchesAndStats {
        matches: initial_matches,
        champion_stats,
        total_cached,
        total_wins,
        total_losses,
    })
}

// ─── get_match_detail ─────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_match_detail(
    api: State<'_, RiotApiClient>,
    db: State<'_, SharedDb>,
    match_id: String,
) -> Result<MatchDetail, String> {
    // Try cache first
    if let Some(detail) = db.lock().map_err(|e| e.to_string())?
        .get_match_detail(&match_id)
        .map_err(|e| e.to_string())?
    {
        return Ok(detail);
    }

    // Cache miss: fetch from Riot API, save participants, return
    let dto = api.get_match(&match_id).await?;
    let parts = dto_to_participants(&dto);

    if let Ok(db_guard) = db.lock() {
        let _ = db_guard.save_match_participants(&dto.metadata.match_id, dto.info.game_duration, &parts);
    }

    let detail = db.lock().map_err(|e| e.to_string())?
        .get_match_detail(&match_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Детали матча не найдены после загрузки".to_string())?;

    Ok(detail)
}

// ─── load_more_matches ────────────────────────────────────────────────────────

#[tauri::command]
pub async fn load_more_matches(
    db: State<'_, SharedDb>,
    puuid: String,
    offset: usize,
    limit: usize,
) -> Result<Vec<MatchSummary>, String> {
    let matches = db.lock().map_err(|e| e.to_string())?
        .get_cached_matches_paged(&puuid, offset, limit)
        .map_err(|e| e.to_string())?;
    Ok(matches)
}

// ─── get_live_game ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_live_game(
    api: State<'_, RiotApiClient>,
) -> Result<LiveGameData, String> {
    let none_result = LiveGameData {
        phase: "none".to_string(),
        queue_id: None,
        my_team: vec![],
        enemy_team: vec![],
        bans: vec![],
        game_time: None,
        timer: None,
    };

    let creds = match lcu::detect_lcu_credentials() {
        Some(c) => c,
        None => return Ok(none_result),
    };

    // 1. Попробовать Champ Select (LCU)
    if let Ok(session) = lcu::get_champ_select(&creds) {
        let mut my_team_players = Vec::new();
        let mut enemy_team_players = Vec::new();

        // Определяем кто сейчас пикает
        let picking_cells: Vec<i32> = session.actions.as_ref()
            .map(|actions| {
                actions.iter().flatten()
                    .filter(|a| a.is_in_progress.unwrap_or(false))
                    .filter_map(|a| a.actor_cell_id)
                    .collect()
            })
            .unwrap_or_default();

        if let Some(my_team) = &session.my_team {
            for p in my_team {
                my_team_players.push(LivePlayer {
                    puuid: p.puuid.clone().filter(|s| !s.is_empty()),
                    game_name: None,
                    tag_line: None,
                    champion_id: p.champion_id.unwrap_or(0),
                    assigned_position: p.assigned_position.clone(),
                    spell1_id: p.spell1_id.unwrap_or(0),
                    spell2_id: p.spell2_id.unwrap_or(0),
                    team_id: 100,
                    rank: None,
                    is_picking: p.cell_id.map(|c| picking_cells.contains(&c)).unwrap_or(false),
                });
            }
        }
        if let Some(their_team) = &session.their_team {
            for p in their_team {
                enemy_team_players.push(LivePlayer {
                    puuid: None,
                    game_name: None,
                    tag_line: None,
                    champion_id: p.champion_id.unwrap_or(0),
                    assigned_position: p.assigned_position.clone(),
                    spell1_id: p.spell1_id.unwrap_or(0),
                    spell2_id: p.spell2_id.unwrap_or(0),
                    team_id: 200,
                    rank: None,
                    is_picking: p.cell_id.map(|c| picking_cells.contains(&c)).unwrap_or(false),
                });
            }
        }

        // Баны
        let mut bans = Vec::new();
        if let Some(b) = &session.bans {
            if let Some(my_bans) = &b.my_team_bans {
                for &cid in my_bans {
                    if cid > 0 { bans.push(LiveBan { champion_id: cid, team_id: 100 }); }
                }
            }
            if let Some(their_bans) = &b.their_team_bans {
                for &cid in their_bans {
                    if cid > 0 { bans.push(LiveBan { champion_id: cid, team_id: 200 }); }
                }
            }
        }

        // Таймер
        let timer = session.timer.as_ref().map(|t| LiveTimer {
            phase: t.phase.clone().unwrap_or_default(),
            time_left_ms: t.adjusted_time_left_in_phase.unwrap_or(0),
        });

        // Обогащение рангами для myTeam (у них есть puuid)
        let puuids: Vec<Option<String>> = my_team_players.iter()
            .map(|p| p.puuid.clone())
            .collect();
        let rank_futs: Vec<_> = puuids.iter()
            .map(|puuid_opt| {
                let api = &api;
                async move {
                    if let Some(puuid) = puuid_opt {
                        api.get_league_entries_by_puuid(puuid).await.ok()
                    } else {
                        None
                    }
                }
            })
            .collect();
        let ranks = futures::future::join_all(rank_futs).await;
        for (i, entries_opt) in ranks.into_iter().enumerate() {
            if let Some(entries) = entries_opt {
                let ri = build_rank_info(entries);
                if let Some(solo) = ri.iter().find(|r| r.queue_type == "RANKED_SOLO_5x5") {
                    my_team_players[i].rank = Some(solo.clone());
                } else if let Some(first) = ri.first() {
                    my_team_players[i].rank = Some(first.clone());
                }
            }
        }

        return Ok(LiveGameData {
            phase: "champ_select".to_string(),
            queue_id: None,
            my_team: my_team_players,
            enemy_team: enemy_team_players,
            bans,
            game_time: None,
            timer,
        });
    }

    // 2. Попробовать Spectator API (in-game)
    let identity = match lcu::get_lcu_identity(&creds).await {
        Ok(id) => id,
        Err(_) => return Ok(none_result),
    };
    let account = match api.get_account_by_riot_id(&identity.game_name, &identity.tag_line).await {
        Ok(a) => a,
        Err(_) => return Ok(none_result),
    };

    let game = match api.get_active_game(&account.puuid).await {
        Ok(g) => g,
        Err(_) => return Ok(none_result),
    };

    // Определить мою команду
    let my_team_id = game.participants.iter()
        .find(|p| p.puuid.as_deref() == Some(&account.puuid))
        .map(|p| p.team_id)
        .unwrap_or(100);

    let mut my_team_players = Vec::new();
    let mut enemy_team_players = Vec::new();

    for p in &game.participants {
        let (gn, tl) = p.riot_id.as_ref()
            .and_then(|rid| rid.split_once('#'))
            .map(|(g, t)| (Some(g.to_string()), Some(t.to_string())))
            .unwrap_or((None, None));

        let player = LivePlayer {
            puuid: p.puuid.clone(),
            game_name: gn,
            tag_line: tl,
            champion_id: p.champion_id,
            assigned_position: None,
            spell1_id: p.spell1_id.unwrap_or(0),
            spell2_id: p.spell2_id.unwrap_or(0),
            team_id: p.team_id,
            rank: None,
            is_picking: false,
        };
        if p.team_id == my_team_id {
            my_team_players.push(player);
        } else {
            enemy_team_players.push(player);
        }
    }

    // Обогащение позициями из Live Client Data API
    if let Ok(live_players) = lcu::get_live_client_playerlist() {
        for lp in &live_players {
            if let (Some(gn), Some(pos)) = (&lp.riot_id_game_name, &lp.position) {
                if pos.is_empty() { continue; }
                let pos_normalized = match pos.as_str() {
                    "TOP" => "top",
                    "JUNGLE" => "jungle",
                    "MIDDLE" => "middle",
                    "BOTTOM" => "bottom",
                    "UTILITY" => "utility",
                    other => other,
                }.to_string();
                for p in my_team_players.iter_mut().chain(enemy_team_players.iter_mut()) {
                    if p.game_name.as_deref() == Some(gn) {
                        p.assigned_position = Some(pos_normalized.clone());
                    }
                }
            }
        }
    }

    // Баны
    let bans: Vec<LiveBan> = game.banned_champions.as_ref()
        .map(|bc| bc.iter()
            .filter(|b| b.champion_id > 0)
            .map(|b| LiveBan { champion_id: b.champion_id, team_id: b.team_id })
            .collect())
        .unwrap_or_default();

    // Обогащение рангами для всех игроков
    let all_puuids: Vec<Option<String>> = my_team_players.iter()
        .chain(enemy_team_players.iter())
        .map(|p| p.puuid.clone())
        .collect();
    let rank_futs: Vec<_> = all_puuids.iter()
        .map(|puuid_opt| {
            let api = &api;
            async move {
                if let Some(puuid) = puuid_opt {
                    api.get_league_entries_by_puuid(puuid).await.ok()
                } else {
                    None
                }
            }
        })
        .collect();
    let ranks = futures::future::join_all(rank_futs).await;

    let my_count = my_team_players.len();
    for (i, entries_opt) in ranks.into_iter().enumerate() {
        if let Some(entries) = entries_opt {
            let ri = build_rank_info(entries);
            let rank = ri.iter().find(|r| r.queue_type == "RANKED_SOLO_5x5")
                .or(ri.first())
                .cloned();
            if i < my_count {
                my_team_players[i].rank = rank;
            } else {
                enemy_team_players[i - my_count].rank = rank;
            }
        }
    }

    let game_time = game.game_length.or_else(|| {
        game.game_start_time.filter(|&t| t > 0).map(|start| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            (now - start / 1000).max(0)
        })
    });

    Ok(LiveGameData {
        phase: "in_game".to_string(),
        queue_id: game.game_queue_config_id,
        my_team: my_team_players,
        enemy_team: enemy_team_players,
        bans,
        game_time,
        timer: None,
    })
}

// ─── compat commands (deprecated stubs) ──────────────────────────────────────

#[tauri::command]
pub async fn get_match_history(
    _api: State<'_, RiotApiClient>,
    db: State<'_, SharedDb>,
    puuid: String,
    count: Option<u32>,
) -> Result<Vec<MatchSummary>, String> {
    let limit = count.unwrap_or(20) as usize;
    let matches = db.lock().map_err(|e| e.to_string())?
        .get_cached_matches_paged(&puuid, 0, limit)
        .map_err(|e| e.to_string())?;
    Ok(matches)
}

#[tauri::command]
pub async fn get_champion_stats(
    _api: State<'_, RiotApiClient>,
    db: State<'_, SharedDb>,
    puuid: String,
) -> Result<Vec<ChampionStat>, String> {
    let all = db.lock().map_err(|e| e.to_string())?
        .get_cached_matches(&puuid, 500)
        .unwrap_or_default();
    Ok(build_champion_stats(&all))
}

// ─── helpers ─────────────────────────────────────────────────────────────────

fn build_rank_info(entries: Vec<LeagueEntry>) -> Vec<RankInfo> {
    entries
        .into_iter()
        .filter(|e| e.tier.is_some())
        .map(|e| {
            let total = e.wins + e.losses;
            let winrate = if total > 0 {
                (e.wins as f64 / total as f64) * 100.0
            } else {
                0.0
            };
            RankInfo {
                queue_type: e.queue_type,
                tier: e.tier.unwrap_or_default(),
                rank: e.rank.unwrap_or_default(),
                lp: e.league_points.unwrap_or(0),
                wins: e.wins,
                losses: e.losses,
                winrate: (winrate * 10.0).round() / 10.0,
            }
        })
        .collect()
}
