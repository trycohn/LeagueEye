use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::State;

use crate::ai_coach::{self, CoachState, ChampionMetaInfo, ChampionAbility};
use crate::api_client::{ServerApiClient, EnrichLiveRequest, EnrichLivePlayer};
use crate::db::Db;
use crate::lcu;
use crate::models::*;

type SharedDb = Arc<Mutex<Db>>;

// ─── ChampionNamesCache (чтобы не загружать DDragon каждые 15 сек) ──────────

// ─── ChampionNamesCache (чтобы не загружать DDragon каждые 15 сек) ──────────

pub struct ChampionNamesCache {
    pub id_to_name: HashMap<i64, String>,
    pub name_to_id: HashMap<String, i64>,
    pub champion_meta: HashMap<String, ChampionMetaInfo>, // key = internal name
    pub loaded: bool,
}

impl ChampionNamesCache {
    pub fn new() -> Self {
        Self {
            id_to_name: HashMap::new(),
            name_to_id: HashMap::new(),
            champion_meta: HashMap::new(),
            loaded: false,
        }
    }
}

// ─── ItemCostCache (золото предметов из DDragon) ───────────────────────────

pub struct ItemCostCache {
    pub item_costs: HashMap<i32, i32>,
    pub loaded: bool,
}

impl ItemCostCache {
    pub fn new() -> Self {
        Self { item_costs: HashMap::new(), loaded: false }
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
    game_name: String,
    tag_line: String,
) -> Result<PlayerProfile, String> {
    let profile = api.search_player(&game_name, &tag_line).await?;
    Ok(profile)
}

// ─── detect_account ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn detect_account(
    api: State<'_, ServerApiClient>,
    db: State<'_, SharedDb>,
) -> Result<DetectedAccount, String> {
    // User-initiated action — bypass credential cache to get fresh data
    let creds = lcu::detect_lcu_credentials_fresh()
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
    let start = std::time::Instant::now();
    let running = lcu::is_lcu_running();
    let elapsed = start.elapsed();
    if elapsed.as_millis() > 50 {
        log::warn!("[perf] poll_client_status took {:?}", elapsed);
    }
    if !running {
        crate::keyboard_hook::set_game_active(false);
    }
    running
}

#[tauri::command]
pub async fn get_overlay_eligibility() -> bool {
    let start = std::time::Instant::now();
    let eligible = crate::overlay_policy::current_overlay_eligibility();
    let elapsed = start.elapsed();
    if elapsed.as_millis() > 50 {
        log::warn!("[perf] get_overlay_eligibility took {:?} (async, off main thread)", elapsed);
    }
    crate::keyboard_hook::set_game_active(eligible);
    eligible
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
                ranked: vec![], // Rank is refreshed from the server when opening the profile
            }))
        }
        None => Ok(None),
    }
}

