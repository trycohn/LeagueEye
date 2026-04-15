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
    pub reason: String,
}

#[derive(Clone, Debug, Default)]
struct BuyerProfile {
    ap: bool,
    ad: bool,
    tank: bool,
    marksman: bool,
    fighter: bool,
}

#[derive(Clone, Debug, Default)]
struct ThreatProfile {
    healing: bool,
    shields: bool,
    ap_burst: bool,
    ad_burst: bool,
    autoattack: bool,
    crit: bool,
    on_hit: bool,
    hard_cc: bool,
    tanky: bool,
    magic_damage: bool,
    physical_damage: bool,
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
    game_time: Option<i64>,
) -> Option<CounterItemSuggestion> {
    if catalog.counter_candidates.is_empty() {
        return None;
    }

    let buyer = derive_buyer_profile(my_meta);
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

fn derive_buyer_profile(meta: Option<&ChampionMetaInfo>) -> BuyerProfile {
    let Some(meta) = meta else {
        return BuyerProfile {
            ad: true,
            fighter: true,
            ..BuyerProfile::default()
        };
    };

    let tags: HashSet<&str> = meta.tags.iter().map(String::as_str).collect();
    let text = champion_text_blob(meta);
    let magic_hits = count_hits(&text, &["magic damage", "ability power"]);
    let physical_hits = count_hits(&text, &["physical damage", "attack damage"]);

    let marksman = tags.contains("Marksman");
    let fighter = tags.contains("Fighter");
    let tank = tags.contains("Tank");
    let mage = tags.contains("Mage");
    let assassin = tags.contains("Assassin");

    let ap = mage || magic_hits > physical_hits + 1 || (assassin && magic_hits > physical_hits);
    let mut ad = marksman
        || physical_hits > magic_hits
        || (fighter && !ap)
        || (assassin && !ap);

    if !ap && !ad && !tank {
        ad = true;
    }

    BuyerProfile {
        ap,
        ad,
        tank,
        marksman,
        fighter,
    }
}

fn derive_enemy_threat(
    enemy_player: &lcu::LiveFullPlayer,
    enemy_meta: Option<&ChampionMetaInfo>,
    catalog: &ItemCatalogSnapshot,
) -> ThreatProfile {
    let mut threat = ThreatProfile::default();

    if let Some(meta) = enemy_meta {
        let tags: HashSet<&str> = meta.tags.iter().map(String::as_str).collect();
        let text = champion_text_blob(meta);
        let magic_hits = count_hits(&text, &["magic damage"]);
        let physical_hits = count_hits(&text, &["physical damage"]);
        let magic_pref = magic_hits > physical_hits;
        let physical_pref = physical_hits > magic_hits;

        threat.healing = text_has(
            &text,
            &[
                "heal",
                "heals",
                "healing",
                "restore health",
                "lifesteal",
                "life steal",
                "omnivamp",
                "vamp",
            ],
        );
        threat.shields = text_has(&text, &["shield", "shields"]);
        threat.hard_cc = text_has(
            &text,
            &[
                "stun",
                "root",
                "snare",
                "taunt",
                "fear",
                "charm",
                "suppress",
                "airborne",
                "knock up",
                "sleep",
                "polymorph",
            ],
        );
        threat.autoattack = tags.contains("Marksman")
            || count_hits(&text, &["basic attack", "critical strike", "attack speed"]) > 0;
        threat.crit = count_hits(&text, &["critical strike"]) > 0;
        threat.on_hit = count_hits(&text, &["on-hit", "on hit"]) > 0;
        threat.tanky = tags.contains("Tank") || count_hits(&text, &["max health", "bonus health"]) > 0;
        threat.magic_damage = tags.contains("Mage") || magic_pref;
        threat.physical_damage = tags.contains("Marksman") || tags.contains("Fighter") || physical_pref;
        threat.ap_burst = tags.contains("Mage") || (tags.contains("Assassin") && magic_pref);
        threat.ad_burst = tags.contains("Marksman") || tags.contains("Fighter") || (tags.contains("Assassin") && !magic_pref);
    }

    if let Some(items) = enemy_player.items.as_ref() {
        for item in items {
            let Some(item_id) = item.item_id else { continue };
            let Some(entry) = catalog.items_by_id.get(&item_id) else { continue };

            threat.healing |= entry.signals.healing_item;
            threat.crit |= entry.signals.crit_item;
            threat.autoattack |= entry.signals.attack_speed_item || entry.signals.crit_item;
            threat.on_hit |= entry.signals.on_hit_item;
            threat.magic_damage |= entry.signals.ap_item && !entry.signals.ad_item;
            threat.physical_damage |= entry.signals.ad_item;
            threat.tanky |= entry.signals.tank_item;
        }
    }

    if threat.crit {
        threat.autoattack = true;
        threat.physical_damage = true;
    }

    threat
}

fn choose_counter_item(
    candidates: &[ItemAdvisorEntry],
    buyer: &BuyerProfile,
    threat: &ThreatProfile,
    owned_item_ids: &HashSet<i32>,
    game_time: Option<i64>,
) -> Option<CounterItemSuggestion> {
    let mut best: Option<(i32, usize, &ItemAdvisorEntry, String)> = None;

    for item in candidates {
        if owned_item_ids.contains(&item.id) {
            continue;
        }

        let Some((score, reason_weight, reason)) = score_counter_item(item, buyer, threat, game_time) else {
            continue;
        };

        match best {
            Some((best_score, best_reason_weight, best_item, _)) => {
                let replace = score > best_score
                    || (score == best_score && reason_weight > best_reason_weight)
                    || (score == best_score
                        && reason_weight == best_reason_weight
                        && item.gold_total < best_item.gold_total);

                if replace {
                    best = Some((score, reason_weight, item, reason));
                }
            }
            None => best = Some((score, reason_weight, item, reason)),
        }
    }

    let (best_score, _, best_item, reason) = best?;
    if best_score < 6 {
        return None;
    }

    Some(CounterItemSuggestion {
        item_id: best_item.id,
        name: best_item.name.clone(),
        reason,
    })
}

fn score_counter_item(
    item: &ItemAdvisorEntry,
    buyer: &BuyerProfile,
    threat: &ThreatProfile,
    game_time: Option<i64>,
) -> Option<(i32, usize, String)> {
    let fit = buyer_fit_score(buyer, &item.signals);
    if fit <= -6 {
        return None;
    }

    let mut score = fit;
    let mut reason = String::from("ситуативный контр-предмет");
    let mut reason_weight = 0usize;

    let mut apply_reason = |delta: i32, weight: usize, message: &str| {
        score += delta;
        if weight > reason_weight {
            reason_weight = weight;
            reason = message.to_string();
        }
    };

    if threat.healing && item.signals.anti_heal {
        apply_reason(8, 8, "антихил против лечения");
    }
    if threat.shields && item.signals.anti_shield {
        apply_reason(8, 8, "контр предмет против щитов");
    }
    if threat.crit && item.signals.anti_crit {
        apply_reason(7, 7, "снижает давление от крит-урона");
    }
    if threat.hard_cc {
        if item.signals.cc_cleanse {
            apply_reason(8, 8, "снимает опасный контроль");
        } else if item.signals.spell_block {
            apply_reason(7, 7, "блокирует ключевой контроль");
        } else if item.signals.magic_resist {
            apply_reason(3, 3, "даёт запас против магии и контроля");
        }
    }
    if threat.ap_burst {
        if item.signals.spell_block {
            apply_reason(8, 8, "защищает от магического прокаста");
        } else if item.signals.burst_protection {
            apply_reason(6, 6, "переживает магический прокаст");
        } else if item.signals.magic_resist {
            apply_reason(5, 5, "даёт магрез против AP-урона");
        }
    }
    if threat.ad_burst {
        if item.signals.burst_protection {
            apply_reason(6, 6, "помогает пережить burst");
        }
        if item.signals.armor {
            apply_reason(5, 5, "даёт броню против AD-урона");
        }
    }
    if threat.autoattack {
        if item.signals.anti_dps {
            apply_reason(6, 6, "срезает урон от автоатак");
        } else if item.signals.armor {
            apply_reason(3, 3, "даёт броню против автоатак");
        }
    }
    if threat.on_hit {
        if item.signals.anti_dps {
            apply_reason(5, 5, "мешает on-hit/DPS урону");
        } else if item.signals.armor {
            apply_reason(2, 2, "даёт броню против DPS");
        }
    }
    if threat.tanky && item.signals.anti_tank {
        apply_reason(7, 7, "лучше пробивает плотную цель");
    }
    if threat.magic_damage && item.signals.magic_resist {
        apply_reason(2, 2, "добавляет магрез");
    }
    if threat.physical_damage && item.signals.armor {
        apply_reason(2, 2, "добавляет броню");
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

    Some((score, reason_weight, reason))
}

fn buyer_fit_score(buyer: &BuyerProfile, signals: &ItemSignals) -> i32 {
    let mut score = 0;

    if buyer.marksman {
        if signals.ad_item {
            score += 5;
        }
        if signals.ap_item && !signals.ad_item {
            score -= 8;
        }
        if signals.tank_item {
            score -= 5;
        }
        return score;
    }

    if buyer.ap && !buyer.fighter {
        if signals.ap_item {
            score += 5;
        }
        if signals.ad_item && !signals.ap_item {
            score -= 8;
        }
        if signals.tank_item && !signals.ap_item {
            score -= 5;
        }
    }

    if buyer.fighter {
        if signals.ad_item {
            score += 4;
        }
        if signals.armor || signals.magic_resist || signals.tank_item {
            score += 2;
        }
        if signals.ap_item && !signals.ad_item {
            score -= 7;
        }
    }

    if buyer.tank {
        if signals.tank_item || signals.armor || signals.magic_resist || signals.mixed_defense {
            score += 5;
        }
        if signals.ad_item && !signals.tank_item {
            score -= 5;
        }
        if signals.ap_item && !signals.tank_item {
            score -= 5;
        }
    }

    if !buyer.ap && !buyer.ad && !buyer.tank {
        if signals.ad_item {
            score += 2;
        }
    }

    score
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
        let buyer = derive_buyer_profile(Some(&champ_meta(&["Mage"], "Deals magic damage.")));
        let threat = ThreatProfile {
            ad_burst: true,
            physical_damage: true,
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

        let suggestion = choose_counter_item(&[zhonya, randuin], &buyer, &threat, &HashSet::new(), Some(1200))
            .expect("expected a recommendation");
        assert_eq!(suggestion.item_id, 3157);
    }

    #[test]
    fn returns_none_when_signal_is_too_weak() {
        let buyer = derive_buyer_profile(Some(&champ_meta(&["Mage"], "Deals magic damage.")));
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

        assert!(choose_counter_item(&[generic_ap], &buyer, &threat, &HashSet::new(), Some(1200)).is_none());
    }

    #[test]
    fn allows_reasonable_general_counter_instead_of_question_mark() {
        let buyer = derive_buyer_profile(Some(&champ_meta(&["Mage"], "Deals magic damage.")));
        let threat = ThreatProfile {
            magic_damage: true,
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

        let suggestion = choose_counter_item(&[banshee], &buyer, &threat, &HashSet::new(), Some(1200))
            .expect("expected a softer fallback recommendation");
        assert_eq!(suggestion.item_id, 3102);
    }
}
