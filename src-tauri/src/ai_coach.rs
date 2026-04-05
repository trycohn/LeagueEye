use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use tauri::{AppHandle, Emitter};

use crate::lcu;

// ─── Config & State ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AiCoachConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
}

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

// ─── Event payload sent to frontend ──────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CoachStreamPayload {
    pub kind: String, // "start" | "delta" | "end" | "error"
    pub text: Option<String>,
}

// ─── Coaching context ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Hash)]
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
}

#[derive(Debug, Clone)]
pub struct CoachingContext {
    pub phase: String,
    pub game_time_secs: Option<i64>,
    pub my_champion: String,
    pub my_position: String,
    pub my_team: Vec<CoachPlayerInfo>,
    pub enemy_team: Vec<CoachPlayerInfo>,
    pub recent_events: Vec<String>,
}

impl CoachingContext {
    pub fn compute_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.phase.hash(&mut hasher);
        // Round game time to 30s buckets to avoid hash churn
        if let Some(t) = self.game_time_secs {
            (t / 30).hash(&mut hasher);
        }
        for p in &self.my_team {
            p.hash(&mut hasher);
        }
        for p in &self.enemy_team {
            p.hash(&mut hasher);
        }
        hasher.finish()
    }
}

// ─── Build coaching context from live data ───────────────────────────────────

pub fn build_context_from_allgamedata(
    alldata: &lcu::LiveAllGameData,
    my_name: &str,
) -> Option<CoachingContext> {
    let players = alldata.all_players.as_ref()?;
    let game_info = alldata.game_data.as_ref();

    let game_time = game_info.and_then(|g| g.game_time.map(|t| t as i64));

    // Determine which team "I" am on
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
            rank_display: String::new(), // filled later from LivePlayer data
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

    // Recent events (last 5)
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

pub fn build_context_champ_select(
    my_team: &[crate::models::LivePlayer],
    enemy_team: &[crate::models::LivePlayer],
    champ_names: &std::collections::HashMap<i64, String>,
    my_puuid: Option<&str>,
) -> CoachingContext {
    let to_info = |p: &crate::models::LivePlayer| -> CoachPlayerInfo {
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

// ─── System prompt ───────────────────────────────────────────────────────────

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

// ─── Build user message ──────────────────────────────────────────────────────

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

// ─── Streaming LLM call ─────────────────────────────────────────────────────

pub async fn stream_coaching_advice(
    app: AppHandle,
    ctx: CoachingContext,
    config: &AiCoachConfig,
) -> Result<(), String> {
    let system_prompt = build_system_prompt(&ctx.phase);
    let user_message = build_user_message(&ctx);

    let _ = app.emit("coach-stream", CoachStreamPayload {
        kind: "start".to_string(),
        text: None,
    });

    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "model": config.model,
        "max_tokens": config.max_tokens,
        "stream": true,
        "system": system_prompt,
        "messages": [
            {
                "role": "user",
                "content": user_message
            }
        ]
    });

    let url = format!("{}/v1/messages", config.base_url.trim_end_matches('/'));

    let response = client
        .post(&url)
        .header("x-api-key", &config.api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            let msg = format!("Ошибка соединения с AI: {}", e);
            let _ = app.emit("coach-stream", CoachStreamPayload {
                kind: "error".to_string(),
                text: Some(msg.clone()),
            });
            msg
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();
        let msg = if status.as_u16() == 401 {
            "Неверный API ключ. Проверьте ANTHROPIC_AUTH_TOKEN.".to_string()
        } else {
            format!("AI API ошибка ({}): {}", status, body_text)
        };
        let _ = app.emit("coach-stream", CoachStreamPayload {
            kind: "error".to_string(),
            text: Some(msg.clone()),
        });
        return Err(msg);
    }

    // Read SSE stream using chunk-by-chunk approach
    let mut buffer = String::new();
    let mut response = response;

    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        // Process complete SSE lines from buffer
        while let Some(pos) = buffer.find("\n\n") {
            let event_block = buffer[..pos].to_string();
            buffer = buffer[pos + 2..].to_string();

            for line in event_block.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        continue;
                    }
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                        // Handle content_block_delta events
                        if parsed.get("type").and_then(|t| t.as_str()) == Some("content_block_delta") {
                            if let Some(text) = parsed
                                .get("delta")
                                .and_then(|d| d.get("text"))
                                .and_then(|t| t.as_str())
                            {
                                let _ = app.emit("coach-stream", CoachStreamPayload {
                                    kind: "delta".to_string(),
                                    text: Some(text.to_string()),
                                });
                            }
                        }
                        // Handle error events
                        if parsed.get("type").and_then(|t| t.as_str()) == Some("error") {
                            let err_msg = parsed.get("error")
                                .and_then(|e| e.get("message"))
                                .and_then(|m| m.as_str())
                                .unwrap_or("Unknown AI error");
                            let _ = app.emit("coach-stream", CoachStreamPayload {
                                kind: "error".to_string(),
                                text: Some(err_msg.to_string()),
                            });
                            return Err(err_msg.to_string());
                        }
                    }
                }
            }
        }
    }

    let _ = app.emit("coach-stream", CoachStreamPayload {
        kind: "end".to_string(),
        text: None,
    });

    Ok(())
}
