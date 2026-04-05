use crate::lcu;
use crate::models::*;
use std::collections::HashMap;

// ─── State (deduplication — stays on client) ────────────────────────────────

pub struct CoachState {
    pub is_requesting: bool,
    pub last_state_hash: u64,
}

impl CoachState {
    pub fn new() -> Self {
        Self {
            is_requesting: false,
            last_state_hash: 0,
        }
    }
}

// ─── Build coaching context from live data (in-game) ────────────────────────

pub fn build_context_from_allgamedata(
    alldata: &lcu::LiveAllGameData,
    my_name: &str,
) -> Option<CoachingContext> {
    let players = alldata.all_players.as_ref()?;
    let game_info = alldata.game_data.as_ref();

    let game_time = game_info.and_then(|g| g.game_time.map(|t| t as i64));

    let me = players.iter().find(|p| {
        p.riot_id_game_name.as_deref() == Some(my_name)
            || p.summoner_name.as_deref() == Some(my_name)
    });

    let my_team_str = me.and_then(|p| p.team.clone()).unwrap_or_else(|| "ORDER".to_string());
    let my_champ = me.and_then(|p| p.champion_name.clone()).unwrap_or_default();
    let my_pos = me.and_then(|p| p.position.clone()).unwrap_or_default();

    let to_info = |p: &lcu::LiveFullPlayer| -> CoachPlayerInfo {
        let scores = p.scores.as_ref();
        let items: Vec<String> = p.items.as_ref()
            .map(|items| items.iter()
                .filter_map(|i| i.display_name.clone())
                .collect())
            .unwrap_or_default();

        CoachPlayerInfo {
            champion_name: p.champion_name.clone().unwrap_or_else(|| "?".into()),
            position: p.position.clone().unwrap_or_default(),
            rank_display: String::new(),
            kills: scores.and_then(|s| s.kills).unwrap_or(0),
            deaths: scores.and_then(|s| s.deaths).unwrap_or(0),
            assists: scores.and_then(|s| s.assists).unwrap_or(0),
            cs: scores.and_then(|s| s.creep_score).unwrap_or(0),
            level: p.level.unwrap_or(1),
            items,
        }
    };

    let mut my_team = Vec::new();
    let mut enemy_team = Vec::new();
    for p in players {
        let info = to_info(p);
        if p.team.as_deref() == Some(&my_team_str) {
            my_team.push(info);
        } else {
            enemy_team.push(info);
        }
    }

    let recent_events: Vec<String> = alldata.events.as_ref()
        .and_then(|ev| ev.events.as_ref())
        .map(|events| {
            events.iter().rev().take(8)
                .filter_map(|e| {
                    let name = e.event_name.as_deref()?;
                    let time = e.event_time.unwrap_or(0.0);
                    let mins = (time / 60.0) as i32;
                    let secs = (time % 60.0) as i32;
                    let mut desc = format!("[{mins}:{secs:02}] {name}");
                    if let Some(killer) = &e.killer_name {
                        desc.push_str(&format!(" by {killer}"));
                    }
                    if let Some(dragon) = &e.dragon_type {
                        desc.push_str(&format!(" ({dragon})"));
                    }
                    Some(desc)
                })
                .collect()
        })
        .unwrap_or_default();

    Some(CoachingContext {
        phase: "in_game".to_string(),
        game_time_secs: game_time,
        my_champion: my_champ,
        my_position: my_pos,
        my_team,
        enemy_team,
        recent_events,
    })
}

// ─── Build coaching context from champ select ───────────────────────────────

pub fn build_context_champ_select(
    my_team: &[LivePlayer],
    enemy_team: &[LivePlayer],
    champ_names: &HashMap<i64, String>,
    my_puuid: Option<&str>,
) -> CoachingContext {
    let to_info = |p: &LivePlayer| -> CoachPlayerInfo {
        let champ = champ_names.get(&p.champion_id)
            .cloned()
            .unwrap_or_else(|| format!("ChampID:{}", p.champion_id));
        let rank_display = p.rank.as_ref()
            .map(|r| format!("{} {} {} LP ({}% WR)", r.tier, r.rank, r.lp, r.winrate))
            .unwrap_or_else(|| "Unranked".to_string());

        CoachPlayerInfo {
            champion_name: champ,
            position: p.assigned_position.clone().unwrap_or_default(),
            rank_display,
            kills: 0,
            deaths: 0,
            assists: 0,
            cs: 0,
            level: 1,
            items: vec![],
        }
    };

    let my_champ = my_puuid
        .and_then(|puuid| my_team.iter().find(|p| p.puuid.as_deref() == Some(puuid)))
        .map(|p| champ_names.get(&p.champion_id).cloned().unwrap_or_default())
        .unwrap_or_default();
    let my_pos = my_puuid
        .and_then(|puuid| my_team.iter().find(|p| p.puuid.as_deref() == Some(puuid)))
        .and_then(|p| p.assigned_position.clone())
        .unwrap_or_default();

    CoachingContext {
        phase: "champ_select".to_string(),
        game_time_secs: None,
        my_champion: my_champ,
        my_position: my_pos,
        my_team: my_team.iter().map(to_info).collect(),
        enemy_team: enemy_team.iter().map(to_info).collect(),
        recent_events: vec![],
    }
}
