use axum::{
    extract::State,
    response::sse::{Event, Sse},
    Json,
};
use futures::stream::Stream;
use leagueeye_shared::models::{CoachingContext, CoachStreamPayload};
use std::sync::Arc;

use crate::AppState;

// ─── System prompt ──────────────────────────────────────────────────────────

fn build_system_prompt(phase: &str) -> String {
    if phase == "champ_select" {
        return r#"Ты — AI-тренер по League of Legends. Анализируй драфт (выбор чемпионов) и давай конкретные рекомендации игроку.

Правила:
- Отвечай ТОЛЬКО на русском языке
- Давай 3-5 коротких конкретных советов в формате маркированного списка
- Анализируй: синергию команд, контр-пики, win condition обеих команд, слабые/сильные стороны драфта
- Подскажи что билдить в первую очередь и на какого противника обращать внимание
- Будь конкретным: называй чемпионов по имени, говори "избегай файтов до 2 предметов" вместо "будь аккуратен"
- НЕ пиши введение или заключение, только советы"#.to_string();
    }

    r#"Ты — AI-тренер по League of Legends. Анализируй текущее состояние игры и давай конкретные рекомендации игроку прямо сейчас.

Правила:
- Отвечай ТОЛЬКО на русском языке
- Давай 3-5 коротких конкретных советов в формате маркированного списка
- Учитывай: KDA, CS, золото, предметы, время игры, командные составы, последние события
- Советуй конкретные действия: "Иди на дракона сейчас", "Фарми до Kraken Slayer", "Сплитпушь топ", "Не дерись — у врага пауэрспайк"
- Оценивай кто впереди (по золоту, уровням, KDA) и адаптируй советы
- Указывай приоритетные цели для фокуса в тимфайтах
- НЕ пиши введение или заключение, только советы"#.to_string()
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

    if !ctx.my_champion.is_empty() {
        msg.push_str(&format!("Мой чемпион: {} ({})\n", ctx.my_champion,
            if ctx.my_position.is_empty() { "?" } else { &ctx.my_position }));
    }

    msg.push_str("\nМоя команда:\n");
    for p in &ctx.my_team {
        msg.push_str(&format!("- {} ({}) ", p.champion_name,
            if p.position.is_empty() { "?" } else { &p.position }));
        if !p.rank_display.is_empty() {
            msg.push_str(&format!("— {} ", p.rank_display));
        }
        if ctx.phase != "champ_select" {
            msg.push_str(&format!("— {}/{}/{} — {} CS — Lv{}",
                p.kills, p.deaths, p.assists, p.cs, p.level));
            if !p.items.is_empty() {
                msg.push_str(&format!(" — Items: {}", p.items.join(", ")));
            }
        }
        msg.push('\n');
    }

    msg.push_str("\nВражеская команда:\n");
    for p in &ctx.enemy_team {
        msg.push_str(&format!("- {} ({}) ", p.champion_name,
            if p.position.is_empty() { "?" } else { &p.position }));
        if !p.rank_display.is_empty() {
            msg.push_str(&format!("— {} ", p.rank_display));
        }
        if ctx.phase != "champ_select" {
            msg.push_str(&format!("— {}/{}/{} — {} CS — Lv{}",
                p.kills, p.deaths, p.assists, p.cs, p.level));
            if !p.items.is_empty() {
                msg.push_str(&format!(" — Items: {}", p.items.join(", ")));
            }
        }
        msg.push('\n');
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

// ─── SSE streaming endpoint ─────────────────────────────────────────────────

pub async fn stream_coach(
    State(state): State<Arc<AppState>>,
    Json(ctx): Json<CoachingContext>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let config = state.ai_coach_config.clone();
    let system_prompt = build_system_prompt(&ctx.phase);
    let user_message = build_user_message(&ctx);

    Sse::new(make_anthropic_stream(config, system_prompt, user_message))
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
                        text: Some("AI Coach не настроен на сервере (ANTHROPIC_AUTH_TOKEN не задан)".to_string()),
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

        let client = reqwest::Client::new();
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

        let response = client
            .post(&url)
            .header("x-api-key", &config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await;

        let mut response = match response {
            Ok(r) => r,
            Err(e) => {
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

        // Read SSE stream from Anthropic and forward deltas
        let mut buffer = String::new();

        while let Some(chunk) = response.chunk().await.ok().flatten() {
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buffer.find("\n\n") {
                let event_block = buffer[..pos].to_string();
                buffer = buffer[pos + 2..].to_string();

                for line in event_block.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
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
        }

        // Emit end
        yield Ok(Event::default().data(
            serde_json::to_string(&CoachStreamPayload {
                kind: "end".to_string(),
                text: None,
            }).unwrap()
        ));
    }
}
