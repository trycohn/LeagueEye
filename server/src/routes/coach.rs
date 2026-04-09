use axum::{
    extract::State,
    response::sse::{Event, Sse},
    Json,
};
use futures::stream::Stream;
use leagueeye_shared::models::{CoachPlayerInfo, CoachingContext, CoachStreamPayload};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

use crate::{AiCoachProvider, AppState};

// ─── System prompt ──────────────────────────────────────────────────────────

fn build_system_prompt(phase: &str) -> String {
    if phase == "champ_select" {
        return r#"Ты — AI-тренер по League of Legends. Анализируй драфт и давай рекомендации.

Структура данных:
- Блок «=== Я (игрок) ===» — это твой подопечный
- В списке команды он помечен «[Я]»
- «Моя команда» — СОЮЗНИКИ. «Вражеская команда» — ПРОТИВНИКИ. Не путай их

Правила:
- Отвечай ТОЛЬКО на русском языке
- ФОРМАТ ОТВЕТА: РОВНО 2 строки, каждая начинается с «- » (дефис + пробел). НИЧЕГО больше — никаких заголовков, секций, вступлений, заключений, пояснений, цитат, выделений
- Каждый совет — максимум 15 слов. Коротко и по делу
- Фокусируйся на самом важном: контр-пики, синергия, слабые стороны драфта
- Называй чемпионов по ПОЛНЫМ именам (Мордекайзер, Мисс Фортуна, Чо'Гат)
- НЕ пиши названия умений — используй ТОЛЬКО (Q), (W), (E), (R), (Пассивное)
- НЕ предполагай что у чемпиона мана — ресурс указан в данных
- НИКАКОГО текста кроме двух строк с советами"#.to_string();
    }

    r#"Ты — AI-тренер по League of Legends. Анализируй текущее состояние игры и давай рекомендации.

Структура данных:
- Блок «=== Я (игрок) ===» — это твой подопечный. Его статы (KDA, CS, золото, предметы) указаны в этом блоке — ИСПОЛЬЗУЙ ИХ
- В списке команды он помечен «[Я]»
- «Моя команда» — СОЮЗНИКИ. «Вражеская команда» — ПРОТИВНИКИ. Не путай их
- Когда говоришь про статы игрока — сверяйся с данными из блока «=== Я (игрок) ===»

Правила:
- Отвечай ТОЛЬКО на русском языке
- ФОРМАТ ОТВЕТА: РОВНО 2 строки, каждая начинается с «- » (дефис + пробел). НИЧЕГО больше — никаких заголовков, секций, вступлений, заключений, пояснений, цитат, выделений, приоритетов, анализов
- Каждый совет — максимум 15 слов. Коротко и по делу
- Фокусируйся на самом важном прямо сейчас: KDA, CS, золото, предметы, время игры
- Советуй конкретно: "Иди на дракона", "Фарми", "Сплитпушь топ", "Не дерись"
- Называй чемпионов по ПОЛНЫМ именам (Мордекайзер, Мисс Фортуна, Чо'Гат)
- НЕ пиши названия умений — используй ТОЛЬКО (Q), (W), (E), (R), (Пассивное)
- НЕ предполагай что у чемпиона мана — ресурс указан в данных
- НИКАКОГО текста кроме двух строк с советами"#.to_string()
}

// ─── Build user message ─────────────────────────────────────────────────────

fn build_user_message(ctx: &CoachingContext) -> String {
    let mut msg = String::new();

    if ctx.phase == "champ_select" {
        msg.push_str("Фаза: Выбор чемпионов\n");
    } else {
        let time_str = ctx.game_time_secs
            .map(|t| format!("{}:{:02}", t / 60, t % 60))
            .unwrap_or_else(|| "?".to_string());
        msg.push_str(&format!("Фаза: В игре ({})\n", time_str));
    }

    // Dedicated block for the player being coached
    msg.push_str("\n=== Я (игрок) ===\n");
    if !ctx.my_champion.is_empty() {
        msg.push_str(&format!("Чемпион: {} ({})\n", ctx.my_champion,
            if ctx.my_position.is_empty() { "?" } else { &ctx.my_position }));
    }

    // Champion resource and class info
    if let Some(ref resource) = ctx.my_champion_resource {
        let display = format_resource(resource);
        msg.push_str(&format!("Ресурс: {}\n", display));
    }
    if let Some(ref class) = ctx.my_champion_class {
        msg.push_str(&format!("Класс: {}\n", class));
    }

    // Champion abilities summary
    if let Some(ref abilities) = ctx.my_champion_abilities_summary {
        if !abilities.is_empty() {
            msg.push_str("Умения:\n");
            msg.push_str(abilities);
            msg.push('\n');
        }
    }

    // Official Riot tips
    if let Some(ref tips) = ctx.my_champion_ally_tips {
        if !tips.is_empty() {
            msg.push_str("Советы Riot:\n");
            for tip in tips {
                msg.push_str(&format!("- {}\n", tip));
            }
        }
    }

    // Find the player's stats from my_team
    if let Some(me) = ctx.my_team.iter().find(|p| p.champion_name == ctx.my_champion) {
        if ctx.phase != "champ_select" {
            msg.push_str(&format!("KDA: {}/{}/{} | CS: {} | Уровень: {}",
                me.kills, me.deaths, me.assists, me.cs, me.level));
            if let Some(gold) = ctx.my_gold {
                msg.push_str(&format!(" | Золото: {}", gold as i64));
            }
            msg.push('\n');
            if !me.items.is_empty() {
                msg.push_str(&format!("Предметы: {}\n", me.items.join(", ")));
            }
        }
        if !me.rank_display.is_empty() {
            msg.push_str(&format!("Ранг: {}\n", me.rank_display));
        }
    }
    if !ctx.my_summoner_spells.is_empty() {
        msg.push_str(&format!("Суммонеры: {}\n", ctx.my_summoner_spells.join(", ")));
    }
    if let Some(runes) = &ctx.my_runes {
        msg.push_str(&format!("Руны: {}\n", runes));
    }
    if let Some(stats) = &ctx.my_stats {
        msg.push_str(&format!("Статы: AD:{:.0} AP:{:.0} Armor:{:.0} MR:{:.0} HP:{:.0}/{:.0} AS:{:.2}\n",
            stats.attack_damage, stats.ability_power, stats.armor,
            stats.magic_resist, stats.current_health, stats.max_health,
            stats.attack_speed));
    }

    // Team listing with [Я] marker
    msg.push_str("\nМоя команда (СОЮЗНИКИ):\n");
    for p in &ctx.my_team {
        let is_me = p.champion_name == ctx.my_champion
            && (ctx.my_position.is_empty() || p.position == ctx.my_position);
        let marker = if is_me { "[Я] " } else { "" };
        write_player_line(&mut msg, p, marker, ctx.phase == "champ_select");
    }

    msg.push_str("\nВражеская команда (ПРОТИВНИКИ):\n");
    for p in &ctx.enemy_team {
        write_player_line(&mut msg, p, "", ctx.phase == "champ_select");
    }

    if !ctx.recent_events.is_empty() {
        msg.push_str("\nПоследние события:\n");
        for ev in &ctx.recent_events {
            msg.push_str(&format!("- {}\n", ev));
        }
    }

    msg.push_str("\nДай мне конкретные советы для текущей ситуации.");
    msg
}

/// Format resource for display. "None" → "Без ресурса", others as-is with English name in parens
fn format_resource(resource: &str) -> String {
    match resource {
        "None" => "Без ресурса".to_string(),
        "Mana" => "Мана".to_string(),
        "Energy" => "Энергия".to_string(),
        "Fury" => "Ярость".to_string(),
        "Shield" => "Щит".to_string(),
        "Heat" => "Нагрев".to_string(),
        "Flow" => "Поток".to_string(),
        "Courage" => "Храбрость".to_string(),
        "Blood Well" => "Без ресурса (Blood Well)".to_string(),
        "Ferocity" => "Свирепость".to_string(),
        "Grit" => "Стойкость".to_string(),
        "Rage" => "Ярость".to_string(),
        "Crimson Rush" => "Без ресурса (Crimson Rush)".to_string(),
        "None (Costs Health)" => "Без ресурса (тратит HP)".to_string(),
        other => format!("{} (англ.)", other),
    }
}

fn write_player_line(msg: &mut String, p: &CoachPlayerInfo, marker: &str, is_champ_select: bool) {
    msg.push_str(&format!("- {}{} ({}) ", marker, p.champion_name,
        if p.position.is_empty() { "?" } else { &p.position }));

    // Add resource and class info
    let mut meta_parts = Vec::new();
    if let Some(ref resource) = p.champion_resource {
        meta_parts.push(format_resource(resource));
    }
    if let Some(ref class) = p.champion_class {
        meta_parts.push(class.clone());
    }
    if !meta_parts.is_empty() {
        msg.push_str(&format!("— {} ", meta_parts.join(", ")));
    }

    if !p.summoner_spells.is_empty() {
        msg.push_str(&format!("— {} ", p.summoner_spells.join("/")));
    }
    if !p.keystone_rune.is_empty() {
        msg.push_str(&format!("— {} ", p.keystone_rune));
    }
    if !p.rank_display.is_empty() {
        msg.push_str(&format!("— {} ", p.rank_display));
    }
    if !is_champ_select {
        msg.push_str(&format!("— {}/{}/{} — {} CS — Lv{}",
            p.kills, p.deaths, p.assists, p.cs, p.level));
        if !p.items.is_empty() {
            msg.push_str(&format!(" — Items: {}", p.items.join(", ")));
        }
        if p.is_dead {
            let secs = p.respawn_timer as i64;
            if secs > 0 {
                msg.push_str(&format!(" — [МЁРТВ {}с]", secs));
            } else {
                msg.push_str(" — [МЁРТВ]");
            }
        }
    }
    msg.push('\n');
}

// ─── SSE streaming endpoint ─────────────────────────────────────────────────

pub async fn stream_coach(
    State(state): State<Arc<AppState>>,
    Json(ctx): Json<CoachingContext>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let config = state.ai_coach_config.clone();
    let system_prompt = build_system_prompt(&ctx.phase);
    let user_message = build_user_message(&ctx);

    log::info!("[coach] === SYSTEM PROMPT ===\n{}\n=========================", system_prompt);
    log::info!("[coach] === USER MESSAGE ===\n{}\n========================", user_message);

    Sse::new(make_ai_stream(config, system_prompt, user_message))
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(std::time::Duration::from_secs(15))
                .text("ping"),
        )
}

type CoachStream = Pin<Box<dyn Stream<Item = Result<Event, std::convert::Infallible>> + Send>>;

fn log_outbound_request(provider: &str, url: &str, body: &serde_json::Value) {
    match serde_json::to_string_pretty(body) {
        Ok(serialized) => log::info!(
            "[coach] === OUTBOUND {} REQUEST ===\nPOST {}\n{}\n========================",
            provider,
            url,
            serialized
        ),
        Err(error) => log::warn!(
            "[coach] Failed to serialize {} request body for logging: {}",
            provider,
            error
        ),
    }
}

fn provider_display_name(provider: AiCoachProvider) -> &'static str {
    match provider {
        AiCoachProvider::Anthropic => "Anthropic",
        AiCoachProvider::OpenRouter => "OpenRouter",
        AiCoachProvider::DeepSeek => "DeepSeek",
    }
}

