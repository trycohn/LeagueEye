use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

// ── LCU credential & install-dir caches ─────────────────────────────────────

const CREDENTIALS_CACHE_TTL: Duration = Duration::from_secs(3);
const INSTALL_DIR_CACHE_TTL: Duration = Duration::from_secs(60);

struct CachedCredentials {
    value: Option<LcuCredentials>,
    updated_at: Instant,
}

struct CachedInstallDir {
    value: Option<PathBuf>,
    updated_at: Instant,
}

static CREDENTIALS_CACHE: Mutex<Option<CachedCredentials>> = Mutex::new(None);
static INSTALL_DIR_CACHE: Mutex<Option<CachedInstallDir>> = Mutex::new(None);

#[derive(Debug, Clone)]
pub struct LcuCredentials {
    pub port: u16,
    pub token: String,
}

#[derive(Debug, Clone)]
pub struct LcuIdentity {
    pub game_name: String,
    pub tag_line: String,
}

/// Response from /lol-summoner/v1/current-summoner
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LcuCurrentSummoner {
    pub game_name: Option<String>,
    pub tag_line: Option<String>,
    pub display_name: Option<String>,
    pub profile_icon_id: Option<i64>,
    pub summoner_level: Option<i64>,
    pub puuid: Option<String>,
}

/// Response from /lol-chat/v1/me
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LcuChatMe {
    pub game_name: Option<String>,
    pub game_tag: Option<String>,
}

// ── Champ Select structs ──────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChampSelectSession {
    pub my_team: Option<Vec<ChampSelectPlayer>>,
    pub their_team: Option<Vec<ChampSelectPlayer>>,
    pub actions: Option<Vec<Vec<ChampSelectAction>>>,
    pub timer: Option<ChampSelectTimer>,
    pub bans: Option<ChampSelectBans>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChampSelectPlayer {
    pub cell_id: Option<i32>,
    pub champion_id: Option<i64>,
    pub summoner_id: Option<i64>,
    pub puuid: Option<String>,
    pub assigned_position: Option<String>,
    pub spell1_id: Option<i32>,
    pub spell2_id: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChampSelectAction {
    pub actor_cell_id: Option<i32>,
    pub champion_id: Option<i64>,
    pub completed: Option<bool>,
    #[serde(rename = "type")]
    pub action_type: Option<String>,
    pub is_in_progress: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChampSelectTimer {
    pub phase: Option<String>,
    pub adjusted_time_left_in_phase: Option<i64>,
    pub internal_now_in_epoch_ms: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChampSelectBans {
    pub my_team_bans: Option<Vec<i64>>,
    pub their_team_bans: Option<Vec<i64>>,
}

// ── Lockfile detection ────────────────────────────────────────────────────────

/// Cached version — returns credentials from cache if TTL has not expired.
/// All high-frequency pollers should use this.
pub fn detect_lcu_credentials() -> Option<LcuCredentials> {
    if let Ok(guard) = CREDENTIALS_CACHE.lock() {
        if let Some(cached) = guard.as_ref() {
            if cached.updated_at.elapsed() < CREDENTIALS_CACHE_TTL {
                return cached.value.clone();
            }
        }
    }

    let result = detect_lcu_credentials_fresh();

    if let Ok(mut guard) = CREDENTIALS_CACHE.lock() {
        *guard = Some(CachedCredentials {
            value: result.clone(),
            updated_at: Instant::now(),
        });
    }

    result
}

/// Bypasses the cache — use only for user-initiated actions (e.g. detect_account).
pub fn detect_lcu_credentials_fresh() -> Option<LcuCredentials> {
    let candidates = lockfile_candidates_from_known_paths();
    for path in &candidates {
        if let Some(creds) = try_read_lockfile(path) {
            // Also update the cache so pollers benefit immediately
            if let Ok(mut guard) = CREDENTIALS_CACHE.lock() {
                *guard = Some(CachedCredentials {
                    value: Some(creds.clone()),
                    updated_at: Instant::now(),
                });
            }
            return Some(creds);
        }
    }

    if let Some(dir) = find_league_dir_from_process() {
        let lockfile = dir.join("lockfile");
        if let Some(creds) = try_read_lockfile(&lockfile) {
            if let Ok(mut guard) = CREDENTIALS_CACHE.lock() {
                *guard = Some(CachedCredentials {
                    value: Some(creds.clone()),
                    updated_at: Instant::now(),
                });
            }
            return Some(creds);
        }
    }

    None
}

/// Cached version — install dir almost never changes at runtime.
pub fn find_league_install_dir() -> Option<PathBuf> {
    if let Ok(guard) = INSTALL_DIR_CACHE.lock() {
        if let Some(cached) = guard.as_ref() {
            if cached.updated_at.elapsed() < INSTALL_DIR_CACHE_TTL {
                return cached.value.clone();
            }
        }
    }

    let result = find_league_install_dir_fresh();

    if let Ok(mut guard) = INSTALL_DIR_CACHE.lock() {
        *guard = Some(CachedInstallDir {
            value: result.clone(),
            updated_at: Instant::now(),
        });
    }

    result
}

fn find_league_install_dir_fresh() -> Option<PathBuf> {
    for path in &lockfile_candidates_from_known_paths() {
        if path.exists() {
            if let Some(parent) = path.parent() {
                return Some(parent.to_path_buf());
            }
        }
    }

    find_league_dir_from_process()
}

pub fn is_game_fullscreen_mode() -> bool {
    let Some(install_dir) = find_league_install_dir() else {
        return false;
    };

    let game_cfg = install_dir.join("Config").join("game.cfg");
    let Ok(contents) = std::fs::read_to_string(game_cfg) else {
        return false;
    };

    let mut in_general_section = false;
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            in_general_section = line.eq_ignore_ascii_case("[General]");
            continue;
        }

        if !in_general_section {
            continue;
        }

        if let Some(value) = line.strip_prefix("WindowMode=") {
            return value.trim() == "0";
        }
    }

    false
}

fn lockfile_candidates_from_known_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for drive in &["C", "D", "E", "F"] {
        for subdir in &[
            "Riot Games\\League of Legends",
            "Program Files\\Riot Games\\League of Legends",
            "Program Files (x86)\\Riot Games\\League of Legends",
            "Games\\League of Legends",
            "Games\\Riot Games\\League of Legends",
        ] {
            paths.push(PathBuf::from(format!("{}:\\{}\\lockfile", drive, subdir)));
        }
    }
    paths
}

