use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::State;

use crate::ai_coach::{self, CoachState};
use crate::api_client::{ServerApiClient, EnrichLiveRequest, EnrichLivePlayer};
use crate::db::Db;
use crate::lcu;
use crate::models::*;

type SharedDb = Arc<Mutex<Db>>;

// ─── ChampionNamesCache (чтобы не загружать DDragon каждые 15 сек) ──────────

pub struct ChampionNamesCache {
    pub id_to_name: HashMap<i64, String>,
    pub name_to_id: HashMap<String, i64>,
    pub loaded: bool,
}

impl ChampionNamesCache {
    pub fn new() -> Self {
        Self {
            id_to_name: HashMap::new(),
            name_to_id: HashMap::new(),
            loaded: false,
        }
    }
}

// ─── LastLiveState (стабилизация фазы при переходах) ────────────────────────

pub struct LastLiveState {
    pub data: Option<LiveGameData>,
}

impl LastLiveState {
    pub fn new() -> Self {
        Self { data: None }
    }
}

// ─── search_player ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn search_player(
    api: State<'_, ServerApiClient>,
    db: State<'_, SharedDb>,
    game_name: String,
    tag_line: String,
) -> Result<PlayerProfile, String> {
    let profile = api.search_player(&game_name, &tag_line).await?;

    // Save to local cache for instant startup
    if let Ok(db) = db.lock() {
        let _ = db.save_account(&profile);
    }

    Ok(profile)
}

// ─── detect_account ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn detect_account(
    api: State<'_, ServerApiClient>,
    db: State<'_, SharedDb>,
) -> Result<DetectedAccount, String> {
    let creds = lcu::detect_lcu_credentials()
        .ok_or_else(|| "League Client не запущен".to_string())?;

    let identity = lcu::get_lcu_identity(&creds).await?;

    // Use server to look up the full profile
    let profile = api.search_player(&identity.game_name, &identity.tag_line).await?;

    // Cache locally
    if let Ok(db) = db.lock() {
        let _ = db.save_account(&profile);
    }

    Ok(DetectedAccount {
        puuid: profile.puuid,
        game_name: profile.game_name,
        tag_line: profile.tag_line,
        profile_icon_id: profile.profile_icon_id,
        summoner_level: profile.summoner_level,
        ranked: profile.ranked,
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
            Ok(Some(DetectedAccount {
                puuid: acc.puuid,
                game_name: acc.game_name,
                tag_line: acc.tag_line,
                profile_icon_id: acc.profile_icon_id,
                summoner_level: acc.summoner_level,
                ranked: vec![], // Ranks will be fetched from server when profile loads
            }))
        }
        None => Ok(None),
    }
}

// ─── get_mastery ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_mastery(
    api: State<'_, ServerApiClient>,
    puuid: String,
) -> Result<Vec<MasteryInfo>, String> {
    api.get_mastery(&puuid).await
}

// ─── get_matches_and_stats ────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_matches_and_stats(
    api: State<'_, ServerApiClient>,
    puuid: String,
    count: Option<u32>,
) -> Result<MatchesAndStats, String> {
    let _ = count;
    api.get_matches_and_stats(&puuid).await
}

// ─── get_match_detail ─────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_match_detail(
    api: State<'_, ServerApiClient>,
    match_id: String,
) -> Result<MatchDetail, String> {
    api.get_match_detail(&match_id).await
}

// ─── load_more_matches ────────────────────────────────────────────────────────

#[tauri::command]
pub async fn load_more_matches(
    api: State<'_, ServerApiClient>,
    puuid: String,
    offset: usize,
    limit: usize,
) -> Result<Vec<MatchSummary>, String> {
    api.load_more_matches(&puuid, offset, limit).await
}