fn invalid_api_key_message(provider: AiCoachProvider) -> &'static str {
    match provider {
        AiCoachProvider::Anthropic => "Неверный API ключ Anthropic на сервере",
        AiCoachProvider::OpenRouter => "Неверный API ключ OpenRouter на сервере",
        AiCoachProvider::DeepSeek => "Неверный API ключ DeepSeek на сервере",
    }
}

fn make_ai_stream(
    config: Option<crate::AiCoachConfig>,
    system_prompt: String,
    user_message: String,
) -> CoachStream {
    let provider = config.as_ref().map(|c| c.provider);
    match provider {
        Some(AiCoachProvider::Anthropic) => Box::pin(make_anthropic_stream(config, system_prompt, user_message)),
        Some(AiCoachProvider::OpenRouter) | Some(AiCoachProvider::DeepSeek) => {
            Box::pin(make_openai_compatible_stream(config, system_prompt, user_message))
        }
        None => Box::pin(make_no_config_stream()),
    }
}

fn make_no_config_stream() -> impl Stream<Item = Result<Event, std::convert::Infallible>> {
    async_stream::stream! {
        yield Ok(Event::default().data(
            serde_json::to_string(&CoachStreamPayload {
                kind: "error".to_string(),
                text: Some("AI Coach не настроен на сервере (не задан AI_COACH_PROVIDER и нет ключей провайдера)".to_string()),
            }).unwrap()
        ));
    }
}

