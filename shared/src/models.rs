use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalStats {
    pub total_players: i64,
    pub analyzed_matches: i64,
    pub hours_played: i64,
    pub pentakills: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BestPlayerRole {
    pub role: String,
    pub player: String,
    pub tag: String,
    pub champ: String,
    pub winrate: String,
    pub kda: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopWinrateChampion {
    pub champ: String,
    pub winrate: String,
    pub games: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalDashboardData {
    pub stats: GlobalStats,
    pub best_by_role: Vec<BestPlayerRole>,
    pub top_winrates: Vec<TopWinrateChampion>,
}

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

// --- MatchesAndStats (combined response) ---

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MatchesAndStats {
    pub matches: Vec<MatchSummary>,
    pub champion_stats: Vec<ChampionStat>,
    pub total_cached: i64,
    pub total_wins: i32,
    pub total_losses: i32,
}

// --- DetectedAccount ---

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DetectedAccount {
    pub puuid: String,
    pub game_name: String,
    pub tag_line: String,
    pub profile_icon_id: i64,
    pub summoner_level: i64,
    pub ranked: Vec<RankInfo>,
}

// --- StoredAccount (DB cache) ---

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StoredAccount {
    pub puuid: String,
    pub game_name: String,
    pub tag_line: String,
    pub profile_icon_id: i64,
    pub summoner_level: i64,
}

// --- Helper: build RankInfo from LeagueEntry ---

pub fn build_rank_info(entries: Vec<LeagueEntry>) -> Vec<RankInfo> {
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

// --- Helper: build ChampionStats from matches ---

pub fn build_champion_stats(matches: &[MatchSummary]) -> Vec<ChampionStat> {
    use std::collections::HashMap;

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

// --- Helper: convert MatchDto to MatchParticipantDetail ---

pub fn dto_to_participants(m: &MatchDto) -> Vec<MatchParticipantDetail> {
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

// --- Helper: convert MatchDto to MatchSummary ---

// --- AI Coach DTOs ---

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CoachPlayerInfo {
    pub champion_name: String,
    pub position: String,
    pub rank_display: String,
    pub kills: i32,
    pub deaths: i32,
    pub assists: i32,
    pub cs: i32,
    pub level: i32,
    pub items: Vec<String>,
    pub summoner_spells: Vec<String>,
    pub keystone_rune: String,
    pub is_dead: bool,
    pub respawn_timer: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CoachMyStats {
    pub attack_damage: f64,
    pub ability_power: f64,
    pub armor: f64,
    pub magic_resist: f64,
    pub current_health: f64,
    pub max_health: f64,
    pub attack_speed: f64,
    pub move_speed: f64,
    pub ability_haste: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CoachingContext {
    pub phase: String,
    pub game_time_secs: Option<i64>,
    pub my_champion: String,
    pub my_position: String,
    pub my_gold: Option<f64>,
    pub my_summoner_spells: Vec<String>,
    pub my_runes: Option<String>,
    pub my_stats: Option<CoachMyStats>,
    pub my_team: Vec<CoachPlayerInfo>,
    pub enemy_team: Vec<CoachPlayerInfo>,
    pub recent_events: Vec<String>,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CoachStreamPayload {
    pub kind: String, // "start" | "delta" | "end" | "error"
    pub text: Option<String>,
}

pub fn dto_to_summary(m: &MatchDto, puuid: &str, lp_delta_fn: impl Fn(&str, i64, i64) -> Option<i32>) -> Option<MatchSummary> {
    let p = m.info.participants.iter().find(|p| p.puuid == puuid)?;
    let cs = p.total_minions_killed + p.neutral_minions_killed.unwrap_or(0);
    let game_end_ms = m.info.game_creation + m.info.game_duration * 1000;
    let lp_delta = lp_delta_fn(puuid, m.info.game_creation, game_end_ms);
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