// ─── get_live_game ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_live_game(
    api: State<'_, ServerApiClient>,
    champ_cache: State<'_, Arc<Mutex<ChampionNamesCache>>>,
    last_live: State<'_, Arc<Mutex<LastLiveState>>>,
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
        None => {
            if let Ok(mut ls) = last_live.lock() { ls.data = None; }
            return Ok(none_result);
        }
    };

    let gameflow_phase = lcu::get_gameflow_phase(&creds).unwrap_or_default();
    log::info!("[live] gameflow_phase = {:?}", gameflow_phase);

    let is_champ_select = gameflow_phase == "ChampSelect";
    let is_in_game = matches!(
        gameflow_phase.as_str(),
        "InProgress" | "GameStart" | "Reconnect" | "WaitingForStats"
    );

    // ── 1. Champ Select ─────────────────────────────────────────────────────
    if is_champ_select {
        if let Ok(session) = lcu::get_champ_select(&creds) {
            let mut players = Vec::new();
            let mut bans = Vec::new();

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
                    players.push(EnrichLivePlayer {
                        puuid: p.puuid.clone().filter(|s| !s.is_empty()),
                        game_name: None,
                        tag_line: None,
                        champion_id: p.champion_id.unwrap_or(0),
                        assigned_position: p.assigned_position.clone(),
                        spell1_id: p.spell1_id.unwrap_or(0),
                        spell2_id: p.spell2_id.unwrap_or(0),
                        team_id: 100,
                        is_picking: p.cell_id.map(|c| picking_cells.contains(&c)).unwrap_or(false),
                    });
                }
            }
            if let Some(their_team) = &session.their_team {
                for p in their_team {
                    players.push(EnrichLivePlayer {
                        puuid: None,
                        game_name: None,
                        tag_line: None,
                        champion_id: p.champion_id.unwrap_or(0),
                        assigned_position: p.assigned_position.clone(),
                        spell1_id: p.spell1_id.unwrap_or(0),
                        spell2_id: p.spell2_id.unwrap_or(0),
                        team_id: 200,
                        is_picking: p.cell_id.map(|c| picking_cells.contains(&c)).unwrap_or(false),
                    });
                }
            }

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

            let timer = session.timer.as_ref().map(|t| LiveTimer {
                phase: t.phase.clone().unwrap_or_default(),
                time_left_ms: t.adjusted_time_left_in_phase.unwrap_or(0),
            });

            // Get my puuid for the server to know which team is mine
            let my_puuid = players.iter()
                .find(|p| p.team_id == 100 && p.puuid.is_some())
                .and_then(|p| p.puuid.clone());

            let request = EnrichLiveRequest {
                phase: "champ_select".to_string(),
                players,
                bans,
                game_time: None,
                timer,
                my_puuid,
                queue_id: None,
            };

            match api.enrich_live_game(&request).await {
                Ok(result) => {
                    if let Ok(mut ls) = last_live.lock() { ls.data = Some(result.clone()); }
                    return Ok(result);
                }
                Err(e) => {
                    log::warn!("[live] Server enrichment failed: {}, returning local data", e);
                    // Fallback: return un-enriched data (no ranks)
                    let result = LiveGameData {
                        phase: "champ_select".to_string(),
                        queue_id: None,
                        my_team: request.players.iter().filter(|p| p.team_id == 100).map(|p| LivePlayer {
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
                        }).collect(),
                        enemy_team: request.players.iter().filter(|p| p.team_id == 200).map(|p| LivePlayer {
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
                        }).collect(),
                        bans: request.bans,
                        game_time: None,
                        timer: request.timer,
                    };
                    if let Ok(mut ls) = last_live.lock() { ls.data = Some(result.clone()); }
                    return Ok(result);
                }
            }
        }
    }

    // ── 2. In-Game ──────────────────────────────────────────────────────────
    if !is_in_game {
        if let Ok(mut ls) = last_live.lock() { ls.data = None; }
        return Ok(none_result);
    }

    // Get identity for the server to use Spectator API
    let identity = lcu::get_lcu_identity(&creds).await.ok();

    // Try to get puuid via server (search_player)
    let my_puuid = if let Some(ref id) = identity {
        match api.search_player(&id.game_name, &id.tag_line).await {
            Ok(profile) => Some(profile.puuid),
            Err(_) => None,
        }
    } else {
        None
    };

    // Try Live Client Data API (allgamedata has champion_name, playerlist doesn't)
    let mut players = Vec::new();

    if let Ok(alldata) = lcu::get_live_client_allgamedata() {
        if let Some(all_players) = &alldata.all_players {
            let (_id_to_name, name_to_id) = get_or_fetch_champion_names(&champ_cache).await;
            let my_name = identity.as_ref().map(|id| id.game_name.clone()).unwrap_or_default();

            let my_team_str = all_players.iter()
                .find(|p| p.riot_id_game_name.as_deref() == Some(&my_name)
                    || p.summoner_name.as_deref() == Some(&my_name))
                .and_then(|p| p.team.clone())
                .unwrap_or_else(|| "ORDER".to_string());

            for lp in all_players {
                let gn = lp.riot_id_game_name.clone();
                let champ_name = lp.champion_name.clone().unwrap_or_default();
                let champ_id = name_to_id.get(&champ_name).copied().unwrap_or(0);
                let pos = lp.position.clone().map(|pos| match pos.as_str() {
                    "TOP" => "top", "JUNGLE" => "jungle", "MIDDLE" => "middle",
                    "BOTTOM" => "bottom", "UTILITY" => "utility", other => other,
                }.to_string());

                let is_my_team = lp.team.as_deref() == Some(&my_team_str);
                let is_me = gn.as_deref() == Some(&my_name)
                    || lp.summoner_name.as_deref() == Some(&my_name);

                players.push(EnrichLivePlayer {
                    puuid: if is_me { my_puuid.clone() } else { None },
                    game_name: gn.or_else(|| lp.summoner_name.clone()),
                    tag_line: None,
                    champion_id: champ_id,
                    assigned_position: pos,
                    spell1_id: 0,
                    spell2_id: 0,
                    team_id: if is_my_team { 100 } else { 200 },
                    is_picking: false,
                });
            }
        }
    }

    let request = EnrichLiveRequest {
        phase: "in_game".to_string(),
        players,
        bans: vec![],
        game_time: lcu::get_live_client_allgamedata().ok()
            .and_then(|d| d.game_data.as_ref().and_then(|g| g.game_time.map(|t| t as i64))),
        timer: None,
        my_puuid: my_puuid.clone(),
        queue_id: None,
    };

    match api.enrich_live_game(&request).await {
        Ok(result) => {
            if let Ok(mut ls) = last_live.lock() { ls.data = Some(result.clone()); }
            return Ok(result);
        }
        Err(e) => {
            log::warn!("[live] Server enrichment failed: {}", e);
        }
    }

    // ── Fallback: return last known state ──
    if let Ok(ls) = last_live.lock() {
        if let Some(ref last) = ls.data {
            return Ok(last.clone());
        }
    }

    Ok(none_result)
}

