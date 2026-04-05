use serde::{Deserialize, Serialize};

// --- Riot Account ---

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RiotAccount {
    pub puuid: String,
    pub game_name: Option<String>,
    pub tag_line: Option<String>,
}

// --- Summoner ---

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Summoner {
    pub id: Option<String>,
    pub account_id: Option<String>,
    pub puuid: String,
    pub profile_icon_id: i64,
    pub summoner_level: i64,
}

// --- League / Rank ---

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LeagueEntry {
    pub queue_type: String,
    pub tier: Option<String>,
    pub rank: Option<String>,
    pub league_points: Option<i32>,
    pub wins: i32,
    pub losses: i32,
    pub hot_streak: Option<bool>,
    pub veteran: Option<bool>,
    pub fresh_blood: Option<bool>,
    pub inactive: Option<bool>,
}

// --- Champion Mastery ---

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChampionMastery {
    pub champion_id: i64,
    pub champion_level: i32,
    pub champion_points: i64,
    pub last_play_time: Option<i64>,
    pub champion_points_since_last_level: Option<i64>,
    pub champion_points_until_next_level: Option<i64>,
}

// --- Match ---

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MatchDto {
    pub metadata: MatchMetadata,
    pub info: MatchInfo,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MatchMetadata {
    pub match_id: String,
    pub participants: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MatchInfo {
    pub game_duration: i64,
    pub game_mode: String,
    pub game_type: Option<String>,
    pub queue_id: i32,
    pub game_creation: i64,
    pub participants: Vec<Participant>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Participant {
    pub puuid: String,
    pub summoner_name: Option<String>,
    pub riot_id_game_name: Option<String>,
    pub riot_id_tagline: Option<String>,
    pub champion_id: i64,
    pub champion_name: String,
    pub team_id: i32,
    pub win: bool,
    pub kills: i32,
    pub deaths: i32,
    pub assists: i32,
    pub total_minions_killed: i32,
    pub neutral_minions_killed: Option<i32>,
    pub gold_earned: i32,
    pub champ_level: i32,
    pub total_damage_dealt_to_champions: i64,
    pub vision_score: Option<i32>,
    pub individual_position: Option<String>,
    pub team_position: Option<String>,
    pub item0: i32,
    pub item1: i32,
    pub item2: i32,
    pub item3: i32,
    pub item4: i32,
    pub item5: i32,
    pub item6: i32,
    pub summoner1_id: Option<i32>,
    pub summoner2_id: Option<i32>,
    pub total_damage_taken: Option<i64>,
    pub wards_placed: Option<i32>,
    pub wards_killed: Option<i32>,
    pub double_kills: Option<i32>,
    pub triple_kills: Option<i32>,
    pub quadra_kills: Option<i32>,
    pub penta_kills: Option<i32>,
    pub first_blood_kill: Option<bool>,
    pub first_blood_assist: Option<bool>,
    pub turret_kills: Option<i32>,
    pub inhibitor_kills: Option<i32>,
}

// --- Spectator API v5 ---

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SpectatorGame {
    pub game_id: i64,
    pub game_mode: String,
    pub game_type: Option<String>,
    pub game_queue_config_id: Option<i32>,
    pub participants: Vec<SpectatorParticipant>,
    pub banned_champions: Option<Vec<BannedChampion>>,
    pub game_start_time: Option<i64>,
    pub game_length: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SpectatorParticipant {
    pub puuid: Option<String>,
    pub team_id: i32,
    pub champion_id: i64,
    pub spell1_id: Option<i32>,
    pub spell2_id: Option<i32>,
    pub riot_id: Option<String>,
    pub summoner_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BannedChampion {
    pub champion_id: i64,
    pub team_id: i32,
    pub pick_turn: i32,
}

// --- Live Game DTOs (frontend) ---

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LiveGameData {
    pub phase: String,
    pub queue_id: Option<i32>,
    pub my_team: Vec<LivePlayer>,
    pub enemy_team: Vec<LivePlayer>,
    pub bans: Vec<LiveBan>,
    pub game_time: Option<i64>,
    pub timer: Option<LiveTimer>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LivePlayer {
    pub puuid: Option<String>,
    pub game_name: Option<String>,
    pub tag_line: Option<String>,
    pub champion_id: i64,
    pub assigned_position: Option<String>,
    pub spell1_id: i32,
    pub spell2_id: i32,
    pub team_id: i32,
    pub rank: Option<RankInfo>,
    pub is_picking: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LiveBan {
    pub champion_id: i64,
    pub team_id: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LiveTimer {
    pub phase: String,
    pub time_left_ms: i64,
}

// --- Frontend DTOs ---

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlayerProfile {
    pub puuid: String,
    pub game_name: String,
    pub tag_line: String,
    pub summoner_level: i64,
    pub profile_icon_id: i64,
    pub ranked: Vec<RankInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RankInfo {
    pub queue_type: String,
    pub tier: String,
    pub rank: String,
    pub lp: i32,
    pub wins: i32,
    pub losses: i32,
    pub winrate: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MasteryInfo {
    pub champion_id: i64,
    pub champion_level: i32,
    pub champion_points: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MatchSummary {
    pub match_id: String,
    pub champion_name: String,
    pub champion_id: i64,
    pub win: bool,
    pub kills: i32,
    pub deaths: i32,
    pub assists: i32,
    pub cs: i32,
    pub gold: i32,
    pub damage: i64,
    pub vision_score: i32,
    pub position: String,
    pub game_duration: i64,
    pub game_creation: i64,
    pub queue_id: i32,
    pub items: Vec<i32>,
    pub summoner_spells: Vec<i32>,
    pub lp_delta: Option<i32>,
}

// --- Match Detail (full game with all participants) ---

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MatchParticipantDetail {
    pub puuid: String,
    pub riot_id_name: String,
    pub riot_id_tagline: String,
    pub champion_id: i64,
    pub champion_name: String,
    pub champ_level: i32,
    pub team_id: i32,
    pub win: bool,
    pub kills: i32,
    pub deaths: i32,
    pub assists: i32,
    pub cs: i32,
    pub gold: i32,
    pub damage: i64,
    pub damage_taken: i64,
    pub vision_score: i32,
    pub wards_placed: i32,
    pub wards_killed: i32,
    pub position: String,
    pub items: Vec<i32>,
    pub summoner_spells: Vec<i32>,
    pub double_kills: i32,
    pub triple_kills: i32,
    pub quadra_kills: i32,
    pub penta_kills: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MatchDetail {
    pub match_id: String,
    pub game_duration: i64,
    pub game_creation: i64,
    pub queue_id: i32,
    pub participants: Vec<MatchParticipantDetail>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChampionStat {
    pub champion_id: i64,
    pub champion_name: String,
    pub games: i32,
    pub wins: i32,
    pub winrate: f64,
    pub avg_kills: f64,
    pub avg_deaths: f64,
    pub avg_assists: f64,
    pub avg_cs: f64,
    pub position: String,
}
