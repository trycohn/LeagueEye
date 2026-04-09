use crate::lcu;
use crate::models::*;
use std::collections::HashMap;

// ─── Champion meta info (passed from commands.rs) ────────────────────────────

#[derive(Clone)]
pub struct ChampionMetaInfo {
    pub resource: String,
    pub tags: Vec<String>,
    pub abilities: Vec<ChampionAbility>,
    pub passive_name: String,
    pub passive_desc: String,
    pub ally_tips: Vec<String>,
}

#[derive(Clone)]
pub struct ChampionAbility {
    pub slot: String,
    pub name: String,
    pub short_desc: String,
}

// ─── State (deduplication — stays on client) ────────────────────────────────

pub struct CoachState {
    pub is_requesting: bool,
}

impl CoachState {
    pub fn new() -> Self {
        Self {
            is_requesting: false,
        }
    }
}

// ─── Build coaching context from live data (in-game) ────────────────────────

pub fn build_context_from_allgamedata(
    alldata: &lcu::LiveAllGameData,
    my_name: &str,
    champion_meta: &HashMap<String, ChampionMetaInfo>,
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

    // Get my champion meta
    let my_meta = champion_meta.get(&my_champ);
    let my_resource = my_meta.map(|m| m.resource.clone());
    let my_class = my_meta.and_then(|m| m.tags.first().cloned());

    // Build abilities summary for my champion
    let my_abilities_summary = my_meta.map(|m| {
        let mut parts = Vec::new();
        if !m.passive_name.is_empty() {
            let desc = if !m.passive_desc.is_empty() {
                format!(" — {}", m.passive_desc)
            } else {
                String::new()
            };
            parts.push(format!("  (Пассивное) {}{}", m.passive_name, desc));
        }
        for ability in &m.abilities {
            let desc = if !ability.short_desc.is_empty() {
                format!(" — {}", ability.short_desc)
            } else {
                String::new()
            };
            parts.push(format!("  ({}) {}{}", ability.slot, ability.name, desc));
        }
        parts.join("\n")
    });

    let my_ally_tips = my_meta.and_then(|m| {
        if m.ally_tips.is_empty() {
            None
        } else {
            Some(m.ally_tips.clone())
        }
    });

    let to_info = |p: &lcu::LiveFullPlayer| -> CoachPlayerInfo {
        let scores = p.scores.as_ref();
        let items: Vec<String> = p.items.as_ref()
            .map(|items| items.iter()
                .filter_map(|i| i.display_name.clone())
                .collect())
            .unwrap_or_default();

        let summoner_spells: Vec<String> = p.summoner_spells.as_ref()
            .map(|ss| {
                let mut v = Vec::new();
                if let Some(s) = ss.summoner_spell_one.as_ref().and_then(|s| s.display_name.clone()) {
                    v.push(s);
                }
                if let Some(s) = ss.summoner_spell_two.as_ref().and_then(|s| s.display_name.clone()) {
                    v.push(s);
                }
                v
            })
            .unwrap_or_default();

        let keystone_rune = p.runes.as_ref()
            .and_then(|r| r.keystone.as_ref())
            .and_then(|k| k.display_name.clone())
            .unwrap_or_default();

        let champ_name = p.champion_name.clone().unwrap_or_else(|| "?".into());
        let meta = champion_meta.get(&champ_name);

        CoachPlayerInfo {
            champion_name: champ_name,
            position: p.position.clone().unwrap_or_default(),
            rank_display: String::new(),
            kills: scores.and_then(|s| s.kills).unwrap_or(0),
            deaths: scores.and_then(|s| s.deaths).unwrap_or(0),
            assists: scores.and_then(|s| s.assists).unwrap_or(0),
            cs: scores.and_then(|s| s.creep_score).unwrap_or(0),
            level: p.level.unwrap_or(1),
            items,
            summoner_spells,
            keystone_rune,
            is_dead: p.is_dead.unwrap_or(false),
            respawn_timer: p.respawn_timer.unwrap_or(0.0),
            champion_resource: meta.map(|m| m.resource.clone()),
            champion_class: meta.and_then(|m| m.tags.first().cloned()),
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

    // Extract active player data (only available for the local player)
    let active = alldata.active_player.as_ref();
    let my_gold = active.and_then(|a| a.current_gold);

    let my_summoner_spells: Vec<String> = me
        .and_then(|p| p.summoner_spells.as_ref())
        .map(|ss| {
            let mut v = Vec::new();
            if let Some(s) = ss.summoner_spell_one.as_ref().and_then(|s| s.display_name.clone()) {
                v.push(s);
            }
            if let Some(s) = ss.summoner_spell_two.as_ref().and_then(|s| s.display_name.clone()) {
                v.push(s);
            }
            v
        })
        .unwrap_or_default();

    let my_runes: Option<String> = active
        .and_then(|a| a.full_runes.as_ref())
        .map(|r| {
            let keystone = r.keystone.as_ref()
                .and_then(|k| k.display_name.clone())
                .unwrap_or_else(|| "?".into());
            let primary = r.primary_rune_tree.as_ref()
                .and_then(|t| t.display_name.clone())
                .unwrap_or_else(|| "?".into());
            let secondary = r.secondary_rune_tree.as_ref()
                .and_then(|t| t.display_name.clone())
                .unwrap_or_else(|| "?".into());
            format!("{} ({} / {})", keystone, primary, secondary)
        });

    let my_stats: Option<CoachMyStats> = active
        .and_then(|a| a.champion_stats.as_ref())
        .map(|s| CoachMyStats {
            attack_damage: s.attack_damage.unwrap_or(0.0),
            ability_power: s.ability_power.unwrap_or(0.0),
            armor: s.armor.unwrap_or(0.0),
            magic_resist: s.magic_resist.unwrap_or(0.0),
            current_health: s.current_health.unwrap_or(0.0),
            max_health: s.max_health.unwrap_or(0.0),
            attack_speed: s.attack_speed.unwrap_or(0.0),
            move_speed: s.move_speed.unwrap_or(0.0),
            ability_haste: s.ability_haste.unwrap_or(0.0),
        });

    Some(CoachingContext {
        phase: "in_game".to_string(),
        game_time_secs: game_time,
        my_champion: my_champ,
        my_position: my_pos,
        my_gold,
        my_summoner_spells,
        my_runes,
        my_stats,
        my_team,
        enemy_team,
        recent_events,
        my_champion_resource: my_resource,
        my_champion_class: my_class,
        my_champion_abilities_summary: my_abilities_summary,
        my_champion_ally_tips: my_ally_tips,
    })
}

// ─── Build coaching context from champ select ───────────────────────────────

pub fn build_context_champ_select(
    my_team: &[LivePlayer],
    enemy_team: &[LivePlayer],
    champ_names: &HashMap<i64, String>,
    champion_meta: &HashMap<String, ChampionMetaInfo>,
    my_puuid: Option<&str>,
) -> CoachingContext {
    let to_info = |p: &LivePlayer| -> CoachPlayerInfo {
        let champ = champ_names.get(&p.champion_id)
            .cloned()
            .unwrap_or_else(|| format!("ChampID:{}", p.champion_id));
        let rank_display = p.rank.as_ref()
            .map(|r| format!("{} {} {} LP ({}% WR)", r.tier, r.rank, r.lp, r.winrate))
            .unwrap_or_else(|| "Unranked".to_string());

        let meta = champion_meta.get(&champ);

        CoachPlayerInfo {
            champion_name: champ.clone(),
            position: p.assigned_position.clone().unwrap_or_default(),
            rank_display,
            kills: 0,
            deaths: 0,
            assists: 0,
            cs: 0,
            level: 1,
            items: vec![],
            summoner_spells: vec![],
            keystone_rune: String::new(),
            is_dead: false,
            respawn_timer: 0.0,
            champion_resource: meta.map(|m| m.resource.clone()),
            champion_class: meta.and_then(|m| m.tags.first().cloned()),
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

    // Get my champion meta
    let my_meta = champion_meta.get(&my_champ);
    let my_resource = my_meta.map(|m| m.resource.clone());
    let my_class = my_meta.and_then(|m| m.tags.first().cloned());

    let my_abilities_summary = my_meta.map(|m| {
        let mut parts = Vec::new();
        if !m.passive_name.is_empty() {
            let desc = if !m.passive_desc.is_empty() {
                format!(" — {}", m.passive_desc)
            } else {
                String::new()
            };
            parts.push(format!("  (Пассивное) {}{}", m.passive_name, desc));
        }
        for ability in &m.abilities {
            let desc = if !ability.short_desc.is_empty() {
                format!(" — {}", ability.short_desc)
            } else {
                String::new()
            };
            parts.push(format!("  ({}) {}{}", ability.slot, ability.name, desc));
        }
        parts.join("\n")
    });

    let my_ally_tips = my_meta.and_then(|m| {
        if m.ally_tips.is_empty() {
            None
        } else {
            Some(m.ally_tips.clone())
        }
    });

    CoachingContext {
        phase: "champ_select".to_string(),
        game_time_secs: None,
        my_champion: my_champ,
        my_position: my_pos,
        my_gold: None,
        my_summoner_spells: vec![],
        my_runes: None,
        my_stats: None,
        my_team: my_team.iter().map(to_info).collect(),
        enemy_team: enemy_team.iter().map(to_info).collect(),
        recent_events: vec![],
        my_champion_resource: my_resource,
        my_champion_class: my_class,
        my_champion_abilities_summary: my_abilities_summary,
        my_champion_ally_tips: my_ally_tips,
    }
}