// ─── compat commands (deprecated stubs) ──────────────────────────────────────

#[tauri::command]
pub async fn get_match_history(
    api: State<'_, ServerApiClient>,
    puuid: String,
    count: Option<u32>,
) -> Result<Vec<MatchSummary>, String> {
    let limit = count.unwrap_or(20) as usize;
    api.load_more_matches(&puuid, 0, limit).await
}

#[tauri::command]
pub async fn get_champion_stats(
    api: State<'_, ServerApiClient>,
    puuid: String,
) -> Result<Vec<ChampionStat>, String> {
    let result = api.get_matches_and_stats(&puuid).await?;
    Ok(result.champion_stats)
}

// ─── request_coaching (AI Coach) ─────────────────────────────────────────────

#[tauri::command]
pub async fn request_coaching(
    app: tauri::AppHandle,
    api: State<'_, ServerApiClient>,
    coach_state: State<'_, Arc<Mutex<CoachState>>>,
    champ_cache: State<'_, Arc<Mutex<ChampionNamesCache>>>,
) -> Result<(), String> {
    // Prevent concurrent requests
    {
        let mut state = coach_state.lock().map_err(|e| e.to_string())?;
        if state.is_requesting {
            return Err("Запрос уже выполняется".to_string());
        }
        state.is_requesting = true;
    }

    let creds = lcu::detect_lcu_credentials()
        .ok_or_else(|| {
            let mut s = coach_state.lock().unwrap();
            s.is_requesting = false;
            "League Client не запущен".to_string()
        })?;

    let gameflow_phase = lcu::get_gameflow_phase(&creds).unwrap_or_default();
    let is_champ_select = gameflow_phase == "ChampSelect";

    let ctx = if is_champ_select {
        let session = lcu::get_champ_select(&creds).map_err(|e| {
            let mut s = coach_state.lock().unwrap();
            s.is_requesting = false;
            e
        })?;

        let (champ_names, _) = get_or_fetch_champion_names(&champ_cache).await;

        let mut my_team_players = Vec::new();
        let mut enemy_team_players = Vec::new();

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
                    is_picking: false,
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
                    is_picking: false,
                });
            }
        }

        // Enrich ranks via server (needed for coach context)
        let my_puuid = my_team_players.iter()
            .find(|p| p.puuid.is_some())
            .and_then(|p| p.puuid.clone());

        let enrich_players: Vec<EnrichLivePlayer> = my_team_players.iter().chain(enemy_team_players.iter())
            .map(|p| EnrichLivePlayer {
                puuid: p.puuid.clone(),
                game_name: p.game_name.clone(),
                tag_line: p.tag_line.clone(),
                champion_id: p.champion_id,
                assigned_position: p.assigned_position.clone(),
                spell1_id: p.spell1_id,
                spell2_id: p.spell2_id,
                team_id: p.team_id,
                is_picking: p.is_picking,
            })
            .collect();

        let enriched = api.enrich_live_game(&EnrichLiveRequest {
            phase: "champ_select".to_string(),
            players: enrich_players,
            bans: vec![],
            game_time: None,
            timer: None,
            my_puuid: my_puuid.clone(),
            queue_id: None,
        }).await;

        if let Ok(enriched_data) = enriched {
            my_team_players = enriched_data.my_team;
            enemy_team_players = enriched_data.enemy_team;
        }

        ai_coach::build_context_champ_select(
            &my_team_players,
            &enemy_team_players,
            &champ_names,
            my_puuid.as_deref(),
        )
    } else {
        // In-game: use allgamedata for rich stats
        let alldata = lcu::get_live_client_allgamedata().map_err(|e| {
            let mut s = coach_state.lock().unwrap();
            s.is_requesting = false;
            format!("Не удалось получить данные игры: {}", e)
        })?;

        let identity = lcu::get_lcu_identity(&creds).await.map_err(|e| {
            let mut s = coach_state.lock().unwrap();
            s.is_requesting = false;
            e
        })?;

        ai_coach::build_context_from_allgamedata(&alldata, &identity.game_name)
            .ok_or_else(|| {
                let mut s = coach_state.lock().unwrap();
                s.is_requesting = false;
                "Не удалось определить игрока в матче".to_string()
            })?
    };

    // Check if state changed since last request
    let state_hash = ctx.compute_hash();
    {
        let mut state = coach_state.lock().map_err(|e| e.to_string())?;
        if state.last_state_hash == state_hash && state_hash != 0 {
            state.is_requesting = false;
            return Err("Состояние игры не изменилось".to_string());
        }
        state.last_state_hash = state_hash;
    }

    // Spawn streaming task — sends context to server, server calls Anthropic
    let api_client = api.inner().clone();
    let coach_state_arc = Arc::clone(&*coach_state);
    let app_handle = app.clone();

    tokio::spawn(async move {
        let _ = api_client.stream_coaching(&app_handle, &ctx).await;
        if let Ok(mut s) = coach_state_arc.lock() {
            s.is_requesting = false;
        }
    });

    Ok(())
}