fn make_anthropic_stream(
    config: Option<crate::AiCoachConfig>,
    system_prompt: String,
    user_message: String,
) -> impl Stream<Item = Result<Event, std::convert::Infallible>> {
    async_stream::stream! {
        let config = match config {
            Some(c) => c,
            None => {
                yield Ok(Event::default().data(
                    serde_json::to_string(&CoachStreamPayload {
                        kind: "error".to_string(),
                        text: Some("AI Coach не настроен на сервере (ключ провайдера не задан)".to_string()),
                    }).unwrap()
                ));
                return;
            }
        };

        // Emit start
        yield Ok(Event::default().data(
            serde_json::to_string(&CoachStreamPayload {
                kind: "start".to_string(),
                text: None,
            }).unwrap()
        ));

        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(90))
            .build()
            .unwrap_or_default();
        let body = serde_json::json!({
            "model": config.model,
            "max_tokens": config.max_tokens,
            "stream": true,
            "system": system_prompt,
            "messages": [{
                "role": "user",
                "content": user_message
            }]
        });

        let url = format!("{}/v1/messages", config.base_url.trim_end_matches('/'));
        let send_start = Instant::now();
        log_outbound_request("ANTHROPIC", &url, &body);
        log::info!("[coach] Отправляю запрос к {} (model: {})", url, config.model);

        let response = client
            .post(&url)
            .header("x-api-key", &config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await;

        log::info!("[coach] Ответ от AI получен через {:.2}s", send_start.elapsed().as_secs_f32());

        let mut response = match response {
            Ok(r) => r,
            Err(e) => {
                log::error!("[coach] Ошибка соединения с Anthropic: {}", e);
                yield Ok(Event::default().data(
                    serde_json::to_string(&CoachStreamPayload {
                        kind: "error".to_string(),
                        text: Some(format!("Ошибка соединения с AI: {}", e)),
                    }).unwrap()
                ));
                return;
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let body_text = response.text().await.unwrap_or_default();
            log::error!("[coach] Anthropic API error ({}): {}", status, body_text);
            let msg = if status.as_u16() == 401 {
                "Неверный API ключ Anthropic на сервере".to_string()
            } else {
                format!("AI API ошибка ({}): {}", status, body_text)
            };
            yield Ok(Event::default().data(
                serde_json::to_string(&CoachStreamPayload {
                    kind: "error".to_string(),
                    text: Some(msg),
                }).unwrap()
            ));
            return;
        }

        // Read SSE stream from Anthropic line-by-line (same approach as OpenRouter).
        let mut buffer = String::new();
        let mut last_chunk_time = Instant::now();
        let mut first_token_sent = false;
        let first_token_start = Instant::now();

        while let Some(chunk) = response.chunk().await.ok().flatten() {
            let chunk_received = Instant::now();
            if chunk_received.duration_since(last_chunk_time).as_secs() > 2 {
                log::warn!("[coach] Пауза между чанками {:.1}s (Anthropic)",
                    chunk_received.duration_since(last_chunk_time).as_secs_f32());
            }
            last_chunk_time = chunk_received;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim_end().to_string();
                buffer = buffer[pos + 1..].to_string();

                if line.is_empty() || line.starts_with("event:") {
                    continue;
                }
                if let Some(data) = line.strip_prefix("data: ").or_else(|| line.strip_prefix("data:")) {
                    let data = data.trim_start();
                    if data == "[DONE]" {
                        continue;
                    }
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                        if parsed.get("type").and_then(|t| t.as_str()) == Some("content_block_delta") {
                            if let Some(text) = parsed
                                .get("delta")
                                .and_then(|d| d.get("text"))
                                .and_then(|t| t.as_str())
                            {
                                if !first_token_sent {
                                    first_token_sent = true;
                                    log::info!("[coach] Первый токен через {:.2}s от начала ответа",
                                        first_token_start.elapsed().as_secs_f32());
                                }
                                yield Ok(Event::default().data(
                                    serde_json::to_string(&CoachStreamPayload {
                                        kind: "delta".to_string(),
                                        text: Some(text.to_string()),
                                    }).unwrap()
                                ));
                            }
                        }
                        if parsed.get("type").and_then(|t| t.as_str()) == Some("error") {
                            let err_msg = parsed.get("error")
                                .and_then(|e| e.get("message"))
                                .and_then(|m| m.as_str())
                                .unwrap_or("Unknown AI error");
                            log::error!("[coach] Anthropic stream error: {}", err_msg);
                            yield Ok(Event::default().data(
                                serde_json::to_string(&CoachStreamPayload {
                                    kind: "error".to_string(),
                                    text: Some(err_msg.to_string()),
                                }).unwrap()
                            ));
                            return;
                        }
                    }
                }
            }
        }

        // Emit end
        log::info!("[coach] Anthropic stream completed");
        yield Ok(Event::default().data(
            serde_json::to_string(&CoachStreamPayload {
                kind: "end".to_string(),
                text: None,
            }).unwrap()
        ));
    }
}

