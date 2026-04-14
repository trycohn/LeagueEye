use std::collections::HashMap;

const DDRAGON_VERSION: &str = "16.7.1";
const CONCURRENT_FETCHES: usize = 20;

#[derive(Clone, Debug)]
pub struct ChampionAbilityInfo {
    pub slot: String,       // "Passive", "Q", "W", "E", "R"
    pub en_name: String,
    pub ru_name: String,
    pub short_desc: String, // cleaned HTML, truncated to ~120 chars
}

#[derive(Clone, Debug)]
pub struct CatalogChampion {
    pub internal_name: String,  // "Ahri", "MissFortune"
    pub en_name: String,        // "Ahri", "Miss Fortune"
    pub ru_name: String,        // "Ари", "Мисс Фортуна"
    pub resource: String,       // "Mana", "Energy", "None"
    pub tags: Vec<String>,      // ["Mage", "Assassin"]
    pub abilities: Vec<ChampionAbilityInfo>,
    pub ally_tips: Vec<String>, // first 3 Riot tips (RU if available, else EN)
}

pub struct ChampionCatalog {
    /// All champions keyed by internal_name
    pub champions: HashMap<String, CatalogChampion>,
    /// EN display_name → RU display_name (for user message replacement)
    pub en_to_ru: HashMap<String, String>,
    /// internal_name → RU display_name
    pub internal_to_ru: HashMap<String, String>,
}

