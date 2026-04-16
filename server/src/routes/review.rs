use axum::{
    extract::{Path, State},
    response::sse::{Event, Sse},
    Json,
};
use futures::stream::Stream;
use leagueeye_shared::models::*;
use std::sync::Arc;

use crate::AppState;
use crate::item_catalog::ItemCatalog;
use crate::champion_catalog::ChampionCatalog;
use crate::routes::coach::{load_catalogs, make_ai_payload_stream};

// ─── POST /api/review/stream ────────────────────────────────────────────────

pub async fn stream_review(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PostGameReviewRequest>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let config = state.ai_coach_config.clone();

    // Check for cached review (unless force_refresh)
    if !req.force_refresh {
        if let Ok(Some(review)) = state.db.get_review(&req.match_id, &req.puuid).await {
            return Sse::new(make_cached_stream(review.review_text));
        }
    } else if let Err(e) = state.db.delete_review(&req.match_id, &req.puuid).await {
        return Sse::new(make_error_stream(&format!("Ошибка удаления старого разбора: {}", e)));
    }

    // Load catalogs
    let (catalog, champ_catalog) = load_catalogs(&state).await;

    // Load match detail
    let match_detail = match state.db.get_match_detail(&req.match_id).await {
        Ok(Some(detail)) => detail,
        Ok(None) => {
            // Try fetching from Riot API
            match state.riot_api.get_match(&req.match_id).await {
                Ok(dto) => {
                    let parts = dto_to_participants(&dto);
                    let _ = state.db.save_match_participants(&dto.metadata.match_id, &parts).await;
                    match state.db.get_match_detail(&req.match_id).await {
                        Ok(Some(d)) => d,
                        _ => return Sse::new(make_error_stream("Не удалось загрузить детали матча")),
                    }
                }
                Err(e) => return Sse::new(make_error_stream(&format!("Ошибка загрузки матча: {}", e))),
            }
        }
        Err(e) => return Sse::new(make_error_stream(&format!("Ошибка БД: {}", e))),
    };

    // Load timeline (cached or from Riot API)
    let timeline_json = match state.db.get_timeline(&req.match_id).await {
        Ok(Some(json)) => Some(json),
        _ => {
            match state.riot_api.get_match_timeline(&req.match_id).await {
                Ok(timeline) => {
                    if let Ok(json) = serde_json::to_value(&timeline) {
                        let _ = state.db.save_timeline(&req.match_id, &json).await;
                        Some(json)
                    } else {
                        None
                    }
                }
                Err(e) => {
                    log::warn!("[review] Timeline fetch failed for {}: {}", req.match_id, e);
                    None
                }
            }
        }
    };

    // Load player's champion stats for comparison
    let champion_stats = state.db.get_champion_stats_for_player(&req.puuid).await.unwrap_or_default();

    // Build prompts
    let system_prompt = build_review_system_prompt(catalog, champ_catalog);
    let user_message = build_review_user_message(
        &match_detail,
        &req.puuid,
        &champion_stats,
        timeline_json.as_ref(),
        catalog,
        champ_catalog,
    );

    log::info!("[review] === SYSTEM PROMPT ===\n{}\n=========================", system_prompt);
    log::info!("[review] === USER MESSAGE ===\n{}\n========================", user_message);

    // Stream AI response and collect for caching
    let db = state.db.clone();
    let match_id = req.match_id.clone();
    let puuid = req.puuid.clone();

    Sse::new(make_review_stream(config, system_prompt, user_message, db, match_id, puuid))
}

// ─── GET /api/review/{match_id}/{puuid} ─────────────────────────────────────

pub async fn get_cached_review(
    State(state): State<Arc<AppState>>,
    Path((match_id, puuid)): Path<(String, String)>,
) -> Result<Json<Option<PostGameReview>>, String> {
    state.db.get_review(&match_id, &puuid).await
        .map(Json)
        .map_err(|e| e.to_string())
}

// ─── System prompt ──────────────────────────────────────────────────────────