fn try_read_lockfile(path: &Path) -> Option<LcuCredentials> {
    let content = std::fs::read_to_string(path).ok()?;
    let parts: Vec<&str> = content.trim().split(':').collect();
    if parts.len() < 5 {
        return None;
    }
    let port = parts[2].parse::<u16>().ok()?;
    let token = parts[3].to_string();
    Some(LcuCredentials { port, token })
}

fn find_league_dir_from_process() -> Option<PathBuf> {
    let ps_cmd = r#"
        $procs = Get-WmiObject Win32_Process |
            Where-Object { $_.Name -like '*League*' -or $_.Name -like '*Riot*' } |
            Where-Object { $_.ExecutablePath -ne $null }
        $procs | ForEach-Object {
            Split-Path -Parent $_.ExecutablePath
        } | Select-Object -First 5
    "#;
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps_cmd])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        let p = Path::new(line);
        if p.join("lockfile").exists() {
            return Some(p.to_path_buf());
        }
        if let Some(parent) = p.parent() {
            if parent.join("lockfile").exists() {
                return Some(parent.to_path_buf());
            }
        }
    }
    None
}

// ── LCU API via curl.exe ──────────────────────────────────────────────────────

/// Uses system curl.exe to bypass TLS compatibility issues with reqwest/rustls
fn curl_lcu(port: u16, token: &str, path: &str) -> Result<String, String> {
    let url = format!("https://127.0.0.1:{}{}", port, path);
    let output = Command::new("curl.exe")
        .args(["-sk", "--max-time", "5", "-u", &format!("riot:{}", token), &url])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("curl.exe error: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if stdout.is_empty() {
        return Err(format!("curl.exe returned empty response for {}: {}", path, stderr));
    }
    if stdout.contains("\"errorCode\"") {
        return Err(format!("LCU error for {}: {}", path, stdout));
    }
    Ok(stdout)
}

pub async fn get_lcu_identity(creds: &LcuCredentials) -> Result<LcuIdentity, String> {
    // Strategy 1: /lol-summoner/v1/current-summoner (gives gameName, tagLine, icon, level)
    if let Ok(body) = curl_lcu(creds.port, &creds.token, "/lol-summoner/v1/current-summoner") {
        if let Ok(s) = serde_json::from_str::<LcuCurrentSummoner>(&body) {
            if let (Some(gn), Some(tl)) = (s.game_name.filter(|n| !n.is_empty()), s.tag_line.filter(|t| !t.is_empty())) {
                return Ok(LcuIdentity {
                    game_name: gn,
                    tag_line: tl,
                });
            }
        }
    }

    // Strategy 2: /lol-chat/v1/me (lighter endpoint, works more broadly)
    if let Ok(body) = curl_lcu(creds.port, &creds.token, "/lol-chat/v1/me") {
        if let Ok(chat) = serde_json::from_str::<LcuChatMe>(&body) {
            if let (Some(gn), Some(tl)) = (chat.game_name.filter(|n| !n.is_empty()), chat.game_tag.filter(|t| !t.is_empty())) {
                return Ok(LcuIdentity {
                    game_name: gn,
                    tag_line: tl,
                });
            }
        }
    }

    Err("LCU: не удалось получить Riot ID из клиента".to_string())
}

pub fn get_champ_select(creds: &LcuCredentials) -> Result<ChampSelectSession, String> {
    let body = curl_lcu(creds.port, &creds.token, "/lol-champ-select/v1/session")?;
    serde_json::from_str::<ChampSelectSession>(&body)
        .map_err(|e| format!("Champ select parse error: {}", e))
}

pub fn is_lcu_running() -> bool {
    detect_lcu_credentials().is_some()
}

