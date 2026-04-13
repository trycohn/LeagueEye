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
use crate::item_catalog::ItemCatalog;

// ─── System prompt ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatchStage {
    Early,
    Mid,
    Late,
}

const UNKNOWN_STAGE_LABEL_PROMPT: &str = "неизвестная";
const UNKNOWN_STAGE_LABEL_MESSAGE: &str = "неизвестна";
const UNKNOWN_STAGE_FOCUS: &str =
    "оцени текущее состояние игры по KDA, золоту, предметам, событиям и позициям";
const UNKNOWN_STAGE_ANTI_REPEAT_RULE: &str =
    "Не повторяй один и тот же шаблонный совет без новой причины из данных";

fn normalized_game_time_secs(game_time_secs: Option<i64>) -> Option<i64> {
    game_time_secs.filter(|secs| *secs >= 0)
}

fn format_game_time(game_time_secs: Option<i64>) -> String {
    normalized_game_time_secs(game_time_secs)
        .map(|t| format!("{}:{:02}", t / 60, t % 60))
        .unwrap_or_else(|| "?".to_string())
}

impl MatchStage {
    fn from_game_time_secs(game_time_secs: Option<i64>) -> Option<Self> {
        let secs = normalized_game_time_secs(game_time_secs)?;
        if secs < 15 * 60 {
            Some(Self::Early)
        } else if secs < 30 * 60 {
            Some(Self::Mid)
        } else {
            Some(Self::Late)
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Early => "ранняя (early game)",
            Self::Mid => "средняя (mid game)",
            Self::Late => "поздняя (late game)",
        }
    }

    fn focus_summary(self) -> &'static str {
        match self {
            Self::Early => "линия, фарм, трейды, контроль волны, безопасные окна",
            Self::Mid => "роумы, objectives, передвижение по карте, сайдлейны",
            Self::Late => {
                "тимфайты, позиционка, Baron / Dragon Soul / Elder, игра от ключевых кулдаунов и ошибок"
            }
        }
    }

    fn anti_repeat_rule(self) -> &'static str {
        match self {
            Self::Early => {
                "Не повторяй две одинаковые раннегеймовые мысли разными словами"
            }
            Self::Mid => {
                "Не зацикливайся на шаблонных раннегеймовых советах про линию, фарм и башню, если данные не требуют этого прямо сейчас"
            }
            Self::Late => {
                "Не возвращайся к шаблонным раннегеймовым советам про линию и фарм, если данные не требуют этого прямо сейчас"
            }
        }
    }
}