fn build_review_system_prompt(
    catalog: Option<&ItemCatalog>,
    champ_catalog: Option<&ChampionCatalog>,
) -> String {
    let item_block = if let Some(c) = catalog {
        if !c.items.is_empty() {
            let mut block = String::from("\n\n=== СПРАВОЧНИК ПРЕДМЕТОВ ===\nФормат: РУ (EN) | цена | теги\n");
            for item in &c.items {
                let tags = if item.tags.is_empty() { String::new() } else { item.tags.join(",") };
                block.push_str(&format!("{} ({}) | {} | {}\n", item.ru_name, item.en_name, item.gold_total, tags));
            }
            block
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let _ = champ_catalog; // Champion catalog is used in user message per-champion

    format!(r#"Ты — AI-аналитик League of Legends, делающий post-game разбор матча.

Твоя задача — дать игроку конструктивный, конкретный и полезный разбор завершённого матча.

ФОРМАТ ОТВЕТА — строго 5 секций с заголовками:

**Итог**
Одно предложение — общая оценка перформанса игрока в этом матче.

**Что получилось**
2-3 пункта (каждый начинается с «- »). Конкретные сильные стороны игрока в этом матче с привязкой к данным.

**Проблемы**
2-3 пункта (каждый начинается с «- »). Конкретные ошибки и слабые места с привязкой к данным и timeline (если есть).

**Ключевые моменты**
2-3 пункта (каждый начинается с «- »). Поворотные события матча: значимые смерти, объективы, тимфайты. Привязывай к таймингам из timeline.

**Рекомендации**
2-3 пункта (каждый начинается с «- »). Конкретные советы что улучшить. Не общие фразы типа «фармите лучше», а конкретика: «при 6 CS/min на Jinx стоит целиться в 7.5+ — тренируй last-hitting под башней».

ПРАВИЛА:
- Отвечай ТОЛЬКО на русском языке
- Используй данные из матча: KDA, CS/min, gold/min, damage share, vision, timeline события
- Сравнивай со средними показателями игрока на этом чемпионе (если предоставлены)
- Сравнивай с лейн-оппонентом
- Называй чемпионов по ПОЛНЫМ русским именам
- НЕ выдумывай данные, которых нет в контексте
- Будь конструктивным, но честным — указывай на реальные проблемы
- Каждый пункт — максимум 2 предложения{item_block}"#)
}

// ─── User message ───────────────────────────────────────────────────────────

fn build_review_user_message(
    detail: &MatchDetail,
    puuid: &str,
    champion_stats: &[ChampionStat],
    timeline_json: Option<&serde_json::Value>,
    catalog: Option<&ItemCatalog>,
    champ_catalog: Option<&ChampionCatalog>,
) -> String {
    let mut msg = String::new();

    let game_duration_min = detail.game_duration as f64 / 60.0;

    // Find the player
    let me = match detail.participants.iter().find(|p| p.puuid == puuid) {
        Some(p) => p,
        None => {
            msg.push_str("Ошибка: игрок не найден в матче\n");
            return msg;
        }
    };

    let my_team_id = me.team_id;
    let my_team: Vec<&MatchParticipantDetail> = detail.participants.iter().filter(|p| p.team_id == my_team_id).collect();
    let enemy_team: Vec<&MatchParticipantDetail> = detail.participants.iter().filter(|p| p.team_id != my_team_id).collect();

    // Result
    msg.push_str(&format!("Результат: {} | Длительность: {:.1} мин\n\n",
        if me.win { "ПОБЕДА" } else { "ПОРАЖЕНИЕ" },
        game_duration_min));

    // Player stats
    let cs_per_min = if game_duration_min > 0.0 { me.cs as f64 / game_duration_min } else { 0.0 };
    let gold_per_min = if game_duration_min > 0.0 { me.gold as f64 / game_duration_min } else { 0.0 };
    let kda = if me.deaths == 0 { format!("Perfect") } else { format!("{:.1}", (me.kills + me.assists) as f64 / me.deaths as f64) };
    let team_damage: i64 = my_team.iter().map(|p| p.damage).sum();
    let damage_share = if team_damage > 0 { me.damage as f64 / team_damage as f64 * 100.0 } else { 0.0 };
    let team_kills: i32 = my_team.iter().map(|p| p.kills).sum();
    let kill_participation = if team_kills > 0 { (me.kills + me.assists) as f64 / team_kills as f64 * 100.0 } else { 0.0 };
    let vision_per_min = if game_duration_min > 0.0 { me.vision_score as f64 / game_duration_min } else { 0.0 };

    let champ_display = translate_champ(me.champion_name.as_str(), champ_catalog);

    msg.push_str("=== Я (игрок) ===\n");
    msg.push_str(&format!("Чемпион: {} ({})\n", champ_display, if me.position.is_empty() { "?" } else { &me.position }));
    msg.push_str(&format!("KDA: {}/{}/{} ({})\n", me.kills, me.deaths, me.assists, kda));
    msg.push_str(&format!("CS: {} ({:.1}/min)\n", me.cs, cs_per_min));
    msg.push_str(&format!("Золото: {} ({:.0}/min)\n", me.gold, gold_per_min));
    msg.push_str(&format!("Урон: {} ({:.0}% команды)\n", me.damage, damage_share));
    msg.push_str(&format!("Урон получен: {}\n", me.damage_taken));
    msg.push_str(&format!("Vision: {} ({:.1}/min) | Варды: {} поставлено, {} уничтожено\n",
        me.vision_score, vision_per_min, me.wards_placed, me.wards_killed));
    msg.push_str(&format!("Участие в убийствах: {:.0}%\n", kill_participation));
    msg.push_str(&format!("Уровень: {}\n", me.champ_level));

    // Items
    let items_display: Vec<String> = me.items.iter()
        .filter(|&&id| id > 0)
        .map(|&id| format_item_by_id(id, catalog))
        .collect();
    if !items_display.is_empty() {
        msg.push_str(&format!("Предметы: {}\n", items_display.join(", ")));
    }

    // Multi-kills
    let mut multi = Vec::new();
    if me.double_kills > 0 { multi.push(format!("{}x Double", me.double_kills)); }
    if me.triple_kills > 0 { multi.push(format!("{}x Triple", me.triple_kills)); }
    if me.quadra_kills > 0 { multi.push(format!("{}x Quadra", me.quadra_kills)); }
    if me.penta_kills > 0 { multi.push(format!("{}x Penta", me.penta_kills)); }
    if !multi.is_empty() {
        msg.push_str(&format!("Мультикиллы: {}\n", multi.join(", ")));
    }

    // Average stats comparison
    if let Some(avg) = champion_stats.iter().find(|s| s.champion_name == me.champion_name) {
        msg.push_str(&format!("\nМои средние на {} ({} игр, {:.0}% WR):\n", champ_display, avg.games, avg.winrate));
        msg.push_str(&format!("  Avg KDA: {:.1}/{:.1}/{:.1} | Avg CS: {:.0}\n",
            avg.avg_kills, avg.avg_deaths, avg.avg_assists, avg.avg_cs));

        // Compare this game vs average
        let kda_diff = if avg.avg_deaths > 0.0 {
            let avg_kda = (avg.avg_kills + avg.avg_assists) / avg.avg_deaths;
            let this_kda = if me.deaths > 0 { (me.kills + me.assists) as f64 / me.deaths as f64 } else { 99.0 };
            if this_kda > avg_kda { "лучше среднего" } else if this_kda < avg_kda * 0.8 { "хуже среднего" } else { "на уровне среднего" }
        } else { "N/A" };
        msg.push_str(&format!("  Эта игра по KDA: {}\n", kda_diff));
    }

    // Lane opponent
    if !me.position.is_empty() && me.position != "UNKNOWN" {
        if let Some(opponent) = enemy_team.iter().find(|p| p.position == me.position) {
            let opp_display = translate_champ(&opponent.champion_name, champ_catalog);
            let opp_cs_pm = if game_duration_min > 0.0 { opponent.cs as f64 / game_duration_min } else { 0.0 };
            let opp_kda = if opponent.deaths == 0 { "Perfect".to_string() } else { format!("{:.1}", (opponent.kills + opponent.assists) as f64 / opponent.deaths as f64) };
            msg.push_str(&format!("\n=== Лейн-оппонент ===\n"));
            msg.push_str(&format!("{} ({})\n", opp_display, opponent.position));
            msg.push_str(&format!("KDA: {}/{}/{} ({}) | CS: {} ({:.1}/min) | Gold: {} | Damage: {}\n",
                opponent.kills, opponent.deaths, opponent.assists, opp_kda,
                opponent.cs, opp_cs_pm, opponent.gold, opponent.damage));

            // Lane diff
            let cs_diff = me.cs - opponent.cs;
            let gold_diff = me.gold - opponent.gold;
            let dmg_diff = me.damage - opponent.damage;
            msg.push_str(&format!("Разница: CS {:+} | Gold {:+} | Damage {:+}\n", cs_diff, gold_diff, dmg_diff));
        }
    }

    // All 10 participants summary
    msg.push_str("\n=== Моя команда ===\n");
    for p in &my_team {
        let name = translate_champ(&p.champion_name, champ_catalog);
        let is_me = p.puuid == puuid;
        let marker = if is_me { "[Я] " } else { "" };
        msg.push_str(&format!("- {}{} ({}) {}/{}/{} | CS:{} | Gold:{} | Dmg:{}\n",
            marker, name, p.position, p.kills, p.deaths, p.assists, p.cs, p.gold, p.damage));
    }

    msg.push_str("\n=== Вражеская команда ===\n");
    for p in &enemy_team {
        let name = translate_champ(&p.champion_name, champ_catalog);
        msg.push_str(&format!("- {} ({}) {}/{}/{} | CS:{} | Gold:{} | Dmg:{}\n",
            name, p.position, p.kills, p.deaths, p.assists, p.cs, p.gold, p.damage));
    }

    // Timeline analysis
    if let Some(timeline) = timeline_json {
        msg.push_str("\n=== Timeline ===\n");

        // Map participant IDs to champion names
        let participant_map = build_participant_id_map(detail, timeline);

        // Extract key events from timeline
        if let Some(info) = timeline.get("info") {
            if let Some(frames) = info.get("frames").and_then(|f| f.as_array()) {
                // Gold progression for the player
                let my_participant_id = find_participant_id(detail, puuid);
                let opponent_participant_id = if !me.position.is_empty() && me.position != "UNKNOWN" {
                    enemy_team.iter()
                        .find(|p| p.position == me.position)
                        .and_then(|p| find_participant_id(detail, &p.puuid))
                } else {
                    None
                };

                if let Some(my_pid) = my_participant_id {
                    let mut gold_diffs = Vec::new();
                    for frame in frames {
                        let timestamp = frame.get("timestamp").and_then(|t| t.as_i64()).unwrap_or(0);
                        let min = timestamp / 60000;
                        if min == 0 { continue; }

                        if let Some(pframes) = frame.get("participantFrames") {
                            let my_gold = pframes.get(&my_pid.to_string())
                                .and_then(|p| p.get("totalGold"))
                                .and_then(|g| g.as_i64())
                                .unwrap_or(0);

                            if let Some(opp_pid) = opponent_participant_id {
                                let opp_gold = pframes.get(&opp_pid.to_string())
                                    .and_then(|p| p.get("totalGold"))
                                    .and_then(|g| g.as_i64())
                                    .unwrap_or(0);
                                gold_diffs.push((min, my_gold - opp_gold));
                            }
                        }
                    }

                    if !gold_diffs.is_empty() {
                        msg.push_str("Gold diff vs лейн-оппонент (по минутам):\n");
                        // Show key moments: every 5 min + min/max
                        for &(min, diff) in &gold_diffs {
                            if min % 5 == 0 || min == gold_diffs.last().unwrap().0 {
                                msg.push_str(&format!("  {}min: {:+}\n", min, diff));
                            }
                        }
                    }
                }

                // Key kills/deaths
                let mut kills_events = Vec::new();
                let mut death_events = Vec::new();
                let mut objectives = Vec::new();

                for frame in frames {
                    if let Some(events) = frame.get("events").and_then(|e| e.as_array()) {
                        for event in events {
                            let event_type = event.get("type").and_then(|t| t.as_str()).unwrap_or("");
                            let timestamp = event.get("timestamp").and_then(|t| t.as_i64()).unwrap_or(0);
                            let min = timestamp / 60000;
                            let sec = (timestamp % 60000) / 1000;

                            match event_type {
                                "CHAMPION_KILL" => {
                                    let killer_id = event.get("killerId").and_then(|k| k.as_i64()).unwrap_or(0);
                                    let victim_id = event.get("victimId").and_then(|v| v.as_i64()).unwrap_or(0);

                                    if let Some(my_pid) = my_participant_id {
                                        let killer_name = participant_map.get(&killer_id).cloned().unwrap_or_else(|| format!("ID:{}", killer_id));
                                        let victim_name = participant_map.get(&victim_id).cloned().unwrap_or_else(|| format!("ID:{}", victim_id));

                                        if killer_id == my_pid as i64 {
                                            kills_events.push(format!("{}:{:02} — Убил {}", min, sec, victim_name));
                                        } else if victim_id == my_pid as i64 {
                                            let assist_ids = event.get("assistingParticipantIds")
                                                .and_then(|a| a.as_array())
                                                .map(|arr| arr.iter()
                                                    .filter_map(|v| v.as_i64())
                                                    .filter_map(|id| participant_map.get(&id).cloned())
                                                    .collect::<Vec<_>>())
                                                .unwrap_or_default();
                                            let assist_str = if assist_ids.is_empty() { String::new() }
                                                else { format!(" (помогали: {})", assist_ids.join(", ")) };
                                            death_events.push(format!("{}:{:02} — Убит: {}{}", min, sec, killer_name, assist_str));
                                        }
                                    }
                                }
                                "ELITE_MONSTER_KILL" => {
                                    let monster = event.get("monsterType").and_then(|m| m.as_str()).unwrap_or("?");
                                    let sub = event.get("monsterSubType").and_then(|m| m.as_str());
                                    let killer_id = event.get("killerId").and_then(|k| k.as_i64()).unwrap_or(0);
                                    let killer_name = participant_map.get(&killer_id).cloned().unwrap_or_default();
                                    let monster_display = match monster {
                                        "DRAGON" => format!("Dragon{}", sub.map(|s| format!(" ({})", s)).unwrap_or_default()),
                                        "BARON_NASHOR" => "Baron".to_string(),
                                        "RIFTHERALD" => "Herald".to_string(),
                                        other => other.to_string(),
                                    };
                                    objectives.push(format!("{}:{:02} — {} убит ({})", min, sec, monster_display, killer_name));
                                }
                                "BUILDING_KILL" => {
                                    let building = event.get("buildingType").and_then(|b| b.as_str()).unwrap_or("?");
                                    let team_id = event.get("teamId").and_then(|t| t.as_i64()).unwrap_or(0);
                                    let lane = event.get("laneType").and_then(|l| l.as_str()).unwrap_or("");
                                    if building == "TOWER_BUILDING" {
                                        let tower = event.get("towerType").and_then(|t| t.as_str()).unwrap_or("");
                                        let whose = if team_id == my_team_id as i64 { "вражеская" } else { "наша" };
                                        objectives.push(format!("{}:{:02} — Башня разрушена ({} {} {})", min, sec, whose, lane, tower));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }

                if !kills_events.is_empty() {
                    msg.push_str("\nМои убийства:\n");
                    for e in kills_events.iter().take(10) {
                        msg.push_str(&format!("  {}\n", e));
                    }
                }

                if !death_events.is_empty() {
                    msg.push_str("\nМои смерти:\n");
                    for e in &death_events {
                        msg.push_str(&format!("  {}\n", e));
                    }
                }

                if !objectives.is_empty() {
                    msg.push_str("\nОбъективы:\n");
                    for e in objectives.iter().take(15) {
                        msg.push_str(&format!("  {}\n", e));
                    }
                }
            }
        }
    }

    msg.push_str("\nСделай разбор этого матча.");
    msg
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn translate_champ<'a>(name: &'a str, catalog: Option<&'a ChampionCatalog>) -> &'a str {
    match catalog {
        Some(c) => c.internal_to_ru.get(name).map(|s| s.as_str()).unwrap_or(name),
        None => name,
    }
}

fn format_item_by_id(item_id: i32, catalog: Option<&ItemCatalog>) -> String {
    let catalog = match catalog {
        Some(c) => c,
        None => return format!("Item:{}", item_id),
    };

    catalog.id_to_ru.get(&item_id)
        .cloned()
        .unwrap_or_else(|| format!("Item:{}", item_id))
}

fn find_participant_id(detail: &MatchDetail, puuid: &str) -> Option<i32> {
    detail.participants.iter()
        .position(|p| p.puuid == puuid)
        .map(|i| (i + 1) as i32) // Riot participant IDs are 1-indexed
}

fn build_participant_id_map(detail: &MatchDetail, _timeline: &serde_json::Value) -> std::collections::HashMap<i64, String> {
    let mut map = std::collections::HashMap::new();
    for (i, p) in detail.participants.iter().enumerate() {
        let id = (i + 1) as i64;
        map.insert(id, p.champion_name.clone());
    }
    map
}

// ─── Streams ────────────────────────────────────────────────────────────────

fn make_cached_stream(text: String) -> std::pin::Pin<Box<dyn Stream<Item = Result<Event, std::convert::Infallible>> + Send>> {
    Box::pin(async_stream::stream! {
        yield Ok(Event::default().data(
            serde_json::to_string(&CoachStreamPayload {
                kind: "cached".to_string(),
                text: Some(text),
            }).unwrap()
        ));
    })
}

fn make_error_stream(msg: &str) -> std::pin::Pin<Box<dyn Stream<Item = Result<Event, std::convert::Infallible>> + Send>> {
    let msg = msg.to_string();
    Box::pin(async_stream::stream! {
        yield Ok(Event::default().data(
            serde_json::to_string(&CoachStreamPayload {
                kind: "error".to_string(),
                text: Some(msg),
            }).unwrap()
        ));
    })
}

fn make_review_stream(
    config: Option<crate::AiCoachConfig>,
    system_prompt: String,
    user_message: String,
    db: crate::db::Db,
    match_id: String,
    puuid: String,
) -> std::pin::Pin<Box<dyn Stream<Item = Result<Event, std::convert::Infallible>> + Send>> {
    Box::pin(async_stream::stream! {
        let inner_stream = make_ai_payload_stream(config, system_prompt, user_message);
        let mut collected_text = String::new();

        use futures::StreamExt;
        let mut inner = std::pin::pin!(inner_stream);

        while let Some(payload) = inner.next().await {
            if payload.kind == "delta" {
                if let Some(text) = payload.text.as_deref() {
                    collected_text.push_str(text);
                }
            }

            if payload.kind == "end" && !collected_text.is_empty() {
                if let Err(e) = db.save_review(&match_id, &puuid, &collected_text).await {
                    log::error!(
                        "[review] Failed to save review for match {} / puuid {}: {}",
                        match_id,
                        puuid,
                        e
                    );
                }
            }

            yield Ok(Event::default().data(
                serde_json::to_string(&payload).unwrap()
            ));
        }
    })
}