/// Fetch champion names, using Tauri-managed cache when available
async fn get_or_fetch_champion_names(
    cache: &Arc<Mutex<ChampionNamesCache>>,
) -> (HashMap<i64, String>, HashMap<String, i64>) {
    {
        let c = cache.lock().unwrap();
        if c.loaded {
            return (c.id_to_name.clone(), c.name_to_id.clone());
        }
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    let mut id_map: HashMap<i64, String> = HashMap::new();
    let mut name_map: HashMap<String, i64> = HashMap::new();

    let url_en = "https://ddragon.leagueoflegends.com/cdn/16.7.1/data/en_US/champion.json";
    if let Ok(resp) = client.get(url_en).send().await {
        if let Ok(json) = resp.json::<serde_json::Value>().await {
            if let Some(data) = json.get("data").and_then(|d| d.as_object()) {
                for (internal_id, info) in data {
                    if let Some(key) = info.get("key").and_then(|k| k.as_str()) {
                        if let Ok(id) = key.parse::<i64>() {
                            id_map.insert(id, internal_id.clone());
                            name_map.insert(internal_id.clone(), id);
                            if let Some(display_name) = info.get("name").and_then(|n| n.as_str()) {
                                name_map.insert(display_name.to_string(), id);
                            }
                        }
                    }
                }
            }
        }
    }

    let url_ru = "https://ddragon.leagueoflegends.com/cdn/16.7.1/data/ru_RU/champion.json";
    if let Ok(resp) = client.get(url_ru).send().await {
        if let Ok(json) = resp.json::<serde_json::Value>().await {
            if let Some(data) = json.get("data").and_then(|d| d.as_object()) {
                for (_internal_id, info) in data {
                    if let Some(key) = info.get("key").and_then(|k| k.as_str()) {
                        if let Ok(id) = key.parse::<i64>() {
                            if let Some(ru_name) = info.get("name").and_then(|n| n.as_str()) {
                                name_map.insert(ru_name.to_string(), id);
                            }
                        }
                    }
                }
            }
        }
    }

    log::info!("[champ_cache] Loaded {} id_to_name, {} name_to_id entries", id_map.len(), name_map.len());

    if id_map.is_empty() {
        return (HashMap::new(), HashMap::new());
    }

    {
        let mut c = cache.lock().unwrap();
        c.id_to_name = id_map.clone();
        c.name_to_id = name_map.clone();
        c.loaded = true;
    }

    (id_map, name_map)
}