/// Returns the current gameflow phase from the LCU.
/// Possible values: "None", "Lobby", "Matchmaking", "ReadyCheck",
/// "ChampSelect", "GameStart", "InProgress", "WaitingForStats",
/// "EndOfGame", "Reconnect", etc.
pub fn get_gameflow_phase(creds: &LcuCredentials) -> Result<String, String> {
    let body = curl_lcu(creds.port, &creds.token, "/lol-gameflow/v1/gameflow-phase")?;
    // The response is a JSON string like "InProgress" (with quotes)
    let phase = body.trim().trim_matches('"').to_string();
    Ok(phase)
}

// ── Live Client Data API (game client on :2999) ─────────────────────────────

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct LiveClientPlayer {
    pub riot_id_game_name: Option<String>,
    pub riot_id_tag_line: Option<String>,
    pub position: Option<String>,
    pub team: Option<String>,
}

pub fn get_live_client_playerlist() -> Result<Vec<LiveClientPlayer>, String> {
    let output = Command::new("curl.exe")
        .args(["-sk", "--max-time", "2", "https://127.0.0.1:2999/liveclientdata/playerlist"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("curl.exe error: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() || stdout.contains("\"errorCode\"") {
        return Err("Live Client API not available".to_string());
    }
    serde_json::from_str::<Vec<LiveClientPlayer>>(&stdout)
        .map_err(|e| format!("Live Client parse error: {}", e))
}

// ── All Game Data (rich live stats: items, gold, KDA, CS, events) ────────────

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct LiveAllGameData {
    pub active_player: Option<LiveActivePlayer>,
    pub all_players: Option<Vec<LiveFullPlayer>>,
    pub events: Option<LiveEvents>,
    pub game_data: Option<LiveGameInfo>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct LiveActivePlayer {
    pub summoner_name: Option<String>,
    pub current_gold: Option<f64>,
    pub level: Option<i32>,
    pub champion_stats: Option<LiveChampionStats>,
    pub full_runes: Option<LiveFullRunes>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct LiveChampionStats {
    pub attack_damage: Option<f64>,
    pub ability_power: Option<f64>,
    pub armor: Option<f64>,
    pub magic_resist: Option<f64>,
    pub current_health: Option<f64>,
    pub max_health: Option<f64>,
    pub attack_speed: Option<f64>,
    pub move_speed: Option<f64>,
    pub ability_haste: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct LiveFullRunes {
    pub keystone: Option<LiveRuneInfo>,
    pub primary_rune_tree: Option<LiveRuneInfo>,
    pub secondary_rune_tree: Option<LiveRuneInfo>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct LiveRuneInfo {
    pub display_name: Option<String>,
    pub id: Option<i64>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct LiveFullPlayer {
    pub champion_name: Option<String>,
    pub team: Option<String>,
    pub position: Option<String>,
    pub level: Option<i32>,
    pub scores: Option<LiveScores>,
    pub items: Option<Vec<LiveItem>>,
    pub summoner_name: Option<String>,
    pub riot_id_game_name: Option<String>,
    pub riot_id_tag_line: Option<String>,
    pub summoner_spells: Option<LiveSummonerSpells>,
    pub is_dead: Option<bool>,
    pub respawn_timer: Option<f64>,
    pub runes: Option<LivePlayerRunes>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct LiveSummonerSpells {
    pub summoner_spell_one: Option<LiveSpellInfo>,
    pub summoner_spell_two: Option<LiveSpellInfo>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct LiveSpellInfo {
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct LivePlayerRunes {
    pub keystone: Option<LiveRuneInfo>,
    pub primary_rune_tree: Option<LiveRuneInfo>,
    pub secondary_rune_tree: Option<LiveRuneInfo>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct LiveScores {
    pub kills: Option<i32>,
    pub deaths: Option<i32>,
    pub assists: Option<i32>,
    pub creep_score: Option<i32>,
    pub ward_score: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct LiveItem {
    #[serde(alias = "itemID")]
    pub item_id: Option<i32>,
    pub display_name: Option<String>,
    pub count: Option<i32>,
    pub price: Option<i32>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LiveEvents {
    #[serde(rename = "Events")]
    pub events: Option<Vec<LiveEventEntry>>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
#[allow(dead_code)]
pub struct LiveEventEntry {
    pub event_name: Option<String>,
    pub event_time: Option<f64>,
    pub killer_name: Option<String>,
    pub recipient: Option<String>,
    pub assisters: Option<Vec<String>>,
    pub dragon_type: Option<String>,
    pub turret_killed: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct LiveGameInfo {
    pub game_time: Option<f64>,
    pub map_number: Option<i32>,
    pub game_mode: Option<String>,
}

pub fn get_live_client_allgamedata() -> Result<LiveAllGameData, String> {
    let output = Command::new("curl.exe")
        .args(["-sk", "--max-time", "3", "https://127.0.0.1:2999/liveclientdata/allgamedata"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("curl.exe error: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() || stdout.contains("\"errorCode\"") {
        return Err("Live Client allgamedata not available".to_string());
    }
    serde_json::from_str::<LiveAllGameData>(&stdout)
        .map_err(|e| format!("Live Client allgamedata parse error: {}", e))
}