pub async fn load_champion_catalog() -> Result<ChampionCatalog, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // Step 1: Load champion list (en_US + ru_RU) for names, resource, tags
    let url_list_en = format!(
        "https://ddragon.leagueoflegends.com/cdn/{}/data/en_US/champion.json",
        DDRAGON_VERSION
    );
    let url_list_ru = format!(
        "https://ddragon.leagueoflegends.com/cdn/{}/data/ru_RU/champion.json",
        DDRAGON_VERSION
    );

    let (en_list_result, ru_list_result) = tokio::join!(
        fetch_json(&client, &url_list_en),
        fetch_json(&client, &url_list_ru),
    );

    let en_list = match en_list_result {
        Ok(data) => data,
        Err(e) => {
            log::error!("[champion_catalog] Failed to load EN champion list: {}", e);
            return Ok(empty_catalog());
        }
    };

    let ru_list = match ru_list_result {
        Ok(data) => Some(data),
        Err(e) => {
            log::warn!("[champion_catalog] Failed to load RU champion list, using EN names: {}", e);
            None
        }
    };

    // Parse champion list to get internal names and basic info
    let en_data = match en_list.get("data").and_then(|d| d.as_object()) {
        Some(data) => data,
        None => return Ok(empty_catalog()),
    };

    let ru_names: HashMap<String, String> = ru_list
        .as_ref()
        .and_then(|j| j.get("data"))
        .and_then(|d| d.as_object())
        .map(|data| {
            data.iter()
                .filter_map(|(internal_id, info)| {
                    let name = info.get("name")?.as_str()?;
                    Some((internal_id.clone(), name.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();

    // Collect internal names and basic info from list endpoint
    let mut champions: HashMap<String, CatalogChampion> = HashMap::new();
    let mut internal_names: Vec<String> = Vec::new();

    for (internal_name, info) in en_data {
        let en_name = match info.get("name").and_then(|n| n.as_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        let resource = info
            .get("partype")
            .and_then(|r| r.as_str())
            .unwrap_or("None")
            .to_string();

        let tags: Vec<String> = info
            .get("tags")
            .and_then(|t| t.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let ru_name = ru_names
            .get(internal_name)
            .cloned()
            .unwrap_or_else(|| en_name.clone());

        champions.insert(
            internal_name.clone(),
            CatalogChampion {
                internal_name: internal_name.clone(),
                en_name,
                ru_name,
                resource,
                tags,
                abilities: Vec::new(),
                ally_tips: Vec::new(),
            },
        );
        internal_names.push(internal_name.clone());
    }

    // Step 2: Load detailed champion data (abilities, passive, tips) — batched
    internal_names.sort(); // deterministic order

    for chunk in internal_names.chunks(CONCURRENT_FETCHES) {
        let futures: Vec<_> = chunk
            .iter()
            .map(|name| {
                let client = client.clone();
                let name = name.clone();
                async move {
                    let url_en = format!(
                        "https://ddragon.leagueoflegends.com/cdn/{}/data/en_US/champion/{}.json",
                        DDRAGON_VERSION, name
                    );
                    let url_ru = format!(
                        "https://ddragon.leagueoflegends.com/cdn/{}/data/ru_RU/champion/{}.json",
                        DDRAGON_VERSION, name
                    );
                    let (en_res, ru_res) = tokio::join!(
                        fetch_json(&client, &url_en),
                        fetch_json(&client, &url_ru),
                    );
                    (name, en_res, ru_res)
                }
            })
            .collect();

        let results = futures::future::join_all(futures).await;

        for (name, en_res, ru_res) in results {
            let en_detail = match en_res {
                Ok(data) => data,
                Err(e) => {
                    log::warn!("[champion_catalog] Failed to load detail for {}: {}", name, e);
                    continue;
                }
            };
            let ru_detail = ru_res.ok();

            let en_champ = en_detail
                .get("data")
                .and_then(|d| d.get(&name))
                .unwrap_or(&en_detail);

            let ru_champ = ru_detail
                .as_ref()
                .and_then(|j| j.get("data"))
                .and_then(|d| d.get(&name));

            // Parse abilities
            let abilities = parse_abilities(en_champ, ru_champ);

            // Parse ally tips (prefer RU)
            let ally_tips = parse_ally_tips(ru_champ.unwrap_or(en_champ));

            if let Some(champ) = champions.get_mut(&name) {
                champ.abilities = abilities;
                champ.ally_tips = ally_tips;
            }
        }
    }

    // Build lookup maps
    let mut en_to_ru = HashMap::new();
    let mut internal_to_ru = HashMap::new();

    for champ in champions.values() {
        en_to_ru.insert(champ.en_name.clone(), champ.ru_name.clone());
        internal_to_ru.insert(champ.internal_name.clone(), champ.ru_name.clone());
    }

    let loaded_with_abilities = champions.values().filter(|c| !c.abilities.is_empty()).count();
    log::info!(
        "[champion_catalog] Loaded {} champions ({} with abilities), {} name mappings (DDragon {})",
        champions.len(),
        loaded_with_abilities,
        en_to_ru.len(),
        DDRAGON_VERSION
    );

    Ok(ChampionCatalog {
        champions,
        en_to_ru,
        internal_to_ru,
    })
}

fn parse_abilities(
    en_champ: &serde_json::Value,
    ru_champ: Option<&serde_json::Value>,
) -> Vec<ChampionAbilityInfo> {
    let mut abilities = Vec::new();

    // Passive
    let en_passive = en_champ.get("passive");
    let ru_passive = ru_champ.and_then(|c| c.get("passive"));

    if let Some(p) = en_passive {
        let en_name = p
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();
        let ru_name = ru_passive
            .and_then(|rp| rp.get("name"))
            .and_then(|n| n.as_str())
            .map(String::from)
            .unwrap_or_else(|| en_name.clone());
        let desc = ru_passive
            .and_then(|rp| rp.get("description"))
            .or_else(|| p.get("description"))
            .and_then(|d| d.as_str())
            .map(|s| strip_html_tags(s))
            .unwrap_or_default();

        abilities.push(ChampionAbilityInfo {
            slot: "Passive".to_string(),
            en_name,
            ru_name,
            short_desc: truncate_desc(&desc, 120),
        });
    }

    // Q, W, E, R
    let en_spells = en_champ.get("spells").and_then(|s| s.as_array());
    let ru_spells = ru_champ.and_then(|c| c.get("spells")).and_then(|s| s.as_array());

    let slots = ["Q", "W", "E", "R"];
    if let Some(spells) = en_spells {
        for (i, spell) in spells.iter().take(4).enumerate() {
            let slot = slots.get(i).unwrap_or(&"?").to_string();
            let en_name = spell
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string();
            let ru_spell = ru_spells.and_then(|arr| arr.get(i));
            let ru_name = ru_spell
                .and_then(|s| s.get("name"))
                .and_then(|n| n.as_str())
                .map(String::from)
                .unwrap_or_else(|| en_name.clone());
            let desc = ru_spell
                .and_then(|s| s.get("description"))
                .or_else(|| spell.get("description"))
                .and_then(|d| d.as_str())
                .map(|s| strip_html_tags(s))
                .unwrap_or_default();

            abilities.push(ChampionAbilityInfo {
                slot,
                en_name,
                ru_name,
                short_desc: truncate_desc(&desc, 120),
            });
        }
    }

    abilities
}

fn parse_ally_tips(champ: &serde_json::Value) -> Vec<String> {
    champ
        .get("allytips")
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| strip_html_tags(s)))
                .take(3)
                .collect()
        })
        .unwrap_or_default()
}

fn empty_catalog() -> ChampionCatalog {
    ChampionCatalog {
        champions: HashMap::new(),
        en_to_ru: HashMap::new(),
        internal_to_ru: HashMap::new(),
    }
}

async fn fetch_json(
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

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("JSON parse failed: {}", e))?;

    Ok(json)
}

/// Strip HTML tags from DDragon descriptions
fn strip_html_tags(s: &str) -> String {
    let cleaned = s.replace("<br>", " ").replace("<br/>", " ").replace("<br />", " ");
    let mut result = String::new();
    let mut inside_tag = false;
    for ch in cleaned.chars() {
        if ch == '<' {
            inside_tag = true;
        } else if ch == '>' {
            inside_tag = false;
        } else if !inside_tag {
            result.push(ch);
        }
    }
    // Collapse multiple spaces
    let mut prev_space = false;
    let collapsed: String = result
        .chars()
        .filter(|&c| {
            if c == ' ' {
                if prev_space {
                    return false;
                }
                prev_space = true;
            } else {
                prev_space = false;
            }
            true
        })
        .collect();
    collapsed.trim().to_string()
}

fn truncate_desc(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        return s.to_string();
    }

    let prefix: String = s.chars().take(max_len).collect();

    // Try to cut at sentence boundary
    let end = prefix
        .rfind(". ")
        .map(|pos| pos + 1)
        .unwrap_or(prefix.len());
    prefix[..end].to_string()
}

/// Format resource for display in catalog
pub fn format_resource_ru(resource: &str) -> &str {
    match resource {
        "None" => "Без ресурса",
        "Mana" => "Мана",
        "Energy" => "Энергия",
        "Fury" | "Rage" => "Ярость",
        "Shield" => "Щит",
        "Heat" => "Нагрев",
        "Flow" => "Поток",
        "Courage" => "Храбрость",
        "Blood Well" => "Без ресурса",
        "Ferocity" => "Свирепость",
        "Grit" => "Стойкость",
        "Crimson Rush" => "Без ресурса",
        "None (Costs Health)" => "Тратит HP",
        _ => resource,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_champion_detail(
        passive_name: &str,
        passive_desc: &str,
        spells: Vec<(&str, &str)>,
        ally_tips: Vec<&str>,
    ) -> serde_json::Value {
        let spells_json: Vec<serde_json::Value> = spells
            .into_iter()
            .map(|(name, desc)| serde_json::json!({ "name": name, "description": desc }))
            .collect();
        serde_json::json!({
            "passive": {
                "name": passive_name,
                "description": passive_desc,
            },
            "spells": spells_json,
            "allytips": ally_tips,
        })
    }

    #[test]
    fn strip_html_tags_works() {
        assert_eq!(
            strip_html_tags("<mainText>Hello <br>World</mainText>"),
            "Hello World"
        );
        assert_eq!(strip_html_tags("No tags here"), "No tags here");
        assert_eq!(strip_html_tags("<b>Bold</b> and <i>italic</i>"), "Bold and italic");
    }

    #[test]
    fn truncate_desc_short_string_unchanged() {
        assert_eq!(truncate_desc("Short desc.", 120), "Short desc.");
    }

    #[test]
    fn truncate_desc_cuts_at_sentence() {
        let long = "First sentence. Second sentence that is quite long and would exceed the limit if we kept going.";
        let result = truncate_desc(long, 40);
        assert_eq!(result, "First sentence.");
    }

    #[test]
    fn truncate_desc_cuts_at_max_len_if_no_sentence() {
        let long = "A very long description without any period boundaries that just keeps going and going forever";
        let result = truncate_desc(long, 30);
        assert_eq!(result.chars().count(), 30);
    }

    #[test]
    fn truncate_desc_handles_cyrillic_utf8_boundary() {
        let long = format!("{}{}{}",
            "н ".repeat(39),
            "abн",
            "a".repeat(40)
        );

        let result = truncate_desc(&long, 120);

        assert_eq!(result.chars().count(), 120);
        assert_eq!(result, format!("{}{}{}", "н ".repeat(39), "abн", "a".repeat(39)));
    }

    #[test]
    fn parse_abilities_from_detail() {
        let en_detail = make_champion_detail(
            "Essence Theft",
            "Ahri gains <b>charges</b> when hitting enemies",
            vec![
                ("Orb of Deception", "Ahri throws and pulls back her orb"),
                ("Fox-Fire", "Ahri releases three fox-fires"),
                ("Charm", "Ahri blows a kiss"),
                ("Spirit Rush", "Ahri dashes forward"),
            ],
            vec!["Use Charm to catch enemies"],
        );

        let ru_detail = make_champion_detail(
            "\u{0411}\u{043b}\u{0430}\u{0433}\u{043e}\u{0434}\u{0430}\u{0442}\u{044c}",
            "\u{0410}\u{0440}\u{0438} \u{043f}\u{043e}\u{043b}\u{0443}\u{0447}\u{0430}\u{0435}\u{0442} \u{0437}\u{0430}\u{0440}\u{044f}\u{0434}\u{044b}",
            vec![
                ("\u{0421}\u{0444}\u{0435}\u{0440}\u{0430} \u{043e}\u{0431}\u{043c}\u{0430}\u{043d}\u{0430}", "\u{0410}\u{0440}\u{0438} \u{0431}\u{0440}\u{043e}\u{0441}\u{0430}\u{0435}\u{0442} \u{0441}\u{0444}\u{0435}\u{0440}\u{0443}"),
                ("\u{041b}\u{0438}\u{0441}\u{0438}\u{0439} \u{043e}\u{0433}\u{043e}\u{043d}\u{044c}", "\u{0410}\u{0440}\u{0438} \u{0432}\u{044b}\u{043f}\u{0443}\u{0441}\u{043a}\u{0430}\u{0435}\u{0442} \u{043e}\u{0433}\u{043d}\u{0438}"),
                ("\u{041e}\u{0447}\u{0430}\u{0440}\u{043e}\u{0432}\u{0430}\u{043d}\u{0438}\u{0435}", "\u{0410}\u{0440}\u{0438} \u{043e}\u{0447}\u{0430}\u{0440}\u{043e}\u{0432}\u{044b}\u{0432}\u{0430}\u{0435}\u{0442}"),
                ("\u{041f}\u{043e}\u{0440}\u{044b}\u{0432} \u{0434}\u{0443}\u{0445}\u{0430}", "\u{0410}\u{0440}\u{0438} \u{0434}\u{0435}\u{043b}\u{0430}\u{0435}\u{0442} \u{0440}\u{044b}\u{0432}\u{043e}\u{043a}"),
            ],
            vec!["\u{0418}\u{0441}\u{043f}\u{043e}\u{043b}\u{044c}\u{0437}\u{0443}\u{0439} \u{041e}\u{0447}\u{0430}\u{0440}\u{043e}\u{0432}\u{0430}\u{043d}\u{0438}\u{0435}"],
        );

        let abilities = parse_abilities(&en_detail, Some(&ru_detail));

        assert_eq!(abilities.len(), 5); // Passive + Q W E R
        assert_eq!(abilities[0].slot, "Passive");
        assert_eq!(abilities[0].en_name, "Essence Theft");
        assert_eq!(abilities[0].ru_name, "\u{0411}\u{043b}\u{0430}\u{0433}\u{043e}\u{0434}\u{0430}\u{0442}\u{044c}");
        assert_eq!(abilities[1].slot, "Q");
        assert_eq!(abilities[1].en_name, "Orb of Deception");
        assert_eq!(abilities[1].ru_name, "\u{0421}\u{0444}\u{0435}\u{0440}\u{0430} \u{043e}\u{0431}\u{043c}\u{0430}\u{043d}\u{0430}");
        assert_eq!(abilities[4].slot, "R");
    }

    #[test]
    fn parse_abilities_without_ru_uses_en() {
        let en_detail = make_champion_detail(
            "Passive Name",
            "Passive desc",
            vec![
                ("Q Spell", "Q desc"),
                ("W Spell", "W desc"),
                ("E Spell", "E desc"),
                ("R Spell", "R desc"),
            ],
            vec![],
        );

        let abilities = parse_abilities(&en_detail, None);

        assert_eq!(abilities.len(), 5);
        assert_eq!(abilities[0].ru_name, "Passive Name");
        assert_eq!(abilities[1].ru_name, "Q Spell");
    }

    #[test]
    fn parse_ally_tips_takes_first_three() {
        let champ = serde_json::json!({
            "allytips": ["Tip 1", "Tip 2", "Tip 3", "Tip 4", "Tip 5"],
        });

        let tips = parse_ally_tips(&champ);

        assert_eq!(tips.len(), 3);
        assert_eq!(tips[0], "Tip 1");
        assert_eq!(tips[2], "Tip 3");
    }

    #[test]
    fn parse_ally_tips_empty_when_missing() {
        let champ = serde_json::json!({});
        let tips = parse_ally_tips(&champ);
        assert!(tips.is_empty());
    }

    #[test]
    fn format_resource_ru_known_types() {
        assert_eq!(format_resource_ru("Mana"), "Мана");
        assert_eq!(format_resource_ru("Energy"), "Энергия");
        assert_eq!(format_resource_ru("None"), "Без ресурса");
        assert_eq!(format_resource_ru("Fury"), "Ярость");
        assert_eq!(format_resource_ru("Heat"), "Нагрев");
    }

    #[test]
    fn format_resource_ru_unknown_passthrough() {
        assert_eq!(format_resource_ru("SomethingNew"), "SomethingNew");
    }

    #[test]
    fn empty_catalog_has_empty_maps() {
        let catalog = empty_catalog();
        assert!(catalog.champions.is_empty());
        assert!(catalog.en_to_ru.is_empty());
        assert!(catalog.internal_to_ru.is_empty());
    }
}