// ─── get_matchups ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_matchups(
    api: State<'_, ServerApiClient>,
    puuid: String,
) -> Result<Vec<MatchupStat>, String> {
    api.get_matchups(&puuid).await
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

// ─── get_global_dashboard ───────────────────────────────────────────────────

#[tauri::command]
pub async fn get_global_dashboard(
    api: State<'_, ServerApiClient>,
) -> Result<GlobalDashboardData, String> {
    api.get_global_dashboard().await
}

// ─── get_live_game ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_live_game(
    api: State<'_, ServerApiClient>,
    champ_cache: State<'_, Arc<Mutex<ChampionNamesCache>>>,
    last_live: State<'_, Arc<Mutex<LastLiveState>>>,
) -> Result<LiveGameData, String> {
    let cmd_start = std::time::Instant::now();

    let none_result = LiveGameData {
        phase: "none".to_string(),
        queue_id: None,
        my_team: vec![],
        enemy_team: vec![],
        bans: vec![],
        game_time: None,
        timer: None,
    };

    let t0 = std::time::Instant::now();
    let creds = match lcu::detect_lcu_credentials() {
        Some(c) => c,
        None => {
            let elapsed = t0.elapsed();
            if elapsed.as_millis() > 50 {
                log::warn!("[perf] get_live_game: detect_lcu_credentials (no creds) took {:?}", elapsed);
            }
            crate::keyboard_hook::set_game_active(false);
            if let Ok(mut ls) = last_live.lock() { ls.data = None; }
            return Ok(none_result);
        }
    };
    let creds_elapsed = t0.elapsed();

    let t1 = std::time::Instant::now();
    let gameflow_phase = lcu::get_gameflow_phase_async(&creds).await.unwrap_or_default();
    let phase_elapsed = t1.elapsed();

    if creds_elapsed.as_millis() > 50 || phase_elapsed.as_millis() > 50 {
        log::warn!("[perf] get_live_game: creds={:?}, gameflow_phase={:?}, phase={}", creds_elapsed, phase_elapsed, gameflow_phase);
    }

    log::debug!("[live] gameflow_phase = {:?}", gameflow_phase);

    let is_champ_select = gameflow_phase == "ChampSelect";
    let is_in_game = matches!(
        gameflow_phase.as_str(),
        "InProgress" | "GameStart" | "Reconnect"
    );

    // ── 1. Champ Select ─────────────────────────────────────────────────────
    if is_champ_select {
        if let Ok(session) = lcu::get_champ_select(&creds).await {
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
                        puuid: p.puuid.clone().filter(|s| !s.is_empty()),
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

            // Always non-blocking: return fresh LCU data merged with any cached
            // enrichment immediately, then fire background enrichment for next poll.
            // This ensures Ban/Pick UI never freezes waiting for server/Riot API.
            let cached_data = last_live.lock().ok()
                .and_then(|ls| ls.data.clone())
                .filter(|d| d.phase == "champ_select");

            let build_player = |p: &EnrichLivePlayer, cached: &Option<LiveGameData>| -> LivePlayer {
                // Try to find matching rank from cached data by puuid or position
                let rank = cached.as_ref().and_then(|cd| {
                    let team = if p.team_id == 100 { &cd.my_team } else { &cd.enemy_team };
                    // Match by puuid first
                    if let Some(ref puuid) = p.puuid {
                        if let Some(found) = team.iter().find(|cp| cp.puuid.as_deref() == Some(puuid)) {
                            return found.rank.clone();
                        }
                    }
                    // Fallback: match by position in team
                    let idx = if p.team_id == 100 {
                        request.players.iter().filter(|rp| rp.team_id == 100).position(|rp| std::ptr::eq(rp, p))
                    } else {
                        request.players.iter().filter(|rp| rp.team_id == 200).position(|rp| std::ptr::eq(rp, p))
                    };
                    idx.and_then(|i| team.get(i)).and_then(|cp| cp.rank.clone())
                });

                LivePlayer {
                    puuid: p.puuid.clone(),
                    game_name: p.game_name.clone(),
                    tag_line: p.tag_line.clone(),
                    champion_id: p.champion_id,
                    assigned_position: p.assigned_position.clone(),
                    spell1_id: p.spell1_id,
                    spell2_id: p.spell2_id,
                    team_id: p.team_id,
                    rank,
                    is_picking: p.is_picking,
                }
            };

            let result = LiveGameData {
                phase: "champ_select".to_string(),
                queue_id: None,
                my_team: request.players.iter().filter(|p| p.team_id == 100)
                    .map(|p| build_player(p, &cached_data)).collect(),
                enemy_team: request.players.iter().filter(|p| p.team_id == 200)
                    .map(|p| build_player(p, &cached_data)).collect(),
                bans: request.bans.clone(),
                game_time: None,
                timer: request.timer.clone(),
            };

            // Fire background enrichment to update ranks for next poll
            let api_bg = api.inner().clone();
            let last_live_bg = last_live.inner().clone();
            tokio::spawn(async move {
                match api_bg.enrich_live_game(&request).await {
                    Ok(enriched) => {
                        if let Ok(mut ls) = last_live_bg.lock() {
                            ls.data = Some(enriched);
                        }
                    }
                    Err(e) => {
                        log::debug!("[live] Background enrichment failed: {}", e);
                    }
                }
            });

            let cmd_elapsed = cmd_start.elapsed();
            if cmd_elapsed.as_millis() > 200 {
                log::warn!("[perf] get_live_game (champ_select) total: {:?}", cmd_elapsed);
            }
            return Ok(result);
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

    // Reuse one allgamedata snapshot per tick to avoid duplicate live client calls.
    let mut players = Vec::new();
    let alldata = lcu::get_live_client_allgamedata().await.ok();

    if let Some(alldata) = alldata.as_ref() {
        if let Some(all_players) = &alldata.all_players {
            let (_id_to_name, name_to_id, _meta) = get_or_fetch_champion_names(&champ_cache).await;
            let my_name = identity.as_ref().map(|id| id.game_name.clone()).unwrap_or_default();

            let my_team_str = all_players.iter()
                .find(|p| p.riot_id_game_name.as_deref() == Some(&my_name)
                    || p.summoner_name.as_deref() == Some(&my_name))
                .and_then(|p| p.team.clone())
                .unwrap_or_else(|| "ORDER".to_string());

            for lp in all_players {
                let riot_game_name = lp.riot_id_game_name.clone().filter(|s| !s.is_empty());
                let summoner_name = lp.summoner_name.clone().filter(|s| !s.is_empty());
                let tag_line = lp.riot_id_tag_line.clone().filter(|s| !s.is_empty());
                let champ_name = lp.champion_name.clone().unwrap_or_default();
                let champ_id = name_to_id.get(&champ_name).copied().unwrap_or(0);
                let pos = lp.position.clone().map(|pos| match pos.as_str() {
                    "TOP" => "top", "JUNGLE" => "jungle", "MIDDLE" => "middle",
                    "BOTTOM" => "bottom", "UTILITY" => "utility", other => other,
                }.to_string());

                let is_my_team = lp.team.as_deref() == Some(&my_team_str);
                let is_me = riot_game_name.as_deref() == Some(&my_name)
                    || summoner_name.as_deref() == Some(&my_name);

                players.push(EnrichLivePlayer {
                    puuid: if is_me { my_puuid.clone() } else { None },
                    game_name: riot_game_name.or(summoner_name),
                    tag_line,
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
        game_time: alldata.as_ref()
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

    let gameflow_phase = lcu::get_gameflow_phase_async(&creds).await.unwrap_or_default();
    let is_champ_select = gameflow_phase == "ChampSelect";

    let ctx = if is_champ_select {
        let session = lcu::get_champ_select(&creds).await.map_err(|e| {
            let mut s = coach_state.lock().unwrap();
            s.is_requesting = false;
            e
        })?;

        let (champ_names, _, _) = get_or_fetch_champion_names(&champ_cache).await;

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

        // Collect all champion internal names for meta fetch
        let all_champ_names: Vec<String> = my_team_players.iter()
            .chain(enemy_team_players.iter())
            .filter_map(|p| champ_names.get(&p.champion_id).cloned())
            .collect();
        let champ_meta = ensure_champion_meta(&champ_cache, &all_champ_names).await;

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
            &champ_meta,
            my_puuid.as_deref(),
        )
    } else {
        // In-game: use allgamedata for rich stats
        let alldata = lcu::get_live_client_allgamedata().await.map_err(|e| {
            let mut s = coach_state.lock().unwrap();
            s.is_requesting = false;
            format!("Не удалось получить данные игры: {}", e)
        })?;

        let identity = lcu::get_lcu_identity(&creds).await.map_err(|e| {
            let mut s = coach_state.lock().unwrap();
            s.is_requesting = false;
            e
        })?;

        // Collect all champion internal names from the game
        let all_champ_names: Vec<String> = alldata.all_players.as_ref()
            .map(|players| {
                players.iter()
                    .filter_map(|p| p.champion_name.clone())
                    .collect()
            })
            .unwrap_or_default();

        let (_, _, _champ_meta) = get_or_fetch_champion_names(&champ_cache).await;
        let champ_meta = ensure_champion_meta(&champ_cache, &all_champ_names).await;

        ai_coach::build_context_from_allgamedata(&alldata, &identity.game_name, &champ_meta)
            .ok_or_else(|| {
                let mut s = coach_state.lock().unwrap();
                s.is_requesting = false;
                "Не удалось определить игрока в матче".to_string()
            })?
    };

    // Spawn streaming task — sends context to server, server calls configured AI provider
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

// ─── request_draft_advice (Draft Helper) ─────────────────────────────────────

#[tauri::command]
pub async fn request_draft_advice(
    app: tauri::AppHandle,
    api: State<'_, ServerApiClient>,
    coach_state: State<'_, Arc<Mutex<CoachState>>>,
    champ_cache: State<'_, Arc<Mutex<ChampionNamesCache>>>,
) -> Result<(), String> {
    // Reuse CoachState to prevent concurrent draft requests
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

    let gameflow_phase = lcu::get_gameflow_phase_async(&creds).await.unwrap_or_default();
    if gameflow_phase != "ChampSelect" {
        let mut s = coach_state.lock().unwrap();
        s.is_requesting = false;
        return Err("Draft Helper доступен только во время выбора чемпионов".to_string());
    }

    let session = lcu::get_champ_select(&creds).await.map_err(|e| {
        let mut s = coach_state.lock().unwrap();
        s.is_requesting = false;
        e
    })?;

    let (champ_names, _, _) = get_or_fetch_champion_names(&champ_cache).await;

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

    // Collect champion names for meta fetch
    let all_champ_names: Vec<String> = my_team_players.iter()
        .chain(enemy_team_players.iter())
        .filter_map(|p| champ_names.get(&p.champion_id).cloned())
        .collect();
    let champ_meta = ensure_champion_meta(&champ_cache, &all_champ_names).await;

    // Get my puuid
    let my_puuid = my_team_players.iter()
        .find(|p| p.puuid.is_some())
        .and_then(|p| p.puuid.clone());

    // Enrich ranks via server
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

    // Fetch champion pool from server (player's match stats)
    let champion_pool: Vec<ChampionPoolEntry> = if let Some(ref puuid) = my_puuid {
        match api.get_matches_and_stats(puuid).await {
            Ok(stats) => stats.champion_stats.iter()
                .filter(|s| s.games >= 2)
                .take(15)
                .map(|s| ChampionPoolEntry {
                    champion_name: s.champion_name.clone(),
                    games: s.games,
                    winrate: s.winrate,
                })
                .collect(),
            Err(_) => vec![],
        }
    } else {
        vec![]
    };

    let ctx = ai_coach::build_context_draft_pick(
        &session,
        &my_team_players,
        &enemy_team_players,
        &champ_names,
        &champ_meta,
        my_puuid.as_deref(),
        champion_pool,
    );

    // Spawn streaming task with draft-specific event kinds
    let api_client = api.inner().clone();
    let coach_state_arc = Arc::clone(&*coach_state);
    let app_handle = app.clone();

    tokio::spawn(async move {
        let _ = api_client.stream_draft_coaching(&app_handle, &ctx).await;
        if let Ok(mut s) = coach_state_arc.lock() {
            s.is_requesting = false;
        }
    });

    Ok(())
}

/// Fetch champion names + basic metadata (resource, tags), using Tauri-managed cache when available
async fn get_or_fetch_champion_names(
    cache: &Arc<Mutex<ChampionNamesCache>>,
) -> (HashMap<i64, String>, HashMap<String, i64>, HashMap<String, ChampionMetaInfo>) {
    {
        let c = cache.lock().unwrap();
        if c.loaded {
            return (c.id_to_name.clone(), c.name_to_id.clone(), c.champion_meta.clone());
        }
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    let mut id_map: HashMap<i64, String> = HashMap::new();
    let mut name_map: HashMap<String, i64> = HashMap::new();
    let mut meta_map: HashMap<String, ChampionMetaInfo> = HashMap::new();

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
                            // Extract resource and tags (available in list endpoint)
                            let resource = info.get("partype")
                                .and_then(|r| r.as_str())
                                .unwrap_or("None")
                                .to_string();
                            let tags = info.get("tags")
                                .and_then(|t| t.as_array())
                                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                                .unwrap_or_default();
                            meta_map.insert(internal_id.clone(), ChampionMetaInfo {
                                resource,
                                tags,
                                abilities: vec![],
                                passive_name: String::new(),
                                passive_desc: String::new(),
                                ally_tips: vec![],
                            });
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

    log::info!("[champ_cache] Loaded {} id_to_name, {} name_to_id, {} meta entries", id_map.len(), name_map.len(), meta_map.len());

    if id_map.is_empty() {
        return (HashMap::new(), HashMap::new(), HashMap::new());
    }

    {
        let mut c = cache.lock().unwrap();
        c.id_to_name = id_map.clone();
        c.name_to_id = name_map.clone();
        c.champion_meta = meta_map.clone();
        c.loaded = true;
    }

    (id_map, name_map, meta_map)
}

/// Lazy-fetch detailed champion data (abilities, passive, tips) for specific champions.
/// Only fetches if not already cached.
async fn ensure_champion_meta(
    cache: &Arc<Mutex<ChampionNamesCache>>,
    champion_names: &[String], // internal names
) -> HashMap<String, ChampionMetaInfo> {
    // Check what's already cached
    let existing = {
        let c = cache.lock().unwrap();
        c.champion_meta.clone()
    };

    let mut result = existing.clone();
    let mut to_fetch: Vec<String> = Vec::new();

    for name in champion_names {
        if let Some(meta) = existing.get(name) {
            if !meta.abilities.is_empty() {
                result.insert(name.clone(), meta.clone());
                continue;
            }
        }
        to_fetch.push(name.clone());
    }

    if to_fetch.is_empty() {
        return result;
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    for name in &to_fetch {
        let url = format!("https://ddragon.leagueoflegends.com/cdn/16.7.1/data/en_US/champion/{name}.json");
        let Ok(resp) = client.get(&url).send().await else { continue };
        let Ok(json) = resp.json::<serde_json::Value>().await else { continue };

        let champ_data = json.get("data")
            .and_then(|d| d.get(name))
            .unwrap_or(&json);

        let resource = champ_data.get("partype")
            .and_then(|r| r.as_str())
            .unwrap_or("None")
            .to_string();

        let tags = champ_data.get("tags")
            .and_then(|t| t.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        // Parse abilities
        let mut abilities = Vec::new();
        if let Some(spells) = champ_data.get("spells").and_then(|s| s.as_array()) {
            let slots = ["Q", "W", "E", "R"];
            for (i, spell) in spells.iter().take(4).enumerate() {
                let slot = slots.get(i).unwrap_or(&"?").to_string();
                let spell_name = spell.get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string();
                let desc = spell.get("description")
                    .and_then(|d| d.as_str())
                    .map(|s| strip_ddragon_html(s))
                    .unwrap_or_default();
                abilities.push(ChampionAbility {
                    slot,
                    name: spell_name,
                    short_desc: truncate_desc(&desc, 100),
                });
            }
        }

        let passive = champ_data.get("passive");
        let passive_name = passive
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();
        let passive_desc = passive
            .and_then(|p| p.get("description"))
            .and_then(|d| d.as_str())
            .map(|s| strip_ddragon_html(s))
            .unwrap_or_default();

        let ally_tips = champ_data.get("allytips")
            .and_then(|t| t.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str().map(|s| strip_ddragon_html(s)))
                .take(3)
                .collect())
            .unwrap_or_default();

        let meta = ChampionMetaInfo {
            resource,
            tags,
            abilities,
            passive_name,
            passive_desc,
            ally_tips,
        };

        result.insert(name.clone(), meta.clone());

        // Update cache
        {
            let mut c = cache.lock().unwrap();
            c.champion_meta.insert(name.clone(), meta);
        }
    }

    result
}

/// Strip DDragon HTML tags from descriptions
fn strip_ddragon_html(s: &str) -> String {
    let mut result = s.to_string();
    // Remove <br> tags
    result = result.replace("<br>", " ");
    // Remove <mainText>, <mainText2>, etc.
    result = strip_html_tags(&result);
    // Clean up multiple spaces
    while result.contains("  ") {
        result = result.replace("  ", " ");
    }
    result.trim().to_string()
}

fn strip_html_tags(s: &str) -> String {
    // Simple regex-like replacement for HTML tags — since we can't use regex crate
    // Just remove anything between < and >
    let mut result = String::new();
    let mut inside_tag = false;
    for ch in s.chars() {
        if ch == '<' {
            inside_tag = true;
        } else if ch == '>' {
            inside_tag = false;
        } else if !inside_tag {
            result.push(ch);
        }
    }
    result
}

fn truncate_desc(s: &str, max_len: usize) -> String {
    // Take first sentence or max_len chars
    let end = s.find(". ").unwrap_or(s.len()).min(max_len);
    if end < s.len() && s[end..].starts_with(". ") {
        format!("{}.", &s[..end])
    } else {
        s[..end.min(s.len())].to_string()
    }
}

// ─── item cost cache ────────────────────────────────────────────────────────

async fn get_or_fetch_item_costs(
    cache: &Arc<Mutex<ItemCostCache>>,
) -> HashMap<i32, i32> {
    {
        let c = cache.lock().unwrap();
        if c.loaded { return c.item_costs.clone(); }
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    let url = "https://ddragon.leagueoflegends.com/cdn/16.7.1/data/en_US/item.json";
    let mut costs: HashMap<i32, i32> = HashMap::new();

    if let Ok(resp) = client.get(url).send().await {
        if let Ok(json) = resp.json::<serde_json::Value>().await {
            if let Some(data) = json.get("data").and_then(|d| d.as_object()) {
                for (id_str, info) in data {
                    if let Ok(id) = id_str.parse::<i32>() {
                        if let Some(total) = info.pointer("/gold/total").and_then(|v| v.as_i64()) {
                            costs.insert(id, total as i32);
                        }
                    }
                }
            }
        }
    }

    log::info!("[item_cache] Loaded {} item costs", costs.len());

    if !costs.is_empty() {
        let mut c = cache.lock().unwrap();
        c.item_costs = costs.clone();
        c.loaded = true;
    }

    costs
}

// ─── get_gold_comparison ────────────────────────────────────────────────────

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoldComparisonData {
    pub lanes: Vec<LaneGoldComparison>,
    pub game_time: Option<i64>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaneGoldComparison {
    pub role: String,
    pub ally_champion_name: String,   // internal name for DDragon icon
    pub ally_gold: i32,
    pub enemy_champion_name: String,
    pub enemy_gold: i32,
    pub gold_diff: i32,               // positive = ally ahead
}

#[tauri::command]
pub async fn get_gold_comparison(
    item_cache: State<'_, Arc<Mutex<ItemCostCache>>>,
    champ_cache: State<'_, Arc<Mutex<ChampionNamesCache>>>,
) -> Result<GoldComparisonData, String> {
    let alldata = lcu::get_live_client_allgamedata().await
        .map_err(|_| "Live Client API недоступен".to_string())?;

    let players = alldata.all_players.as_ref()
        .ok_or("Нет данных игроков")?;

    let game_time = alldata.game_data.as_ref()
        .and_then(|g| g.game_time.map(|t| t as i64));

    // Determine my team
    let creds = lcu::detect_lcu_credentials()
        .ok_or("League Client не запущен")?;
    let identity = lcu::get_lcu_identity(&creds).await
        .map_err(|e| format!("Не удалось определить игрока: {}", e))?;

    let my_team_str = players.iter()
        .find(|p| {
            p.riot_id_game_name.as_deref() == Some(&identity.game_name)
                || p.summoner_name.as_deref() == Some(&identity.game_name)
        })
        .and_then(|p| p.team.clone())
        .unwrap_or_else(|| "ORDER".to_string());

    let item_costs = get_or_fetch_item_costs(&item_cache).await;
    let (id_to_name, name_to_id, _meta) = get_or_fetch_champion_names(&champ_cache).await;

    let calc_gold = |p: &lcu::LiveFullPlayer| -> i32 {
        p.items.as_ref().map(|items| {
            items.iter().map(|item| {
                let id = item.item_id.unwrap_or(0);
                let count = item.count.unwrap_or(1).max(1);
                item_costs.get(&id).copied().unwrap_or(0) * count
            }).sum()
        }).unwrap_or(0)
    };

    // Resolve Russian champion name → internal name for icon URLs
    let resolve_champ_name = |ru_name: &str| -> String {
        if let Some(&champ_id) = name_to_id.get(ru_name) {
            id_to_name.get(&champ_id).cloned().unwrap_or_else(|| ru_name.to_string())
        } else {
            ru_name.to_string()
        }
    };

    // Group by role
    let role_order = ["TOP", "JUNGLE", "MIDDLE", "BOTTOM", "UTILITY"];

    let mut allies: HashMap<String, (&lcu::LiveFullPlayer, i32)> = HashMap::new();
    let mut enemies: HashMap<String, (&lcu::LiveFullPlayer, i32)> = HashMap::new();

    for p in players {
        let role = p.position.clone().unwrap_or_default().to_uppercase();
        if role.is_empty() { continue; }
        let gold = calc_gold(p);
        if p.team.as_deref() == Some(&my_team_str) {
            allies.insert(role, (p, gold));
        } else {
            enemies.insert(role, (p, gold));
        }
    }

    let mut lanes = Vec::new();
    for role in &role_order {
        let role_str = role.to_string();
        if let (Some((ally, ally_gold)), Some((enemy, enemy_gold))) =
            (allies.get(&role_str), enemies.get(&role_str))
        {
            let ally_champ = ally.champion_name.clone().unwrap_or_default();
            let enemy_champ = enemy.champion_name.clone().unwrap_or_default();
            lanes.push(LaneGoldComparison {
                role: role_str,
                ally_champion_name: resolve_champ_name(&ally_champ),
                ally_gold: *ally_gold,
                enemy_champion_name: resolve_champ_name(&enemy_champ),
                enemy_gold: *enemy_gold,
                gold_diff: ally_gold - enemy_gold,
            });
        }
    }

    Ok(GoldComparisonData { lanes, game_time })
}

// ─── get_app_version ─────────────────────────────────────────────────────────

// ─── Favorites ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_favorites(
    db: State<'_, SharedDb>,
) -> Result<Vec<FavoritePlayer>, String> {
    let db = db.lock().map_err(|e| e.to_string())?;
    db.get_favorites().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_favorite(
    db: State<'_, SharedDb>,
    puuid: String,
    game_name: String,
    tag_line: String,
    profile_icon_id: i64,
    source: Option<String>,
) -> Result<(), String> {
    let src = source.unwrap_or_else(|| "manual".to_string());
    let db = db.lock().map_err(|e| e.to_string())?;
    db.add_favorite(&puuid, &game_name, &tag_line, profile_icon_id, &src)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_favorite(
    db: State<'_, SharedDb>,
    puuid: String,
) -> Result<(), String> {
    let db = db.lock().map_err(|e| e.to_string())?;
    db.remove_favorite(&puuid).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn is_favorite(
    db: State<'_, SharedDb>,
    puuid: String,
) -> Result<bool, String> {
    let db = db.lock().map_err(|e| e.to_string())?;
    db.is_favorite(&puuid).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_frequent_teammates(
    api: State<'_, ServerApiClient>,
    puuid: String,
) -> Result<Vec<FrequentTeammate>, String> {
    api.get_frequent_teammates(&puuid).await
}

// ─── App version & updates ──────────────────────────────────────────────────

#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub body: Option<String>,
    pub date: Option<String>,
}

#[tauri::command]
pub async fn check_for_update(
    app: tauri::AppHandle,
) -> Result<Option<UpdateInfo>, String> {
    use tauri_plugin_updater::UpdaterExt;

    let updater = app.updater().map_err(|e| format!("Updater init error: {}", e))?;

    match updater.check().await {
        Ok(Some(update)) => {
            Ok(Some(UpdateInfo {
                version: update.version.clone(),
                body: update.body.clone(),
                date: update.date.map(|d| d.to_string()),
            }))
        }
        Ok(None) => Ok(None),
        Err(e) => {
            log::warn!("[updater] Check failed: {}", e);
            Err(format!("Ошибка проверки обновлений: {}", e))
        }
    }
}

#[tauri::command]
pub async fn install_update(
    app: tauri::AppHandle,
) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;

    let updater = app.updater().map_err(|e| format!("Updater init error: {}", e))?;

    let update = updater.check().await
        .map_err(|e| format!("Ошибка проверки: {}", e))?
        .ok_or_else(|| "Нет доступных обновлений".to_string())?;

    log::info!("[updater] Downloading v{}", update.version);

    // Step 1: Download the update first (app stays alive during download)
    let mut downloaded: u64 = 0;
    let bytes = update.download(
        |chunk_len, content_len| {
            downloaded += chunk_len as u64;
            log::debug!(
                "[updater] Downloaded {} / {}",
                downloaded,
                content_len.unwrap_or(0)
            );
        },
        || {
            log::info!("[updater] Download complete");
        },
    )
    .await
    .map_err(|e| format!("Ошибка скачивания: {}", e))?;

    log::info!("[updater] Installing (app will exit on Windows)...");

    // Step 2: Install — on Windows this exits the process and runs NSIS installer
    // The NSIS hook (nsis-hooks.nsh) will also taskkill the process as a safety net
    update.install(bytes)
        .map_err(|e| format!("Ошибка установки: {}", e))?;

    // On Windows we typically never reach here because install() exits the process.
    // On other platforms, restart manually.
    log::info!("[updater] Update installed, relaunching...");
    app.restart();
}