fn make_openai_compatible_stream(
    config: Option<crate::AiCoachConfig>,
    system_prompt: String,
    user_message: String,
) -> impl Stream<Item = Result<Event, std::convert::Infallible>> {
    async_stream::stream! {
        let config = match config {
            Some(c) => c,
            None => {
                yield Ok(Event::default().data(
                    serde_json::to_string(&CoachStreamPayload {
                        kind: "error".to_string(),
                        text: Some("AI Coach не настроен на сервере (ключ провайдера не задан)".to_string()),
                    }).unwrap()
                ));
                return;
            }
        };
        let provider = config.provider;
        let provider_name = provider_display_name(provider);

        // Emit start
        yield Ok(Event::default().data(
            serde_json::to_string(&CoachStreamPayload {
                kind: "start".to_string(),
                text: None,
            }).unwrap()
        ));

        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(90))
            .build()
            .unwrap_or_default();

        let body = serde_json::json!({
            "model": config.model,
            "stream": true,
            "max_tokens": config.max_tokens,
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": user_message }
            ]
        });

        let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));
        let send_start = Instant::now();
        log_outbound_request(provider_name, &url, &body);
        log::info!(
            "[coach] Отправляю запрос к {} (provider: {}, model: {})",
            url,
            provider_name,
            config.model
        );

        let mut req = client
            .post(&url)
            .header("authorization", format!("Bearer {}", config.api_key))
            .header("content-type", "application/json")
            .json(&body);

        if provider == AiCoachProvider::OpenRouter {
            if let Some(r) = &config.openrouter_http_referer {
                req = req.header("http-referer", r);
            }
            if let Some(t) = &config.openrouter_title {
                req = req.header("x-openrouter-title", t);
            }
        }

        let response = req.send().await;

        log::info!(
            "[coach] Ответ от {} получен через {:.2}s",
            provider_name,
            send_start.elapsed().as_secs_f32()
        );

        let mut response = match response {
            Ok(r) => r,
            Err(e) => {
                log::error!("[coach] Ошибка соединения с {}: {}", provider_name, e);
                yield Ok(Event::default().data(
                    serde_json::to_string(&CoachStreamPayload {
                        kind: "error".to_string(),
                        text: Some(format!("Ошибка соединения с AI: {}", e)),
                    }).unwrap()
                ));
                return;
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let body_text = response.text().await.unwrap_or_default();
            log::error!("[coach] {} API error ({}): {}", provider_name, status, body_text);
            let msg = if status.as_u16() == 401 {
                invalid_api_key_message(provider).to_string()
            } else {
                format!("AI API ошибка ({}): {}", status, body_text)
            };
            yield Ok(Event::default().data(
                serde_json::to_string(&CoachStreamPayload {
                    kind: "error".to_string(),
                    text: Some(msg),
                }).unwrap()
            ));
            return;
        }

        // Read OpenAI-compatible SSE stream line-by-line.
        // Some providers send "data: {...}\n" with a single newline rather
        // than the double-newline that the SSE spec technically requires.
        // Waiting for "\n\n" causes massive buffering delays.
        let mut buffer = String::new();

        while let Some(chunk) = response.chunk().await.ok().flatten() {
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim_end().to_string();
                buffer = buffer[pos + 1..].to_string();

                if line.is_empty() || line.starts_with(':') {
                    continue;
                }
                if let Some(data) = line.strip_prefix("data: ").or_else(|| line.strip_prefix("data:")) {
                    let data = data.trim_start();
                    if data == "[DONE]" {
                        continue;
                    }
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                        let content = parsed
                            .get("choices")
                            .and_then(|c| c.get(0))
                            .and_then(|c0| c0.get("delta"))
                            .and_then(|d| d.get("content"))
                            .and_then(|t| t.as_str());
                        if let Some(text) = content {
                            if !text.is_empty() {
                                yield Ok(Event::default().data(
                                    serde_json::to_string(&CoachStreamPayload {
                                        kind: "delta".to_string(),
                                        text: Some(text.to_string()),
                                    }).unwrap()
                                ));
                            }
                        }

                        if let Some(err_msg) = parsed.get("error")
                            .and_then(|e| e.get("message"))
                            .and_then(|m| m.as_str())
                        {
                            log::error!("[coach] {} stream error: {}", provider_name, err_msg);
                            yield Ok(Event::default().data(
                                serde_json::to_string(&CoachStreamPayload {
                                    kind: "error".to_string(),
                                    text: Some(err_msg.to_string()),
                                }).unwrap()
                            ));
                            return;
                        }
                    }
                }
            }
        }

        // Emit end
        log::info!("[coach] {} stream completed", provider_name);
        yield Ok(Event::default().data(
            serde_json::to_string(&CoachStreamPayload {
                kind: "end".to_string(),
                text: None,
            }).unwrap()
        ));
    }
}
