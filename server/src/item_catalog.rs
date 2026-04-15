use std::collections::HashMap;

const DDRAGON_VERSION: &str = "16.7.1";

pub struct CatalogItem {
    pub id: i32,
    pub en_name: String,
    pub ru_name: String,
    pub gold_total: i32,
    pub tags: Vec<String>,
}

pub struct ItemCatalog {
    /// Final items for AI catalog (system prompt)
    pub items: Vec<CatalogItem>,
    /// EN display_name -> RU name (all items including components, for user message replacement)
    pub en_to_ru: HashMap<String, String>,
    /// EN display_name -> gold total
    pub en_to_gold: HashMap<String, i32>,
    /// EN display_name -> tags
    pub en_to_tags: HashMap<String, Vec<String>>,
}

pub async fn load_item_catalog() -> Result<ItemCatalog, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let url_en = format!(
        "https://ddragon.leagueoflegends.com/cdn/{}/data/en_US/item.json",
        DDRAGON_VERSION
    );
    let url_ru = format!(
        "https://ddragon.leagueoflegends.com/cdn/{}/data/ru_RU/item.json",
        DDRAGON_VERSION
    );

    // Fetch both locales concurrently
    let (en_result, ru_result) = tokio::join!(
        fetch_item_json(&client, &url_en),
        fetch_item_json(&client, &url_ru),
    );

    let en_data = match en_result {
        Ok(data) => data,
        Err(e) => {
            log::error!("[item_catalog] Failed to load EN item.json: {}", e);
            return Ok(empty_catalog());
        }
    };

    // RU data is optional — fallback to EN names if unavailable
    let ru_data = match ru_result {
        Ok(data) => Some(data),
        Err(e) => {
            log::warn!("[item_catalog] Failed to load RU item.json, using EN names: {}", e);
            None
        }
    };

    Ok(build_catalog(en_data, ru_data))
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

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("JSON parse failed: {}", e))?;

    Ok(json)
}

fn empty_catalog() -> ItemCatalog {
    ItemCatalog {
        items: Vec::new(),
        en_to_ru: HashMap::new(),
        en_to_gold: HashMap::new(),
        en_to_tags: HashMap::new(),
    }
}