fn build_system_prompt(ctx: &CoachingContext, catalog: Option<&ItemCatalog>) -> String {
    let item_catalog_block = build_item_catalog_block(catalog);

    if ctx.phase == "draft_pick" {
        let pick_order = ctx.draft_pick_order.as_deref().unwrap_or("mid");
        let pick_strategy = match pick_order {
            "early" => "Игрок пикает РАНО (первый/второй). Рекомендуй БЕЗОПАСНЫЕ (safe) и ГИБКИЕ (flex) пики, которые сложно законтрить. НЕ рекомендуй чемпионов, которые легко контрятся без знания вражеского пика",
            "late" => "Игрок пикает ПОЗДНО (последний/предпоследний). Рекомендуй КОНТРПИКИ на основе вражеского драфта. Учитывай слабости вражеских чемпионов",
            _ => "Игрок пикает в середине драфта. Учитывай и безопасность пика, и возможность контрпика на основе уже выбранных врагов",
        };

        let mut prompt = format!(r#"Ты — AI-помощник по драфту в League of Legends. Помогай игроку выбрать лучший пик.

Стратегия пика:
{pick_strategy}

Структура данных:
- Блок «=== Я (игрок) ===» — это твой подопечный
- «Моя команда» — СОЮЗНИКИ. «Вражеская команда» — ПРОТИВНИКИ
- «Забаненные чемпионы» — НЕ рекомендуй их, они недоступны
- «Мой чемпион-пул» — чемпионы, на которых игрок реально играет. Приоритет пикам из пула

Правила:
- Отвечай ТОЛЬКО на русском языке
- ФОРМАТ ОТВЕТА: от 3 до 5 строк, каждая — рекомендация чемпиона
- Каждая строка: «- ИмяЧемпиона — причина (максимум 12 слов)»
- После рекомендаций одна строка: «! Драфту не хватает: [чего]» — ТОЛЬКО если есть явная дыра в командной композиции (нет AP/AD/танка/инициации/и т.д.). Если дыры нет — НЕ пиши эту строку
- НЕ рекомендуй забаненных чемпионов
- НЕ рекомендуй уже взятых чемпионов (из обеих команд)
- Если чемпион из пула игрока подходит — ставь его выше в списке
- Называй чемпионов по ПОЛНЫМ именам (Мордекайзер, Мисс Фортуна, Чо'Гат)
- НИКАКОГО текста кроме рекомендаций"#);
        if !item_catalog_block.is_empty() {
            prompt.push_str(&item_catalog_block);
        }
        return prompt;
    }

    if ctx.phase == "champ_select" {
        let mut prompt = r#"Ты — AI-тренер по League of Legends. Анализируй драфт и давай рекомендации.

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
        if !item_catalog_block.is_empty() {
            prompt.push_str(&item_catalog_block);
        }
        return prompt;
    }

    let stage = MatchStage::from_game_time_secs(ctx.game_time_secs);
    let stage_label = stage
        .map(MatchStage::label)
        .unwrap_or(UNKNOWN_STAGE_LABEL_PROMPT);
    let stage_focus = stage
        .map(MatchStage::focus_summary)
        .unwrap_or(UNKNOWN_STAGE_FOCUS);
    let anti_repeat_rule = stage
        .map(MatchStage::anti_repeat_rule)
        .unwrap_or(UNKNOWN_STAGE_ANTI_REPEAT_RULE);

    let mut prompt = format!(r#"Ты — AI-тренер по League of Legends. Анализируй текущее состояние игры и давай рекомендации.

Структура данных:
- Блок «=== Я (игрок) ===» — это твой подопечный. Его статы (KDA, CS, золото, предметы) указаны в этом блоке — ИСПОЛЬЗУЙ ИХ
- В списке команды он помечен «[Я]»
- «Моя команда» — СОЮЗНИКИ. «Вражеская команда» — ПРОТИВНИКИ. Не путай их
- Когда говоришь про статы игрока — сверяйся с данными из блока «=== Я (игрок) ===»

Правила:
- Отвечай ТОЛЬКО на русском языке
- ФОРМАТ ОТВЕТА: РОВНО 2 строки, каждая начинается с «- » (дефис + пробел). НИЧЕГО больше — никаких заголовков, секций, вступлений, заключений, пояснений, цитат, выделений, приоритетов, анализов
- Каждый совет — максимум 15 слов. Коротко и по делу
- Фокусируйся на самом важном прямо сейчас: KDA, CS, золото, предметы, время игры, стадия матча
- Стадия матча сейчас: {stage_label}
- Приоритет этой стадии: {stage_focus}
- {anti_repeat_rule}
- Советуй конкретно: "Иди на дракона", "Фарми", "Сплитпушь топ", "Не дерись"
- Называй чемпионов по ПОЛНЫМ именам (Мордекайзер, Мисс Фортуна, Чо'Гат)
- НЕ пиши названия умений — используй ТОЛЬКО (Q), (W), (E), (R), (Пассивное)
- НЕ предполагай что у чемпиона мана — ресурс указан в данных
- НИКАКОГО текста кроме двух строк с советами"#);

    if !item_catalog_block.is_empty() {
        prompt.push_str(&item_catalog_block);
    }

    prompt
}

/// Build the item catalog block for the system prompt.
/// Returns empty string if catalog is None or has no items.
fn build_item_catalog_block(catalog: Option<&ItemCatalog>) -> String {
    let catalog = match catalog {
        Some(c) if !c.items.is_empty() => c,
        _ => return String::new(),
    };

    let mut block = String::from("\n\n=== СПРАВОЧНИК ПРЕДМЕТОВ ===\nФормат: РУ (EN) | цена | теги\n");

    for item in &catalog.items {
        let tags = if item.tags.is_empty() {
            String::new()
        } else {
            item.tags.join(",")
        };
        block.push_str(&format!(
            "{} ({}) | {} | {}\n",
            item.ru_name, item.en_name, item.gold_total, tags
        ));
    }

    block.push_str("\n- Когда советуешь купить предмет — ОБЯЗАТЕЛЬНО используй русское название из СПРАВОЧНИКА ПРЕДМЕТОВ\n\
- НЕ переводи названия предметов самостоятельно — бери ТОЛЬКО из справочника\n\
- Если нужен предмет с определённым свойством — ищи по тегам в справочнике (Damage, SpellDamage, Armor, MagicResist, Health, AttackSpeed, CriticalStrike, AbilityHaste и т.д.)\n\
- Учитывай стоимость предмета и текущее золото игрока");

    block
}

// ─── Build user message ─────────────────────────────────────────────────────

fn build_user_message(ctx: &CoachingContext, catalog: Option<&ItemCatalog>) -> String {
    let mut msg = String::new();

    if ctx.phase == "draft_pick" {
        let pick_order = ctx.draft_pick_order.as_deref().unwrap_or("mid");
        let pick_label = match pick_order {
            "early" => "Ранний пик (первый/второй)",
            "late" => "Поздний пик (последний/предпоследний)",
            _ => "Средний пик",
        };
        msg.push_str(&format!("Фаза: Помощь с пиком\nПозиция в драфте: {}\n", pick_label));

        // My role
        if !ctx.my_position.is_empty() {
            msg.push_str(&format!("Моя роль: {}\n", ctx.my_position));
        }

        // Bans
        if !ctx.banned_champions.is_empty() {
            msg.push_str(&format!("\nЗабаненные чемпионы: {}\n", ctx.banned_champions.join(", ")));
        }

        // My champion pool
        if !ctx.my_champion_pool.is_empty() {
            msg.push_str("\nМой чемпион-пул (на ком я играю):\n");
            for entry in &ctx.my_champion_pool {
                msg.push_str(&format!("- {} — {} игр, {}% WR\n", entry.champion_name, entry.games, entry.winrate));
            }
        }

        // Already picked allies
        let picked_allies: Vec<&CoachPlayerInfo> = ctx.my_team.iter()
            .filter(|p| !p.champion_name.is_empty() && !p.champion_name.starts_with("ChampID:"))
            .collect();
        if !picked_allies.is_empty() {
            msg.push_str("\nМоя команда (уже выбраны):\n");
            for p in &picked_allies {
                msg.push_str(&format!("- {} ({})", p.champion_name,
                    if p.position.is_empty() { "?" } else { &p.position }));
                if let Some(ref class) = p.champion_class {
                    msg.push_str(&format!(" — {}", class));
                }
                if !p.rank_display.is_empty() {
                    msg.push_str(&format!(" — {}", p.rank_display));
                }
                msg.push('\n');
            }
        }

        // Already picked enemies
        let picked_enemies: Vec<&CoachPlayerInfo> = ctx.enemy_team.iter()
            .filter(|p| !p.champion_name.is_empty() && !p.champion_name.starts_with("ChampID:"))
            .collect();
        if !picked_enemies.is_empty() {
            msg.push_str("\nВражеская команда (уже выбраны):\n");
            for p in &picked_enemies {
                msg.push_str(&format!("- {} ({})", p.champion_name,
                    if p.position.is_empty() { "?" } else { &p.position }));
                if let Some(ref class) = p.champion_class {
                    msg.push_str(&format!(" — {}", class));
                }
                msg.push('\n');
            }
        }

        msg.push_str("\nПорекомендуй мне лучшие пики для текущего драфта.");
        return msg;
    }

    if ctx.phase == "champ_select" {
        msg.push_str("Фаза: Выбор чемпионов\n");
    } else {
        let time_str = format_game_time(ctx.game_time_secs);
        msg.push_str(&format!("Фаза: В игре ({})\n", time_str));
        if let Some(stage) = MatchStage::from_game_time_secs(ctx.game_time_secs) {
            msg.push_str(&format!("Стадия матча: {}\n", stage.label()));
            msg.push_str(&format!("Фокус стадии: {}\n", stage.focus_summary()));
        } else {
            msg.push_str(&format!("Стадия матча: {}\n", UNKNOWN_STAGE_LABEL_MESSAGE));
            msg.push_str(&format!("Фокус стадии: {}\n", UNKNOWN_STAGE_FOCUS));
        }
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
                let items_str = format_items_for_player(&me.items, catalog, true);
                msg.push_str(&format!("Предметы: {}\n", items_str));
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
        write_player_line(&mut msg, p, marker, ctx.phase == "champ_select", catalog);
    }

    msg.push_str("\nВражеская команда (ПРОТИВНИКИ):\n");
    for p in &ctx.enemy_team {
        write_player_line(&mut msg, p, "", ctx.phase == "champ_select", catalog);
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

/// Format a list of item display_names using the catalog for translation.
/// For `detailed` mode (the player's own block), includes gold + tags as hashtags.
/// For team listing, includes only gold.
fn format_items_for_player(items: &[String], catalog: Option<&ItemCatalog>, detailed: bool) -> String {
    items
        .iter()
        .map(|en_name| format_single_item(en_name, catalog, detailed))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_single_item(en_name: &str, catalog: Option<&ItemCatalog>, detailed: bool) -> String {
    let catalog = match catalog {
        Some(c) => c,
        None => return en_name.to_string(),
    };

    let ru_name = catalog
        .en_to_ru
        .get(en_name)
        .cloned()
        .unwrap_or_else(|| en_name.to_string());
    let gold = catalog.en_to_gold.get(en_name).copied();
    let tags = catalog.en_to_tags.get(en_name);

    if detailed {
        // "Спутник Людена (2850g #SpellDamage #Mana)"
        let mut parts = ru_name.clone();
        let mut meta = Vec::new();
        if let Some(g) = gold {
            meta.push(format!("{}g", g));
        }
        if let Some(t) = tags {
            for tag in t {
                meta.push(format!("#{}", tag));
            }
        }
        if !meta.is_empty() {
            parts.push_str(&format!(" ({})", meta.join(" ")));
        }
        parts
    } else {
        // "Спутник Людена (2850g)"
        match gold {
            Some(g) if g > 0 => format!("{} ({}g)", ru_name, g),
            _ => ru_name,
        }
    }
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

fn write_player_line(msg: &mut String, p: &CoachPlayerInfo, marker: &str, is_champ_select: bool, catalog: Option<&ItemCatalog>) {
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
            let items_str = format_items_for_player(&p.items, catalog, false);
            msg.push_str(&format!(" — Items: {}", items_str));
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

    // Load item catalog (lazy, cached in OnceCell)
    let catalog = state
        .item_catalog
        .get_or_try_init(|| async {
            crate::item_catalog::load_item_catalog()
                .await
                .map_err(|e| {
                    log::error!("[coach] Failed to load item catalog: {}", e);
                    e
                })
        })
        .await
        .ok();

    let system_prompt = build_system_prompt(&ctx, catalog);
    let user_message = build_user_message(&ctx, catalog);

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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_player(champion_name: &str, position: &str) -> CoachPlayerInfo {
        CoachPlayerInfo {
            champion_name: champion_name.to_string(),
            position: position.to_string(),
            rank_display: "Gold I".to_string(),
            kills: 3,
            deaths: 1,
            assists: 4,
            cs: 156,
            level: 12,
            items: vec!["Lost Chapter".to_string()],
            summoner_spells: vec!["Flash".to_string(), "Ignite".to_string()],
            keystone_rune: "Electrocute".to_string(),
            is_dead: false,
            respawn_timer: 0.0,
            champion_resource: Some("Mana".to_string()),
            champion_class: Some("Mage".to_string()),
        }
    }

    fn sample_context(game_time_secs: Option<i64>) -> CoachingContext {
        CoachingContext {
            phase: "in_game".to_string(),
            game_time_secs,
            my_champion: "Ahri".to_string(),
            my_position: "MIDDLE".to_string(),
            my_gold: Some(1250.0),
            my_summoner_spells: vec!["Flash".to_string(), "Ignite".to_string()],
            my_runes: Some("Electrocute (Domination / Sorcery)".to_string()),
            my_stats: None,
            my_team: vec![sample_player("Ahri", "MIDDLE")],
            enemy_team: vec![sample_player("Zed", "MIDDLE")],
            recent_events: vec![],
            my_champion_resource: Some("Mana".to_string()),
            my_champion_class: Some("Mage".to_string()),
            my_champion_abilities_summary: None,
            my_champion_ally_tips: None,
            draft_pick_order: None,
            banned_champions: vec![],
            my_champion_pool: vec![],
        }
    }

    #[test]
    fn match_stage_uses_expected_thresholds() {
        assert_eq!(MatchStage::from_game_time_secs(None), None);
        assert_eq!(MatchStage::from_game_time_secs(Some(-1)), None);
        assert_eq!(MatchStage::from_game_time_secs(Some(0)), Some(MatchStage::Early));
        assert_eq!(MatchStage::from_game_time_secs(Some(14 * 60 + 59)), Some(MatchStage::Early));
        assert_eq!(MatchStage::from_game_time_secs(Some(15 * 60)), Some(MatchStage::Mid));
        assert_eq!(MatchStage::from_game_time_secs(Some(29 * 60 + 59)), Some(MatchStage::Mid));
        assert_eq!(MatchStage::from_game_time_secs(Some(30 * 60)), Some(MatchStage::Late));
    }

    #[test]
    fn user_message_includes_match_stage_and_focus() {
        let message = build_user_message(&sample_context(Some(20 * 60)), None);

        assert!(message.contains("Фаза: В игре (20:00)"));
        assert!(message.contains("Стадия матча: средняя (mid game)"));
        assert!(message.contains("Фокус стадии: роумы, objectives, передвижение по карте, сайдлейны"));
    }

    #[test]
    fn user_message_uses_unknown_stage_fallback_when_time_missing() {
        let message = build_user_message(&sample_context(None), None);

        assert!(message.contains("Фаза: В игре (?)"));
        assert!(message.contains("Стадия матча: неизвестна"));
        assert!(message.contains(
            "Фокус стадии: оцени текущее состояние игры по KDA, золоту, предметам, событиям и позициям"
        ));
    }

    #[test]
    fn system_prompt_uses_stage_specific_focus_and_anti_repeat_rule() {
        let prompt = build_system_prompt(&sample_context(Some(31 * 60)), None);

        assert!(prompt.contains("Стадия матча сейчас: поздняя (late game)"));
        assert!(prompt.contains(
            "Приоритет этой стадии: тимфайты, позиционка, Baron / Dragon Soul / Elder, игра от ключевых кулдаунов и ошибок"
        ));
        assert!(prompt.contains(
            "Не возвращайся к шаблонным раннегеймовым советам про линию и фарм"
        ));
    }

    #[test]
    fn system_prompt_uses_neutral_fallback_when_stage_unknown() {
        let prompt = build_system_prompt(&sample_context(None), None);

        assert!(prompt.contains("Стадия матча сейчас: неизвестная"));
        assert!(prompt.contains(
            "Приоритет этой стадии: оцени текущее состояние игры по KDA, золоту, предметам, событиям и позициям"
        ));
        assert!(prompt.contains(
            "Не повторяй один и тот же шаблонный совет без новой причины из данных"
        ));
    }

    #[test]
    fn champ_select_prompt_does_not_include_match_stage_rules() {
        let mut ctx = sample_context(Some(20 * 60));
        ctx.phase = "champ_select".to_string();

        let prompt = build_system_prompt(&ctx, None);

        assert!(prompt.contains("Анализируй драфт и давай рекомендации"));
        assert!(!prompt.contains("Стадия матча сейчас:"));
        assert!(!prompt.contains("{stage_label}"));
    }

    // ── Item catalog integration tests ──────────────────────────────────

    fn sample_catalog() -> ItemCatalog {
        use std::collections::HashMap;

        let items = vec![
            crate::item_catalog::CatalogItem {
                id: 3031,
                en_name: "Infinity Edge".to_string(),
                ru_name: "\u{0413}\u{0440}\u{0430}\u{043d}\u{044c} \u{0411}\u{0435}\u{0441}\u{043a}\u{043e}\u{043d}\u{0435}\u{0447}\u{043d}\u{043e}\u{0441}\u{0442}\u{0438}".to_string(),
                gold_total: 3400,
                tags: vec!["Damage".to_string(), "CriticalStrike".to_string()],
            },
            crate::item_catalog::CatalogItem {
                id: 2900,
                en_name: "Spirit Visage".to_string(),
                ru_name: "\u{041e}\u{0431}\u{043b}\u{0430}\u{0447}\u{0435}\u{043d}\u{0438}\u{0435} \u{0414}\u{0443}\u{0445}\u{0430}".to_string(),
                gold_total: 2900,
                tags: vec!["Health".to_string(), "SpellBlock".to_string()],
            },
        ];

        let mut en_to_ru = HashMap::new();
        let mut en_to_gold = HashMap::new();
        let mut en_to_tags = HashMap::new();

        // Include items + a component item "Lost Chapter"
        for item in &items {
            en_to_ru.insert(item.en_name.clone(), item.ru_name.clone());
            en_to_gold.insert(item.en_name.clone(), item.gold_total);
            en_to_tags.insert(item.en_name.clone(), item.tags.clone());
        }
        en_to_ru.insert("Lost Chapter".to_string(), "\u{041f}\u{043e}\u{0442}\u{0435}\u{0440}\u{044f}\u{043d}\u{043d}\u{0430}\u{044f} \u{0413}\u{043b}\u{0430}\u{0432}\u{0430}".to_string());
        en_to_gold.insert("Lost Chapter".to_string(), 1300);
        en_to_tags.insert("Lost Chapter".to_string(), vec!["SpellDamage".to_string(), "Mana".to_string()]);

        ItemCatalog {
            items,
            en_to_ru,
            en_to_gold,
            en_to_tags,
        }
    }

    #[test]
    fn system_prompt_includes_item_catalog_when_present() {
        let catalog = sample_catalog();
        let prompt = build_system_prompt(&sample_context(Some(20 * 60)), Some(&catalog));

        assert!(prompt.contains("=== СПРАВОЧНИК ПРЕДМЕТОВ ==="));
        assert!(prompt.contains("\u{0413}\u{0440}\u{0430}\u{043d}\u{044c} \u{0411}\u{0435}\u{0441}\u{043a}\u{043e}\u{043d}\u{0435}\u{0447}\u{043d}\u{043e}\u{0441}\u{0442}\u{0438} (Infinity Edge) | 3400 | Damage,CriticalStrike"));
        assert!(prompt.contains("ОБЯЗАТЕЛЬНО используй русское название из СПРАВОЧНИКА ПРЕДМЕТОВ"));
    }

    #[test]
    fn system_prompt_without_catalog_has_no_item_block() {
        let prompt = build_system_prompt(&sample_context(Some(20 * 60)), None);

        assert!(!prompt.contains("СПРАВОЧНИК ПРЕДМЕТОВ"));
    }

    #[test]
    fn champ_select_prompt_includes_catalog_too() {
        let catalog = sample_catalog();
        let mut ctx = sample_context(Some(20 * 60));
        ctx.phase = "champ_select".to_string();

        let prompt = build_system_prompt(&ctx, Some(&catalog));

        assert!(prompt.contains("=== СПРАВОЧНИК ПРЕДМЕТОВ ==="));
        assert!(prompt.contains("Анализируй драфт и давай рекомендации"));
    }

    #[test]
    fn user_message_translates_items_with_catalog() {
        let catalog = sample_catalog();
        let message = build_user_message(&sample_context(Some(20 * 60)), Some(&catalog));

        // Player's own items (detailed format with gold + tags)
        assert!(message.contains("\u{041f}\u{043e}\u{0442}\u{0435}\u{0440}\u{044f}\u{043d}\u{043d}\u{0430}\u{044f} \u{0413}\u{043b}\u{0430}\u{0432}\u{0430} (1300g #SpellDamage #Mana)"));

        // Team listing (short format with gold only)
        assert!(message.contains("\u{041f}\u{043e}\u{0442}\u{0435}\u{0440}\u{044f}\u{043d}\u{043d}\u{0430}\u{044f} \u{0413}\u{043b}\u{0430}\u{0432}\u{0430} (1300g)"));
    }

    #[test]
    fn user_message_keeps_en_name_when_not_in_catalog() {
        let catalog = sample_catalog();
        let mut ctx = sample_context(Some(20 * 60));
        ctx.my_team[0].items = vec!["Unknown New Item".to_string()];
        ctx.enemy_team[0].items = vec!["Unknown New Item".to_string()];

        let message = build_user_message(&ctx, Some(&catalog));

        assert!(message.contains("Unknown New Item"));
    }

    #[test]
    fn user_message_without_catalog_uses_en_names() {
        let message = build_user_message(&sample_context(Some(20 * 60)), None);

        assert!(message.contains("Lost Chapter"));
        assert!(!message.contains("\u{041f}\u{043e}\u{0442}\u{0435}\u{0440}\u{044f}\u{043d}\u{043d}\u{0430}\u{044f}"));
    }

    #[test]
    fn format_items_empty_list() {
        let result = format_items_for_player(&[], Some(&sample_catalog()), true);
        assert_eq!(result, "");
    }

    // ── Draft Helper tests ──────────────────────────────────────────────

    fn sample_draft_context(pick_order: &str) -> CoachingContext {
        use leagueeye_shared::models::ChampionPoolEntry;

        CoachingContext {
            phase: "draft_pick".to_string(),
            game_time_secs: None,
            my_champion: String::new(),
            my_position: "top".to_string(),
            my_gold: None,
            my_summoner_spells: vec![],
            my_runes: None,
            my_stats: None,
            my_team: vec![
                sample_player("Jinx", "bottom"),
                sample_player("Thresh", "utility"),
            ],
            enemy_team: vec![
                sample_player("Zed", "MIDDLE"),
                sample_player("LeeSin", "jungle"),
            ],
            recent_events: vec![],
            my_champion_resource: None,
            my_champion_class: None,
            my_champion_abilities_summary: None,
            my_champion_ally_tips: None,
            draft_pick_order: Some(pick_order.to_string()),
            banned_champions: vec!["Darius".to_string(), "Yasuo".to_string(), "Yone".to_string()],
            my_champion_pool: vec![
                ChampionPoolEntry { champion_name: "Mordekaiser".to_string(), games: 25, winrate: 64.0 },
                ChampionPoolEntry { champion_name: "Sett".to_string(), games: 15, winrate: 53.3 },
                ChampionPoolEntry { champion_name: "Renekton".to_string(), games: 10, winrate: 50.0 },
            ],
        }
    }

    #[test]
    fn draft_pick_prompt_recommends_safe_picks_for_early_picker() {
        let ctx = sample_draft_context("early");
        let prompt = build_system_prompt(&ctx, None);

        assert!(prompt.contains("AI-помощник по драфту"));
        assert!(prompt.contains("БЕЗОПАСНЫЕ (safe)"));
        assert!(prompt.contains("ГИБКИЕ (flex)"));
        assert!(!prompt.contains("КОНТРПИКИ"));
    }

    #[test]
    fn draft_pick_prompt_recommends_counterpicks_for_late_picker() {
        let ctx = sample_draft_context("late");
        let prompt = build_system_prompt(&ctx, None);

        assert!(prompt.contains("AI-помощник по драфту"));
        assert!(prompt.contains("КОНТРПИКИ"));
        assert!(!prompt.contains("БЕЗОПАСНЫЕ (safe)"));
    }

    #[test]
    fn draft_pick_user_message_includes_bans_and_picks() {
        let ctx = sample_draft_context("late");
        let message = build_user_message(&ctx, None);

        assert!(message.contains("Фаза: Помощь с пиком"));
        assert!(message.contains("Поздний пик"));
        assert!(message.contains("Моя роль: top"));
        assert!(message.contains("Забаненные чемпионы: Darius, Yasuo, Yone"));
        assert!(message.contains("Вражеская команда (уже выбраны):"));
        assert!(message.contains("Zed"));
        assert!(message.contains("LeeSin"));
    }

    #[test]
    fn draft_pick_user_message_includes_champion_pool() {
        let ctx = sample_draft_context("early");
        let message = build_user_message(&ctx, None);

        assert!(message.contains("Мой чемпион-пул"));
        assert!(message.contains("Mordekaiser"));
        assert!(message.contains("25 игр"));
        assert!(message.contains("64%"));
        assert!(message.contains("Sett"));
    }

    #[test]
    fn draft_pick_prompt_does_not_include_match_stage() {
        let ctx = sample_draft_context("mid");
        let prompt = build_system_prompt(&ctx, None);

        assert!(!prompt.contains("Стадия матча сейчас:"));
        assert!(!prompt.contains("early game"));
        assert!(!prompt.contains("mid game"));
        assert!(prompt.contains("середине драфта"));
    }
}
