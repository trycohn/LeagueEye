use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::ai_coach::ChampionMetaInfo;
use crate::lcu;

const DDRAGON_VERSION: &str = "16.7.1";

#[derive(Clone, Default)]
pub struct ItemCatalogCache {
    pub snapshot: Option<ItemCatalogSnapshot>,
}

impl ItemCatalogCache {
    pub fn new() -> Self {
        Self { snapshot: None }
    }
}

#[derive(Clone, Default)]
pub struct ItemCatalogSnapshot {
    pub item_costs: HashMap<i32, i32>,
    pub items_by_id: HashMap<i32, ItemAdvisorEntry>,
    pub counter_candidates: Vec<ItemAdvisorEntry>,
}

#[derive(Clone, Debug)]
pub struct ItemAdvisorEntry {
    pub id: i32,
    pub name: String,
    pub gold_total: i32,
    pub signals: ItemSignals,
}

#[derive(Clone, Debug, Default)]
pub struct ItemSignals {
    pub anti_heal: bool,
    pub anti_shield: bool,
    pub anti_crit: bool,
    pub armor: bool,
    pub magic_resist: bool,
    pub mixed_defense: bool,
    pub burst_protection: bool,
    pub spell_block: bool,
    pub cc_cleanse: bool,
    pub anti_tank: bool,
    pub anti_dps: bool,
    pub ap_item: bool,
    pub ad_item: bool,
    pub tank_item: bool,
    pub crit_item: bool,
    pub attack_speed_item: bool,
    pub on_hit_item: bool,
    pub healing_item: bool,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CounterItemSuggestion {
    pub item_id: i32,
    pub name: String,
    pub build_reason: Option<String>,
    pub counter_reason: String,
}

#[derive(Clone, Debug)]
struct BuyerProfile {
    ap_weight: f32,
    ad_weight: f32,
    tank_weight: f32,
    is_marksman: bool,
}

impl Default for BuyerProfile {
    fn default() -> Self {
        Self {
            ap_weight: 0.0,
            ad_weight: 0.5,
            tank_weight: 0.0,
            is_marksman: false,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct ThreatProfile {
    healing: f32,
    shields: f32,
    ap_burst: f32,
    ad_burst: f32,
    autoattack: f32,
    crit: f32,
    on_hit: f32,
    hard_cc: f32,
    tanky: f32,
    magic_damage: f32,
    physical_damage: f32,
}

pub async fn get_or_fetch_item_catalog(
    cache: &Arc<Mutex<ItemCatalogCache>>,
) -> ItemCatalogSnapshot {
    {
        let c = cache.lock().unwrap();
        if let Some(snapshot) = &c.snapshot {
            return snapshot.clone();
        }
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    let url_en = format!(
        "https://ddragon.leagueoflegends.com/cdn/{}/data/en_US/item.json",
        DDRAGON_VERSION
    );
    let url_ru = format!(
        "https://ddragon.leagueoflegends.com/cdn/{}/data/ru_RU/item.json",
        DDRAGON_VERSION
    );

    let (en_result, ru_result) = tokio::join!(fetch_item_json(&client, &url_en), fetch_item_json(&client, &url_ru));

    let snapshot = match en_result {
        Ok(en_json) => build_item_snapshot(en_json, ru_result.ok()),
        Err(err) => {
            log::warn!("[gold_counter] Failed to load item.json: {}", err);
            ItemCatalogSnapshot::default()
        }
    };

    if !snapshot.item_costs.is_empty() {
        let mut c = cache.lock().unwrap();
        c.snapshot = Some(snapshot.clone());
    }

    snapshot
}

pub fn recommend_counter_item(
    catalog: &ItemCatalogSnapshot,
    my_player: &lcu::LiveFullPlayer,
    my_meta: Option<&ChampionMetaInfo>,
    enemy_player: &lcu::LiveFullPlayer,
    enemy_meta: Option<&ChampionMetaInfo>,
    enemy_champion_name: &str,
    game_time: Option<i64>,
) -> Option<CounterItemSuggestion> {
    if catalog.counter_candidates.is_empty() {
        return None;
    }

    let buyer = derive_buyer_profile(my_player, my_meta, catalog);
    let threat = derive_enemy_threat(enemy_player, enemy_meta, catalog);
    let owned_item_ids: HashSet<i32> = my_player
        .items
        .as_ref()
        .map(|items| {
            items.iter()
                .filter_map(|item| item.item_id)
                .filter(|id| *id > 0)
                .collect()
        })
        .unwrap_or_default();

    choose_counter_item(
        &catalog.counter_candidates,
        &buyer,
        &threat,
        &owned_item_ids,
        enemy_champion_name,
        game_time,
    )
}

async fn fetch_item_json(
    client: &reqwest::Client,
    url: &str,
) -> Result<serde_json::Value, String> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {} from {}", resp.status(), url));
    }

    resp.json::<serde_json::Value>()
        .await
        .map_err(|e| format!("JSON parse failed: {}", e))
}

fn build_item_snapshot(
    en_json: serde_json::Value,
    ru_json: Option<serde_json::Value>,
) -> ItemCatalogSnapshot {
    let Some(en_data) = en_json.get("data").and_then(|d| d.as_object()) else {
        return ItemCatalogSnapshot::default();
    };

    let ru_names: HashMap<String, String> = ru_json
        .as_ref()
        .and_then(|json| json.get("data"))
        .and_then(|data| data.as_object())
        .map(|data| {
            data.iter()
                .filter_map(|(id, info)| {
                    let name = info.get("name")?.as_str()?;
                    Some((id.clone(), name.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();

    let mut item_costs = HashMap::new();
    let mut items_by_id = HashMap::new();
    let mut counter_candidates = Vec::new();

    for (id_str, info) in en_data {
        let Ok(id) = id_str.parse::<i32>() else { continue };
        let Some(en_name) = info.get("name").and_then(|v| v.as_str()) else { continue };
        let gold_total = info
            .pointer("/gold/total")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;

        if gold_total > 0 {
            item_costs.insert(id, gold_total);
        }

        let display_name = ru_names
            .get(id_str)
            .cloned()
            .unwrap_or_else(|| en_name.to_string());

        let signals = build_item_signals(info, en_name);
        let entry = ItemAdvisorEntry {
            id,
            name: display_name,
            gold_total,
            signals,
        };

        items_by_id.insert(id, entry.clone());

        if is_counter_item_candidate(info) {
            counter_candidates.push(entry);
        }
    }

    log::info!(
        "[gold_counter] Loaded {} items, {} counter candidates",
        items_by_id.len(),
        counter_candidates.len()
    );

    ItemCatalogSnapshot {
        item_costs,
        items_by_id,
        counter_candidates,
    }
}

fn build_item_signals(info: &serde_json::Value, en_name: &str) -> ItemSignals {
    let tags = item_tags(info);
    let stats = item_stats(info);
    let text = normalize_item_text(info, en_name);
    let name = en_name.to_lowercase();

    let armor = tags.iter().any(|tag| tag == "armor") || stats_has(&stats, "Armor");
    let magic_resist = tags.iter().any(|tag| tag == "spellblock") || stats_has(&stats, "SpellBlock");
    let health = tags.iter().any(|tag| tag == "health") || stats_has(&stats, "HPPool");
    let ap_item = tags.iter().any(|tag| tag == "spelldamage") || stats_has(&stats, "MagicDamage");
    let crit_item = tags.iter().any(|tag| tag == "criticalstrike") || stats_has(&stats, "CritChance");
    let attack_speed_item = tags.iter().any(|tag| tag == "attackspeed") || stats_has(&stats, "AttackSpeed");
    let lifesteal = tags.iter().any(|tag| tag == "lifesteal")
        || stats_has(&stats, "LifeSteal")
        || text_has(&text, &["lifesteal", "life steal", "omnivamp"]);
    let ad_item = tags.iter().any(|tag| tag == "damage")
        || crit_item
        || attack_speed_item
        || lifesteal
        || stats_has(&stats, "PhysicalDamage");
    let on_hit_item = text_has(&text, &["on-hit", "on hit"]);
    let tank_item = (armor || magic_resist || health) && !ap_item && !ad_item;

    let anti_heal = text_has(&text, &["grievous wounds"]);
    let anti_shield = text_has(
        &text,
        &[
            "shield reaver",
            "enemy shields",
            "shielded target",
            "shields take",
        ],
    );
    let spell_block = text_has(&text, &["spell shield"]);
    let cc_cleanse = text_has(
        &text,
        &[
            "remove all crowd control debuffs",
            "remove crowd control debuffs",
            "cleanse all crowd control",
            "quicksilver",
        ],
    ) || name.contains("mercurial")
        || name.contains("silvermere");
    let burst_protection = text_has(
        &text,
        &[
            "stasis",
            "spell shield",
            "lifeline",
            "becoming immune",
            "cannot take damage",
        ],
    ) || name.contains("zhonya")
        || name.contains("banshee")
        || name.contains("edge of night")
        || name.contains("maw of malmortius");
    let anti_crit = text_has(&text, &["critical strike damage"]) || name.contains("randuin");
    let sustain = lifesteal
        || text_has(
            &text,
            &[
                "restore health",
                "heal",
                "healing",
                "omnivamp",
                "life steal",
            ],
        );
    let anti_tank = tags.iter().any(|tag| tag == "armorpenetration" || tag == "magicpenetration")
        || text_has(
            &text,
            &[
                "max health",
                "% maximum health",
                "bonus health",
                "giant slayer",
            ],
        );
    let anti_dps = text_has(
        &text,
        &[
            "reduce the attack speed",
            "attack speed of nearby enemies",
            "basic attack damage",
            "rock solid",
            "cripple",
        ],
    ) || name.contains("frozen heart")
        || name.contains("randuin")
        || name.contains("thornmail");

    ItemSignals {
        anti_heal,
        anti_shield,
        anti_crit,
        armor,
        magic_resist,
        mixed_defense: armor && magic_resist,
        burst_protection,
        spell_block,
        cc_cleanse,
        anti_tank,
        anti_dps,
        ap_item,
        ad_item,
        tank_item,
        crit_item,
        attack_speed_item,
        on_hit_item,
        healing_item: sustain,
    }
}

fn is_counter_item_candidate(info: &serde_json::Value) -> bool {
    let on_summoners_rift = info
        .pointer("/maps/11")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !on_summoners_rift {
        return false;
    }

    let purchasable = info
        .pointer("/gold/purchasable")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !purchasable {
        return false;
    }

    let gold_total = info
        .pointer("/gold/total")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    if gold_total <= 0 {
        return false;
    }

    let into_empty = match info.get("into") {
        None => true,
        Some(into) => into.as_array().map(|arr| arr.is_empty()).unwrap_or(true),
    };
    if !into_empty {
        return false;
    }

    let has_recipe = info
        .get("from")
        .and_then(|from| from.as_array())
        .map(|arr| !arr.is_empty())
        .unwrap_or(false);
    if !has_recipe {
        return false;
    }

    let tags = item_tags(info);
    if tags.iter().any(|tag| tag == "consumable" || tag == "trinket" || tag == "boots") {
        return false;
    }

    if info.get("requiredChampion").is_some_and(|v| !v.is_null()) {
        return false;
    }
    if info.get("requiredAlly").is_some_and(|v| !v.is_null()) {
        return false;
    }
    if info.get("inStore").and_then(|v| v.as_bool()) == Some(false) {
        return false;
    }
    if info.get("hideFromAll").and_then(|v| v.as_bool()) == Some(true) {
        return false;
    }

    true
}

fn derive_buyer_profile(
    my_player: &lcu::LiveFullPlayer,
    meta: Option<&ChampionMetaInfo>,
    catalog: &ItemCatalogSnapshot,
) -> BuyerProfile {
    let is_marksman = meta
        .map(|m| m.tags.iter().any(|t| t == "Marksman"))
        .unwrap_or(false);

    // Try to derive from actual items first
    if let Some(items) = my_player.items.as_ref() {
        let mut ap_gold = 0i32;
        let mut ad_gold = 0i32;
        let mut tank_gold = 0i32;
        let mut total_gold = 0i32;

        for item in items {
            let Some(item_id) = item.item_id else { continue };
            let Some(entry) = catalog.items_by_id.get(&item_id) else { continue };
            let gold = entry.gold_total.max(0);
            if gold == 0 { continue; }
            total_gold += gold;

            if entry.signals.ap_item { ap_gold += gold; }
            if entry.signals.ad_item { ad_gold += gold; }
            if entry.signals.tank_item || entry.signals.armor || entry.signals.magic_resist {
                tank_gold += gold;
            }
        }

        // If we have meaningful item data (at least one completed item worth >= 1000g)
        if total_gold >= 1000 {
            let total = total_gold as f32;
            return BuyerProfile {
                ap_weight: (ap_gold as f32 / total).min(1.0),
                ad_weight: (ad_gold as f32 / total).min(1.0),
                tank_weight: (tank_gold as f32 / total).min(1.0),
                is_marksman,
            };
        }
    }

    // Fallback: derive from champion meta tags
    let Some(meta) = meta else {
        return BuyerProfile {
            ad_weight: 0.5,
            is_marksman,
            ..BuyerProfile::default()
        };
    };

    let tags: HashSet<&str> = meta.tags.iter().map(String::as_str).collect();
    let text = champion_text_blob(meta);
    let magic_hits = count_hits(&text, &["magic damage", "ability power"]);
    let physical_hits = count_hits(&text, &["physical damage", "attack damage"]);

    let mage = tags.contains("Mage");
    let fighter = tags.contains("Fighter");
    let tank = tags.contains("Tank");
    let assassin = tags.contains("Assassin");

    let ap_signal = mage || magic_hits > physical_hits + 1 || (assassin && magic_hits > physical_hits);
    let ad_signal = is_marksman
        || physical_hits > magic_hits
        || (fighter && !ap_signal)
        || (assassin && !ap_signal);

    let ap_weight = if ap_signal { 0.7 } else { 0.0 };
    let mut ad_weight = if ad_signal { 0.7 } else { 0.0 };
    let tank_weight = if tank { 0.6 } else if fighter { 0.3 } else { 0.0 };

    if !ap_signal && !ad_signal && !tank {
        ad_weight = 0.5;
    }

    BuyerProfile {
        ap_weight,
        ad_weight,
        tank_weight,
        is_marksman,
    }
}

fn derive_enemy_threat(
    enemy_player: &lcu::LiveFullPlayer,
    enemy_meta: Option<&ChampionMetaInfo>,
    catalog: &ItemCatalogSnapshot,
) -> ThreatProfile {
    let mut threat = ThreatProfile::default();

    // Base level from champion meta tags (0.3 baseline)
    if let Some(meta) = enemy_meta {
        let tags: HashSet<&str> = meta.tags.iter().map(String::as_str).collect();
        let text = champion_text_blob(meta);
        let magic_hits = count_hits(&text, &["magic damage"]);
        let physical_hits = count_hits(&text, &["physical damage"]);
        let magic_pref = magic_hits > physical_hits;
        let physical_pref = physical_hits > magic_hits;

        if text_has(
            &text,
            &["heal", "heals", "healing", "restore health", "lifesteal", "life steal", "omnivamp", "vamp"],
        ) {
            threat.healing = 0.3;
        }
        if text_has(&text, &["shield", "shields"]) {
            threat.shields = 0.3;
        }
        if text_has(
            &text,
            &["stun", "root", "snare", "taunt", "fear", "charm", "suppress", "airborne", "knock up", "sleep", "polymorph"],
        ) {
            threat.hard_cc = 0.3;
        }
        if tags.contains("Marksman")
            || count_hits(&text, &["basic attack", "critical strike", "attack speed"]) > 0
        {
            threat.autoattack = 0.3;
        }
        if count_hits(&text, &["critical strike"]) > 0 {
            threat.crit = 0.3;
        }
        if count_hits(&text, &["on-hit", "on hit"]) > 0 {
            threat.on_hit = 0.3;
        }
        if tags.contains("Tank") || count_hits(&text, &["max health", "bonus health"]) > 0 {
            threat.tanky = 0.3;
        }
        if tags.contains("Mage") || magic_pref {
            threat.magic_damage = 0.3;
        }
        if tags.contains("Marksman") || tags.contains("Fighter") || physical_pref {
            threat.physical_damage = 0.3;
        }
        if tags.contains("Mage") || (tags.contains("Assassin") && magic_pref) {
            threat.ap_burst = 0.3;
        }
        if tags.contains("Marksman")
            || tags.contains("Fighter")
            || (tags.contains("Assassin") && !magic_pref)
        {
            threat.ad_burst = 0.3;
        }
    }

    // Amplify from actual enemy items
    if let Some(items) = enemy_player.items.as_ref() {
        for item in items {
            let Some(item_id) = item.item_id else { continue };
            let Some(entry) = catalog.items_by_id.get(&item_id) else { continue };
            let boost = if entry.gold_total >= 2000 { 0.25 } else { 0.15 };

            if entry.signals.healing_item { threat.healing = (threat.healing + boost).min(1.0); }
            if entry.signals.crit_item { threat.crit = (threat.crit + boost).min(1.0); }
            if entry.signals.attack_speed_item || entry.signals.crit_item {
                threat.autoattack = (threat.autoattack + boost).min(1.0);
            }
            if entry.signals.on_hit_item { threat.on_hit = (threat.on_hit + boost).min(1.0); }
            if entry.signals.ap_item && !entry.signals.ad_item {
                threat.magic_damage = (threat.magic_damage + boost).min(1.0);
                threat.ap_burst = (threat.ap_burst + boost * 0.5).min(1.0);
            }
            if entry.signals.ad_item {
                threat.physical_damage = (threat.physical_damage + boost).min(1.0);
            }
            if entry.signals.tank_item {
                threat.tanky = (threat.tanky + boost).min(1.0);
            }
        }
    }

    if threat.crit > 0.3 {
        threat.autoattack = threat.autoattack.max(threat.crit);
        threat.physical_damage = threat.physical_damage.max(threat.crit * 0.8);
    }

    threat
}

fn choose_counter_item(
    candidates: &[ItemAdvisorEntry],
    buyer: &BuyerProfile,
    threat: &ThreatProfile,
    owned_item_ids: &HashSet<i32>,
    enemy_champion_name: &str,
    game_time: Option<i64>,
) -> Option<CounterItemSuggestion> {
    let mut best: Option<(i32, usize, &ItemAdvisorEntry, String, Option<String>)> = None;

    for item in candidates {
        if owned_item_ids.contains(&item.id) {
            continue;
        }

        let Some((score, reason_weight, counter_reason, build_reason)) =
            score_counter_item(item, buyer, threat, enemy_champion_name, game_time)
        else {
            continue;
        };

        match best {
            Some((best_score, best_reason_weight, best_item, _, _)) => {
                let replace = score > best_score
                    || (score == best_score && reason_weight > best_reason_weight)
                    || (score == best_score
                        && reason_weight == best_reason_weight
                        && item.gold_total < best_item.gold_total);

                if replace {
                    best = Some((score, reason_weight, item, counter_reason, build_reason));
                }
            }
            None => best = Some((score, reason_weight, item, counter_reason, build_reason)),
        }
    }

    let (best_score, _, best_item, counter_reason, build_reason) = best?;
    if best_score < 6 {
        return None;
    }

    Some(CounterItemSuggestion {
        item_id: best_item.id,
        name: best_item.name.clone(),
        build_reason,
        counter_reason,
    })
}

fn score_counter_item(
    item: &ItemAdvisorEntry,
    buyer: &BuyerProfile,
    threat: &ThreatProfile,
    enemy_champion_name: &str,
    game_time: Option<i64>,
) -> Option<(i32, usize, String, Option<String>)> {
    let (fit, build_reason) = buyer_fit_score(buyer, &item.signals);
    if fit <= -6 {
        return None;
    }

    let mut score = fit;
    let mut reason = String::from("ситуативный контр-предмет");
    let mut reason_weight = 0usize;

    let enemy = if enemy_champion_name.is_empty() { "" } else { enemy_champion_name };

    let mut apply_reason = |intensity: f32, base_delta: i32, weight: usize, message: &str| {
        if intensity <= 0.0 {
            return;
        }
        let delta = (base_delta as f32 * intensity).round() as i32;
        score += delta;
        if weight > reason_weight {
            reason_weight = weight;
            if enemy.is_empty() {
                reason = message.to_string();
            } else {
                reason = format!("{} ({})", message, enemy);
            }
        }
    };

    if item.signals.anti_heal {
        apply_reason(threat.healing, 8, 8, "антихил против лечения");
    }
    if item.signals.anti_shield {
        apply_reason(threat.shields, 8, 8, "контр предмет против щитов");
    }
    if item.signals.anti_crit {
        apply_reason(threat.crit, 7, 7, "снижает давление от крит-урона");
    }
    if threat.hard_cc > 0.0 {
        if item.signals.cc_cleanse {
            apply_reason(threat.hard_cc, 8, 8, "снимает опасный контроль");
        } else if item.signals.spell_block {
            apply_reason(threat.hard_cc, 7, 7, "блокирует ключевой контроль");
        } else if item.signals.magic_resist {
            apply_reason(threat.hard_cc, 3, 3, "даёт запас против магии и контроля");
        }
    }
    if threat.ap_burst > 0.0 {
        if item.signals.spell_block {
            apply_reason(threat.ap_burst, 8, 8, "защищает от магического прокаста");
        } else if item.signals.burst_protection {
            apply_reason(threat.ap_burst, 6, 6, "переживает магический прокаст");
        } else if item.signals.magic_resist {
            apply_reason(threat.ap_burst, 5, 5, "даёт магрез против AP-урона");
        }
    }
    if threat.ad_burst > 0.0 {
        if item.signals.burst_protection {
            apply_reason(threat.ad_burst, 6, 6, "помогает пережить burst");
        }
        if item.signals.armor {
            apply_reason(threat.ad_burst, 5, 5, "даёт броню против AD-урона");
        }
    }
    if threat.autoattack > 0.0 {
        if item.signals.anti_dps {
            apply_reason(threat.autoattack, 6, 6, "срезает урон от автоатак");
        } else if item.signals.armor {
            apply_reason(threat.autoattack, 3, 3, "даёт броню против автоатак");
        }
    }
    if threat.on_hit > 0.0 {
        if item.signals.anti_dps {
            apply_reason(threat.on_hit, 5, 5, "мешает on-hit/DPS урону");
        } else if item.signals.armor {
            apply_reason(threat.on_hit, 2, 2, "даёт броню против DPS");
        }
    }
    if item.signals.anti_tank {
        apply_reason(threat.tanky, 7, 7, "лучше пробивает плотную цель");
    }
    if item.signals.magic_resist {
        apply_reason(threat.magic_damage, 2, 2, "добавляет магрез");
    }
    if item.signals.armor {
        apply_reason(threat.physical_damage, 2, 2, "добавляет броню");
    }

    let cost_penalty = match game_time.unwrap_or_default() {
        0..=899 => item.gold_total / 1_300,
        900..=1799 => item.gold_total / 1_800,
        _ => item.gold_total / 2_400,
    };
    score -= cost_penalty;

    if reason_weight == 0 {
        return None;
    }

    Some((score, reason_weight, reason, build_reason))
}

fn buyer_fit_score(buyer: &BuyerProfile, signals: &ItemSignals) -> (i32, Option<String>) {
    let mut score = 0i32;
    let mut label: Option<&str> = None;

    // Marksman: strongly prefers AD, rejects AP/tank
    if buyer.is_marksman {
        if signals.ad_item {
            score += 5;
            label = Some("предмет для стрелка");
        }
        if signals.ap_item && !signals.ad_item {
            score -= 8;
        }
        if signals.tank_item {
            score -= 5;
        }
        let reason = if score > 3 { label.map(String::from) } else { None };
        return (score, reason);
    }

    // Weight-based scoring
    if signals.ap_item {
        let delta = (5.0 * buyer.ap_weight - 6.0 * buyer.ad_weight).round() as i32;
        score += delta;
        if buyer.ap_weight > 0.4 && delta > 0 {
            label = Some("вписывается в AP-билд");
        }
    }

    if signals.ad_item && !signals.ap_item {
        let delta = (5.0 * buyer.ad_weight - 6.0 * buyer.ap_weight).round() as i32;
        score += delta;
        if buyer.ad_weight > 0.4 && delta > 0 {
            label = Some("вписывается в AD-билд");
        }
    }

    if signals.tank_item || signals.armor || signals.magic_resist {
        let delta = (4.0 * buyer.tank_weight).round() as i32;
        score += delta;
        if buyer.tank_weight > 0.4 && delta > 0 && label.is_none() {
            label = Some("подходит танковому билду");
        }
        // Penalize tank items for pure damage dealers
        if buyer.tank_weight < 0.1 && !signals.ad_item && !signals.ap_item {
            score -= 4;
        }
    }

    let reason = if score > 2 { label.map(String::from) } else { None };
    (score, reason)
}

fn item_tags(info: &serde_json::Value) -> Vec<String> {
    info.get("tags")
        .and_then(|tags| tags.as_array())
        .map(|tags| {
            tags.iter()
                .filter_map(|tag| tag.as_str().map(|tag| tag.to_lowercase()))
                .collect()
        })
        .unwrap_or_default()
}

fn item_stats(info: &serde_json::Value) -> HashMap<String, f64> {
    info.get("stats")
        .and_then(|stats| stats.as_object())
        .map(|stats| {
            stats.iter()
                .filter_map(|(key, value)| value.as_f64().map(|value| (key.clone(), value)))
                .collect()
        })
        .unwrap_or_default()
}

fn stats_has(stats: &HashMap<String, f64>, needle: &str) -> bool {
    stats.iter().any(|(key, value)| key.contains(needle) && *value > 0.0)
}

fn normalize_item_text(info: &serde_json::Value, name: &str) -> String {
    let plaintext = info
        .get("plaintext")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let description = info
        .get("description")
        .and_then(|value| value.as_str())
        .unwrap_or_default();

    let combined = format!("{} {} {}", name, plaintext, description);
    strip_html_tags(&combined).to_lowercase()
}

fn champion_text_blob(meta: &ChampionMetaInfo) -> String {
    let mut parts = Vec::new();
    parts.push(meta.passive_name.clone());
    parts.push(meta.passive_desc.clone());
    parts.extend(meta.ally_tips.iter().cloned());
    for ability in &meta.abilities {
        parts.push(ability.name.clone());
        parts.push(ability.short_desc.clone());
    }
    parts.join(" ").to_lowercase()
}

fn count_hits(text: &str, needles: &[&str]) -> usize {
    needles
        .iter()
        .map(|needle| text.matches(needle).count())
        .sum()
}

fn text_has(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn strip_html_tags(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut inside_tag = false;

    for ch in text.chars() {
        match ch {
            '<' => inside_tag = true,
            '>' => inside_tag = false,
            _ if !inside_tag => result.push(ch),
            _ => {}
        }
    }

    result
        .replace("&nbsp;", " ")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_coach::ChampionAbility;

    fn test_item(info: serde_json::Value) -> serde_json::Value {
        info
    }

    fn candidate_entry(id: i32, name: &str, signals: ItemSignals, gold_total: i32) -> ItemAdvisorEntry {
        ItemAdvisorEntry {
            id,
            name: name.to_string(),
            gold_total,
            signals,
        }
    }

    fn champ_meta(tags: &[&str], short_desc: &str) -> ChampionMetaInfo {
        ChampionMetaInfo {
            resource: "Mana".to_string(),
            tags: tags.iter().map(|tag| (*tag).to_string()).collect(),
            abilities: vec![ChampionAbility {
                slot: "Q".to_string(),
                name: "Test".to_string(),
                short_desc: short_desc.to_string(),
            }],
            passive_name: String::new(),
            passive_desc: String::new(),
            ally_tips: vec![],
        }
    }

    fn empty_player() -> lcu::LiveFullPlayer {
        lcu::LiveFullPlayer {
            champion_name: None,
            team: None,
            position: None,
            level: None,
            scores: None,
            items: None,
            summoner_name: None,
            riot_id_game_name: None,
            riot_id_tag_line: None,
            summoner_spells: None,
            is_dead: None,
            respawn_timer: None,
            runes: None,
        }
    }

    fn player_with_items(item_ids: &[i32]) -> lcu::LiveFullPlayer {
        let mut p = empty_player();
        p.items = Some(
            item_ids
                .iter()
                .map(|&id| lcu::LiveItem {
                    item_id: Some(id),
                    display_name: None,
                    count: Some(1),
                    price: None,
                })
                .collect(),
        );
        p
    }

    fn empty_catalog() -> ItemCatalogSnapshot {
        ItemCatalogSnapshot::default()
    }

    fn buyer_from_meta(meta: &ChampionMetaInfo) -> BuyerProfile {
        derive_buyer_profile(&empty_player(), Some(meta), &empty_catalog())
    }

    #[test]
    fn filters_out_boots_and_special_items() {
        let boots = test_item(serde_json::json!({
            "gold": { "total": 1100, "purchasable": true },
            "maps": { "11": true },
            "tags": ["Boots"],
            "from": ["1001"],
            "into": []
        }));
        assert!(!is_counter_item_candidate(&boots));

        let special = test_item(serde_json::json!({
            "gold": { "total": 3200, "purchasable": true },
            "maps": { "11": true },
            "tags": ["Damage"],
            "from": ["3006"],
            "into": [],
            "requiredChampion": "Ornn"
        }));
        assert!(!is_counter_item_candidate(&special));

        let atma_like = test_item(serde_json::json!({
            "gold": { "total": 3000, "purchasable": true },
            "maps": { "11": true },
            "tags": ["Health", "CriticalStrike", "Lane"],
            "into": [],
            "from": []
        }));
        assert!(!is_counter_item_candidate(&atma_like));
    }

    #[test]
    fn detects_anti_heal_from_description() {
        let item = test_item(serde_json::json!({
            "name": "Morellonomicon",
            "description": "Dealing magic damage applies Grievous Wounds to enemies.",
            "plaintext": "Applies anti-heal.",
            "tags": ["SpellDamage"],
            "stats": { "FlatMagicDamageMod": 90.0 }
        }));

        let signals = build_item_signals(&item, "Morellonomicon");
        assert!(signals.anti_heal);
        assert!(signals.ap_item);
    }

    #[test]
    fn mage_prefers_zhonya_into_ad_burst() {
        let buyer = buyer_from_meta(&champ_meta(&["Mage"], "Deals magic damage."));
        let threat = ThreatProfile {
            ad_burst: 0.8,
            physical_damage: 0.7,
            ..ThreatProfile::default()
        };
        let zhonya = candidate_entry(
            3157,
            "Zhonya's Hourglass",
            ItemSignals {
                armor: true,
                burst_protection: true,
                ap_item: true,
                ..ItemSignals::default()
            },
            3250,
        );
        let randuin = candidate_entry(
            3143,
            "Randuin's Omen",
            ItemSignals {
                armor: true,
                anti_crit: true,
                anti_dps: true,
                tank_item: true,
                ..ItemSignals::default()
            },
            3000,
        );

        let suggestion = choose_counter_item(&[zhonya, randuin], &buyer, &threat, &HashSet::new(), "Zed", Some(1200))
            .expect("expected a recommendation");
        assert_eq!(suggestion.item_id, 3157);
    }

    #[test]
    fn returns_none_when_signal_is_too_weak() {
        let buyer = buyer_from_meta(&champ_meta(&["Mage"], "Deals magic damage."));
        let threat = ThreatProfile::default();
        let generic_ap = candidate_entry(
            3089,
            "Rabadon's Deathcap",
            ItemSignals {
                ap_item: true,
                ..ItemSignals::default()
            },
            3600,
        );

        assert!(choose_counter_item(&[generic_ap], &buyer, &threat, &HashSet::new(), "", Some(1200)).is_none());
    }

    #[test]
    fn allows_reasonable_general_counter_instead_of_question_mark() {
        let buyer = buyer_from_meta(&champ_meta(&["Mage"], "Deals magic damage."));
        let threat = ThreatProfile {
            magic_damage: 0.6,
            ..ThreatProfile::default()
        };
        let banshee = candidate_entry(
            3102,
            "Banshee's Veil",
            ItemSignals {
                ap_item: true,
                magic_resist: true,
                spell_block: true,
                burst_protection: true,
                ..ItemSignals::default()
            },
            3100,
        );

        let suggestion = choose_counter_item(&[banshee], &buyer, &threat, &HashSet::new(), "", Some(1200))
            .expect("expected a softer fallback recommendation");
        assert_eq!(suggestion.item_id, 3102);
    }

    #[test]
    fn buyer_profile_from_items_overrides_meta() {
        // AP champion who bought AD items should get AD-weighted profile
        let mut catalog = empty_catalog();
        let ad_entry = ItemAdvisorEntry {
            id: 3031,
            name: "Infinity Edge".to_string(),
            gold_total: 3400,
            signals: ItemSignals { ad_item: true, crit_item: true, ..ItemSignals::default() },
        };
        catalog.items_by_id.insert(3031, ad_entry);
        let ad_entry2 = ItemAdvisorEntry {
            id: 3036,
            name: "Lord Dominik's".to_string(),
            gold_total: 3000,
            signals: ItemSignals { ad_item: true, anti_tank: true, ..ItemSignals::default() },
        };
        catalog.items_by_id.insert(3036, ad_entry2);

        let player = player_with_items(&[3031, 3036]);
        let meta = champ_meta(&["Mage"], "Deals magic damage.");
        let buyer = derive_buyer_profile(&player, Some(&meta), &catalog);

        // Even though meta says Mage, items say AD
        assert!(buyer.ad_weight > 0.5, "ad_weight should be high: {}", buyer.ad_weight);
        assert!(buyer.ap_weight < 0.1, "ap_weight should be low: {}", buyer.ap_weight);
    }

    #[test]
    fn threat_intensity_scales_with_items() {
        let meta = champ_meta(&["Marksman"], "Deals physical damage with basic attacks and critical strikes.");
        let mut catalog = empty_catalog();
        let crit1 = ItemAdvisorEntry {
            id: 3031,
            name: "IE".to_string(),
            gold_total: 3400,
            signals: ItemSignals { ad_item: true, crit_item: true, ..ItemSignals::default() },
        };
        let crit2 = ItemAdvisorEntry {
            id: 3094,
            name: "RFC".to_string(),
            gold_total: 2800,
            signals: ItemSignals { attack_speed_item: true, crit_item: true, ..ItemSignals::default() },
        };
        catalog.items_by_id.insert(3031, crit1);
        catalog.items_by_id.insert(3094, crit2);

        // With 0 items
        let player_no_items = empty_player();
        let threat_low = derive_enemy_threat(&player_no_items, Some(&meta), &catalog);

        // With 2 crit items
        let player_crit = player_with_items(&[3031, 3094]);
        let threat_high = derive_enemy_threat(&player_crit, Some(&meta), &catalog);

        assert!(threat_high.crit > threat_low.crit,
            "crit with items ({}) should be > without ({})", threat_high.crit, threat_low.crit);
        assert!(threat_high.autoattack > threat_low.autoattack,
            "autoattack with items ({}) should be > without ({})", threat_high.autoattack, threat_low.autoattack);
    }

    #[test]
    fn suggestion_has_both_reasons() {
        let buyer = BuyerProfile {
            ap_weight: 0.8,
            ad_weight: 0.0,
            tank_weight: 0.0,
            is_marksman: false,
        };
        let threat = ThreatProfile {
            healing: 0.9,
            ..ThreatProfile::default()
        };
        let morello = candidate_entry(
            3165,
            "Morellonomicon",
            ItemSignals {
                ap_item: true,
                anti_heal: true,
                ..ItemSignals::default()
            },
            3000,
        );

        let suggestion = choose_counter_item(&[morello], &buyer, &threat, &HashSet::new(), "Aatrox", Some(1200))
            .expect("expected a recommendation");
        assert!(suggestion.counter_reason.contains("Aatrox"),
            "counter_reason should mention enemy name: {}", suggestion.counter_reason);
        assert!(suggestion.build_reason.is_some(),
            "build_reason should be set for matching AP item with AP buyer");
    }
}