fn build_catalog(en_json: serde_json::Value, ru_json: Option<serde_json::Value>) -> ItemCatalog {
    let en_data = match en_json.get("data").and_then(|d| d.as_object()) {
        Some(data) => data,
        None => return empty_catalog(),
    };

    // Build RU name lookup: item_id -> ru_name
    let ru_names: HashMap<String, String> = ru_json
        .as_ref()
        .and_then(|j| j.get("data"))
        .and_then(|d| d.as_object())
        .map(|data| {
            data.iter()
                .filter_map(|(id, info)| {
                    let name = info.get("name")?.as_str()?;
                    Some((id.clone(), name.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();

    let mut en_to_ru = HashMap::new();
    let mut en_to_gold = HashMap::new();
    let mut en_to_tags = HashMap::new();
    let mut items = Vec::new();

    for (id_str, info) in en_data {
        let id: i32 = match id_str.parse() {
            Ok(id) => id,
            Err(_) => continue,
        };

        let en_name = match info.get("name").and_then(|n| n.as_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        let gold_total = info
            .pointer("/gold/total")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;

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
            .get(id_str)
            .cloned()
            .unwrap_or_else(|| en_name.clone());

        // All items go into en_to_ru / en_to_gold / en_to_tags (for user message replacement)
        en_to_ru.insert(en_name.clone(), ru_name.clone());
        en_to_gold.insert(en_name.clone(), gold_total);
        en_to_tags.insert(en_name.clone(), tags.clone());

        // Filter for final items (AI catalog in system prompt)
        if is_final_item(info) {
            items.push(CatalogItem {
                id,
                en_name,
                ru_name,
                gold_total,
                tags,
            });
        }
    }

    // Sort by gold cost descending for readability
    items.sort_by(|a, b| b.gold_total.cmp(&a.gold_total));

    log::info!(
        "[item_catalog] Loaded {} final items, {} total mappings (DDragon {})",
        items.len(),
        en_to_ru.len(),
        DDRAGON_VERSION
    );

    ItemCatalog {
        items,
        en_to_ru,
        en_to_gold,
        en_to_tags,
    }
}

/// Check if an item qualifies as a "final" item for the AI catalog.
/// Criteria:
/// - maps["11"] == true (Summoner's Rift)
/// - gold.purchasable == true
/// - `into` is empty or absent (no further upgrades)
/// - `from` is present and non-empty (normal build path from components)
/// - gold.total > 0 (exclude free trinkets)
/// - No "Consumable" or "Trinket" tag
fn is_final_item(info: &serde_json::Value) -> bool {
    // Must be available on Summoner's Rift
    let on_sr = info
        .pointer("/maps/11")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !on_sr {
        return false;
    }

    // Must be purchasable
    let purchasable = info
        .pointer("/gold/purchasable")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !purchasable {
        return false;
    }

    // Must be a final item (no further upgrades)
    let into_empty = match info.get("into") {
        None => true,
        Some(arr) => arr.as_array().map(|a| a.is_empty()).unwrap_or(true),
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

    // Must cost gold (exclude free trinkets)
    let gold_total = info
        .pointer("/gold/total")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    if gold_total <= 0 {
        return false;
    }

    // Must not be consumable or trinket
    let tags: Vec<String> = info
        .get("tags")
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    if tags.iter().any(|t| t == "Consumable" || t == "Trinket") {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(
        id: &str,
        name: &str,
        gold_total: i64,
        purchasable: bool,
        on_sr: bool,
        from: Option<Vec<&str>>,
        into: Option<Vec<&str>>,
        tags: Vec<&str>,
    ) -> (String, serde_json::Value) {
        let mut item = serde_json::json!({
            "name": name,
            "gold": {
                "total": gold_total,
                "purchasable": purchasable
            },
            "maps": {
                "11": on_sr,
                "12": true
            },
            "tags": tags,
        });

        if let Some(items) = into {
            item.as_object_mut()
                .unwrap()
                .insert("into".to_string(), serde_json::json!(items));
        }

        if let Some(items) = from {
            item.as_object_mut()
                .unwrap()
                .insert("from".to_string(), serde_json::json!(items));
        }

        (id.to_string(), item)
    }

    fn build_test_en_json(items: Vec<(String, serde_json::Value)>) -> serde_json::Value {
        let mut data = serde_json::Map::new();
        for (id, val) in items {
            data.insert(id, val);
        }
        serde_json::json!({ "data": data })
    }

    fn build_test_ru_json(items: Vec<(&str, &str)>) -> serde_json::Value {
        let mut data = serde_json::Map::new();
        for (id, name) in items {
            data.insert(id.to_string(), serde_json::json!({ "name": name }));
        }
        serde_json::json!({ "data": data })
    }

    #[test]
    fn final_item_passes_filter() {
        let (_, item) = make_item(
            "3031",
            "Infinity Edge",
            3400,
            true,  // purchasable
            true,  // on SR
            Some(vec!["1038", "1018"]),
            Some(vec![]), // empty into = final
            vec!["Damage", "CriticalStrike"],
        );
        assert!(is_final_item(&item));
    }

    #[test]
    fn component_item_filtered_out() {
        let (_, item) = make_item(
            "1026",
            "B. F. Sword",
            1300,
            true,
            true,
            Some(vec!["1036"]),
            Some(vec!["3031", "3072"]), // builds into other items
            vec!["Damage"],
        );
        assert!(!is_final_item(&item));
    }

    #[test]
    fn item_without_into_field_is_final() {
        let (_, item) = make_item(
            "3031",
            "Infinity Edge",
            3400,
            true,
            true,
            Some(vec!["1038", "1018"]),
            None, // no `into` field at all
            vec!["Damage", "CriticalStrike"],
        );
        assert!(is_final_item(&item));
    }

    #[test]
    fn trinket_filtered_out() {
        let (_, item) = make_item(
            "3340",
            "Stealth Ward",
            0,
            true,
            true,
            Some(vec![]),
            Some(vec![]),
            vec!["Trinket"],
        );
        assert!(!is_final_item(&item)); // gold == 0 AND trinket tag
    }

    #[test]
    fn consumable_filtered_out() {
        let (_, item) = make_item(
            "2003",
            "Health Potion",
            50,
            true,
            true,
            Some(vec![]),
            Some(vec![]),
            vec!["Consumable"],
        );
        assert!(!is_final_item(&item));
    }

    #[test]
    fn zero_gold_item_filtered_out() {
        let (_, item) = make_item(
            "3400",
            "Your Cut",
            0,
            true,
            true,
            Some(vec!["1036"]),
            Some(vec![]),
            vec!["Damage"],
        );
        assert!(!is_final_item(&item));
    }

    #[test]
    fn non_sr_item_filtered_out() {
        let (_, item) = make_item(
            "9999",
            "ARAM Only Item",
            3000,
            true,
            false, // not on SR
            Some(vec!["1036"]),
            Some(vec![]),
            vec!["Damage"],
        );
        assert!(!is_final_item(&item));
    }

    #[test]
    fn non_purchasable_item_filtered_out() {
        let (_, item) = make_item(
            "9998",
            "Ornn Upgrade",
            0,
            false, // not purchasable
            true,
            Some(vec!["1036"]),
            Some(vec![]),
            vec!["Damage"],
        );
        assert!(!is_final_item(&item));
    }

    #[test]
    fn item_without_recipe_filtered_out() {
        let (_, item) = make_item(
            "663039",
            "Atma's Reckoning",
            3000,
            true,
            true,
            Some(vec![]),
            Some(vec![]),
            vec!["Health", "CriticalStrike", "Lane"],
        );
        assert!(!is_final_item(&item));
    }

    #[test]
    fn build_catalog_creates_correct_mappings() {
        let en_json = build_test_en_json(vec![
            make_item("3031", "Infinity Edge", 3400, true, true, Some(vec!["1038", "1018"]), Some(vec![]), vec!["Damage", "CriticalStrike"]),
            make_item("1026", "B. F. Sword", 1300, true, true, Some(vec!["1036"]), Some(vec!["3031"]), vec!["Damage"]),
            make_item("3340", "Stealth Ward", 0, true, true, Some(vec![]), Some(vec![]), vec!["Trinket"]),
            make_item("2003", "Health Potion", 50, true, true, Some(vec![]), Some(vec![]), vec!["Consumable"]),
            make_item("2900", "Spirit Visage", 2900, true, true, Some(vec!["1057", "3211"]), None, vec!["Health", "SpellBlock"]),
        ]);

        let ru_json = build_test_ru_json(vec![
            ("3031", "\u{0413}\u{0440}\u{0430}\u{043d}\u{044c} \u{0411}\u{0435}\u{0441}\u{043a}\u{043e}\u{043d}\u{0435}\u{0447}\u{043d}\u{043e}\u{0441}\u{0442}\u{0438}"),
            ("1026", "\u{041c}\u{0435}\u{0447} \u{041c}\u{043e}\u{0433}\u{0443}\u{0447}\u{0435}\u{0433}\u{043e}"),
            ("3340", "\u{0422}\u{043e}\u{0442}\u{0435}\u{043c} \u{041d}\u{0435}\u{0432}\u{0438}\u{0434}\u{0438}\u{043c}\u{043e}\u{0441}\u{0442}\u{0438}"),
            ("2003", "\u{0417}\u{0435}\u{043b}\u{044c}\u{0435} \u{0417}\u{0434}\u{043e}\u{0440}\u{043e}\u{0432}\u{044c}\u{044f}"),
            ("2900", "\u{041e}\u{0431}\u{043b}\u{0430}\u{0447}\u{0435}\u{043d}\u{0438}\u{0435} \u{0414}\u{0443}\u{0445}\u{0430}"),
        ]);

        let catalog = build_catalog(en_json, Some(ru_json));

        // Only 2 final items: Infinity Edge and Spirit Visage
        assert_eq!(catalog.items.len(), 2);

        // en_to_ru has ALL items (including components, trinkets, consumables)
        assert_eq!(catalog.en_to_ru.len(), 5);
        assert_eq!(catalog.en_to_ru.get("Infinity Edge").unwrap(), "\u{0413}\u{0440}\u{0430}\u{043d}\u{044c} \u{0411}\u{0435}\u{0441}\u{043a}\u{043e}\u{043d}\u{0435}\u{0447}\u{043d}\u{043e}\u{0441}\u{0442}\u{0438}");
        assert_eq!(catalog.en_to_ru.get("B. F. Sword").unwrap(), "\u{041c}\u{0435}\u{0447} \u{041c}\u{043e}\u{0433}\u{0443}\u{0447}\u{0435}\u{0433}\u{043e}");
        assert_eq!(catalog.en_to_ru.get("Health Potion").unwrap(), "\u{0417}\u{0435}\u{043b}\u{044c}\u{0435} \u{0417}\u{0434}\u{043e}\u{0440}\u{043e}\u{0432}\u{044c}\u{044f}");

        // Gold mappings
        assert_eq!(*catalog.en_to_gold.get("Infinity Edge").unwrap(), 3400);
        assert_eq!(*catalog.en_to_gold.get("B. F. Sword").unwrap(), 1300);

        // Tags
        assert_eq!(
            catalog.en_to_tags.get("Infinity Edge").unwrap(),
            &vec!["Damage".to_string(), "CriticalStrike".to_string()]
        );
    }

    #[test]
    fn build_catalog_without_ru_uses_en_names() {
        let en_json = build_test_en_json(vec![
            make_item("3031", "Infinity Edge", 3400, true, true, Some(vec!["1038", "1018"]), Some(vec![]), vec!["Damage"]),
        ]);

        let catalog = build_catalog(en_json, None);

        assert_eq!(catalog.items.len(), 1);
        assert_eq!(catalog.en_to_ru.get("Infinity Edge").unwrap(), "Infinity Edge");
    }

    #[test]
    fn empty_en_json_returns_empty_catalog() {
        let en_json = serde_json::json!({});
        let catalog = build_catalog(en_json, None);

        assert!(catalog.items.is_empty());
        assert!(catalog.en_to_ru.is_empty());
    }

    #[test]
    fn items_sorted_by_gold_descending() {
        let en_json = build_test_en_json(vec![
            make_item("1001", "Cheap Item", 1000, true, true, Some(vec!["1036"]), Some(vec![]), vec!["Damage"]),
            make_item("1002", "Expensive Item", 4000, true, true, Some(vec!["1038", "1037"]), Some(vec![]), vec!["Damage"]),
            make_item("1003", "Mid Item", 2500, true, true, Some(vec!["1036", "1037"]), Some(vec![]), vec!["Damage"]),
        ]);

        let catalog = build_catalog(en_json, None);

        assert_eq!(catalog.items.len(), 3);
        assert_eq!(catalog.items[0].gold_total, 4000);
        assert_eq!(catalog.items[1].gold_total, 2500);
        assert_eq!(catalog.items[2].gold_total, 1000);
    }
}
