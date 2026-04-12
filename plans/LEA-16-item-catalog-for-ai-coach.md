# LEA-16: Каталог предметов для AI Coach

**Status:** completed
**Issue:** [LEA-16](https://linear.app/leagueeye/issue/LEA-16/dobavit-helper-dlya-korrektnyh-russkih-nazvanij-predmetov-v-sovetah-ai)
**Priority:** High
**Branch:** `feat/lea-16-item-catalog-ai-coach`

---

## Проблема

AI Coach имеет две проблемы с предметами:

1. **Входящие данные** — предметы игроков приходят из LCU API с английскими `display_name` (например "Infinity Edge"). AI может перевести криво ("Бесконечный Клинок" вместо правильного "Грань Бесконечности").

2. **Исходящие советы** — когда AI хочет посоветовать купить предмет (например "купи магическую защиту"), она выдумывает название, потому что не знает точный русский каталог предметов League of Legends.

## Решение

Загружать DDragon `item.json` (en_US + ru_RU) при старте сервера, строить компактный каталог финальных предметов и встраивать его в системный промпт AI. Также заменять английские названия предметов на русские в user message.

---

## Scope Boundaries (не в скоупе)

- Каталог чемпионов/способностей на русском — отдельный тикет
- Рекомендации "оптимального билда" — AI сама решает, что советовать
- Кэш на диске / в БД — достаточно in-memory кэша, живёт пока работает сервер
- Фильтрация каталога по классу чемпиона / ситуации — отдаём полный каталог, AI сама разберётся

---

## Implementation Units

### Unit 1: `ItemCatalog` — загрузка и кэширование на сервере

**Goal:** Создать структуру `ItemCatalog` на сервере, которая загружает DDragon item.json (en_US + ru_RU) при первом запросе коучинга и кэширует в `AppState`.

**Files:**
- `server/src/item_catalog.rs` (новый файл)
- `server/src/main.rs` (добавить `ItemCatalog` в `AppState`)

**Approach:**

1. Создать структуры:

```rust
// server/src/item_catalog.rs

pub struct CatalogItem {
    pub id: i32,
    pub en_name: String,
    pub ru_name: String,
    pub gold_total: i32,
    pub tags: Vec<String>,
}

pub struct ItemCatalog {
    /// Финальные предметы для каталога AI (en_name → CatalogItem)
    pub items: Vec<CatalogItem>,
    /// EN display_name → RU name (для замены в user message, все предметы включая компоненты)
    pub en_to_ru: HashMap<String, String>,
    /// EN display_name → gold total (для добавления стоимости в user message)
    pub en_to_gold: HashMap<String, i32>,
    /// EN display_name → tags
    pub en_to_tags: HashMap<String, Vec<String>>,
}
```

2. Функция загрузки:

```rust
pub async fn load_item_catalog() -> Result<ItemCatalog, String>
```

- Загрузить `https://ddragon.leagueoflegends.com/cdn/{VERSION}/data/en_US/item.json`
- Загрузить `https://ddragon.leagueoflegends.com/cdn/{VERSION}/data/ru_RU/item.json`
- Построить `en_to_ru` маппинг: ключ = EN `name`, значение = RU `name` (для ВСЕХ предметов, включая компоненты, чтобы замена в user message работала)
- Построить `en_to_gold` и `en_to_tags` маппинги аналогично
- Для каталога AI (поле `items`) фильтровать:
  - `maps["11"] == true` (Summoner's Rift)
  - `gold.purchasable == true`
  - `into` пустой или отсутствует (финальные предметы)
  - `gold.total > 0` (исключить тринкеты)
  - Не содержит тег `Consumable` или `Trinket`
- Результат: ~140-155 финальных предметов

3. Кэш в `AppState`:

```rust
// server/src/main.rs
pub struct AppState {
    pub riot_api: RiotApiClient,
    pub db: Db,
    pub ai_coach_config: Option<AiCoachConfig>,
    pub item_catalog: tokio::sync::OnceCell<ItemCatalog>, // новое поле
}
```

Использовать `tokio::sync::OnceCell` — загрузка один раз при первом запросе коучинга, потом из кэша.

**Patterns to follow:**
- `src-tauri/src/commands.rs:894-935` (`get_or_fetch_item_costs`) — аналогичный паттерн загрузки DDragon и парсинга JSON
- `src-tauri/src/commands.rs:648-735` (`get_or_fetch_champion_names`) — паттерн загрузки двух локалей и маппинга

**Execution note:** Тесты для фильтрации: написать unit-тест с мок-данными (5-10 предметов: финальный, компонент, тринкет, расходник) чтобы проверить правильность фильтра.

**Test scenarios:**
- Happy path: загрузка 2 JSON, фильтрация, построение маппингов
- Edge case: предмет без `into` поля vs пустой `into: []`
- Edge case: предмет с `gold.total == 0` отсеивается
- Edge case: предмет с тегом `Consumable` отсеивается
- Error path: если DDragon недоступен — возвращается пустой каталог, AI продолжает работать без каталога

**Verification:** `cargo check -p leagueeye-server` проходит. Unit-тест фильтрации проходит.

---

### Unit 2: Интеграция каталога в системный промпт

**Goal:** Добавить блок `=== СПРАВОЧНИК ПРЕДМЕТОВ ===` в системный промпт AI, чтобы она могла искать предметы по тегам и использовать правильные русские названия.

**Files:**
- `server/src/routes/coach.rs` — `build_system_prompt()` и обработчик `stream_coaching()`

**Approach:**

1. В `build_system_prompt()` добавить параметр `catalog: Option<&ItemCatalog>`.

2. Генерировать компактный блок каталога:

```
=== СПРАВОЧНИК ПРЕДМЕТОВ ===
Формат: РУ_название (EN_название) | цена | теги
Грань Бесконечности (Infinity Edge) | 3450 | Damage,CriticalStrike
Облачение Духа (Spirit Visage) | 2900 | Health,MagicResist,HealthRegen
...
```

3. Добавить в системный промпт правило:

```
- Когда советуешь купить предмет — ОБЯЗАТЕЛЬНО используй русское название из СПРАВОЧНИКА ПРЕДМЕТОВ
- НЕ переводи названия предметов самостоятельно — бери ТОЛЬКО из справочника
- Если нужен предмет с определённым свойством — ищи по тегам в справочнике (Damage, SpellDamage, Armor, MagicResist, Health, AttackSpeed, CriticalStrike, AbilityHaste и т.д.)
- Учитывай стоимость предмета и текущее золото игрока
```

4. В обработчике `stream_coaching()` загружать каталог из `AppState` (через `OnceCell`) и передавать в `build_system_prompt()`.

5. Если каталог не загрузился — промпт работает как раньше, без блока справочника.

**Patterns to follow:**
- `server/src/routes/coach.rs:85-137` — текущий `build_system_prompt()`, формат и стиль
- `server/src/routes/coach.rs:316-330` — текущий обработчик, как достаётся `AppState`

**Execution note:** Промпт должен оставаться компактным. Формат каталога — одна строка на предмет, без лишних пробелов. ~140 предметов × ~60 символов ≈ 8.5KB ≈ 2000-2500 токенов.

**Test scenarios:**
- Happy path: каталог есть — промпт содержит блок справочника
- Edge case: каталог пустой (DDragon недоступен) — промпт без справочника, всё работает как раньше
- Verification: проверить что каталог не дублируется между champ_select и in_game промптами

**Verification:** `cargo check -p leagueeye-server` проходит. Промпт содержит справочник. Без каталога промпт идентичен текущему.

---

### Unit 3: Замена английских названий предметов в user message

**Goal:** В `build_user_message()` заменять английские display_name предметов на русские названия с тегами и стоимостью.

**Files:**
- `server/src/routes/coach.rs` — `build_user_message()` и `write_player_line()`

**Approach:**

1. В `build_user_message()` добавить параметр `catalog: Option<&ItemCatalog>`.

2. В блоке `=== Я (игрок) ===` (строка 202-204) заменить:
   - Было: `Предметы: Lost Chapter, Doran's Ring`
   - Стало: `Предметы: Потерянная Глава (1300g #SpellDamage #Mana), Кольцо Дорана (400g #SpellDamage)`

3. В `write_player_line()` аналогично заменять названия предметов для всех игроков.

4. Если предмет не найден в маппинге (новый предмет, баг) — оставлять оригинальное EN-название.

**Patterns to follow:**
- `server/src/routes/coach.rs:202-204` — текущий вывод предметов игрока
- `server/src/routes/coach.rs:269-310` — `write_player_line()`

**Test scenarios:**
- Happy path: EN-название заменяется на RU + стоимость + теги
- Edge case: предмет не найден в маппинге — оставляется как есть
- Edge case: пустой список предметов — ничего не выводится
- Edge case: предмет-компонент (не финальный) — всё равно переводится (en_to_ru содержит все предметы)

**Verification:** `cargo check -p leagueeye-server` проходит. В user message предметы отображаются на русском.

---

### Unit 4: Константа версии DDragon и подключение модуля

**Goal:** Вынести версию DDragon в единую константу на сервере и правильно подключить новый модуль.

**Files:**
- `server/src/main.rs` — `mod item_catalog;`, добавить `ItemCatalog` в `AppState`
- `server/src/item_catalog.rs` — использовать константу версии

**Approach:**

1. Определить константу в `item_catalog.rs`:
```rust
const DDRAGON_VERSION: &str = "16.7.1";
```

Примечание: в клиенте версия хардкожена в 4 местах. На сервере пока DDragon не используется, так что начинаем чисто. В будущем можно сделать авто-определение версии через `https://ddragon.leagueoflegends.com/api/versions.json`.

2. В `main.rs`:
```rust
mod item_catalog;
// ...
pub struct AppState {
    pub riot_api: RiotApiClient,
    pub db: Db,
    pub ai_coach_config: Option<AiCoachConfig>,
    pub item_catalog: tokio::sync::OnceCell<item_catalog::ItemCatalog>,
}
```

3. Инициализировать `OnceCell::new()` при создании `AppState`.

**Patterns to follow:**
- `server/src/main.rs:30-34` — текущий `AppState`
- `server/src/main.rs:1-10` — текущие `mod` декларации

**Verification:** `cargo check -p leagueeye-server` проходит. Сервер запускается без ошибок.

---

## Dependency Graph

```
Unit 4 (модуль + AppState)
  └── Unit 1 (ItemCatalog загрузка)
        ├── Unit 2 (каталог в системный промпт)
        └── Unit 3 (замена в user message)
```

Порядок реализации: 4 → 1 → 2 → 3 (или 2 и 3 параллельно после 1).

---

## Оценка токенов

| Компонент | Токены |
|-----------|--------|
| Текущий системный промпт | ~300-400 |
| Блок справочника (~140 предметов) | ~2000-2500 |
| User message (без изменений) | ~500-800 |
| User message (добавка от тегов/стоимости) | +200-300 |
| **Итого сверху** | **~2200-2800** |

При стоимости ~$3/1M input tokens (Claude Sonnet) это +$0.007-0.008 за запрос. Приемлемо.

---

## Формат каталога в промпте

```
=== СПРАВОЧНИК ПРЕДМЕТОВ ===
Формат: РУ (EN) | цена | теги

Грань Бесконечности (Infinity Edge) | 3450 | Damage,CriticalStrike
Клинок Бесконечности Погибели (Blade of The Ruined King) | 3200 | Damage,AttackSpeed,LifeSteal
Облачение Духа (Spirit Visage) | 2900 | Health,SpellBlock,HealthRegen
Песочные Часы Жони (Zhonya's Hourglass) | 3250 | SpellDamage,Armor
Спутник Людена (Luden's Companion) | 2850 | SpellDamage,Mana,AbilityHaste
Ботинки Берсерка (Berserker's Greaves) | 1100 | Boots,AttackSpeed
...
```

---

## Формат предметов в user message

```
=== Я (игрок) ===
Чемпион: Ahri (MID)
...
Предметы: Спутник Людена (2850g #SpellDamage #Mana), Песочные Часы Жони (3250g #SpellDamage #Armor), Ботинки Колдуна (1100g #Boots #MagicPenetration)

Моя команда (СОЮЗНИКИ):
[Я] Ahri MID | ... | Items: Спутник Людена (2850g), Песочные Часы Жони (3250g)
Jinx BOT | ... | Items: Грань Бесконечности (3450g), Ураганный Лук Рунаан (2800g)
```

---

## Fallback при ошибках

- DDragon недоступен → `OnceCell` не инициализируется, каталог = `None`
- Каталог = `None` → промпт без справочника, user message без замены — поведение идентично текущему
- Предмет не найден в маппинге → оставляется оригинальное EN-название
- Один из двух JSON (en/ru) не загрузился → используем то что загрузилось, для отсутствующей локали используем EN-названия

---

## Риски и митигации

| Риск | Вероятность | Митигация |
|------|-------------|-----------|
| DDragon меняет формат JSON | Низкая | Парсинг через serde_json::Value, не жёсткие типы. Graceful fallback |
| Слишком много токенов для маленьких моделей | Средняя | Каталог ~2000-2500 токенов — приемлемо даже для 4K контекста |
| Новый патч добавляет предметы | Каждые 2 недели | Достаточно обновить `DDRAGON_VERSION` константу. В будущем — автоопределение |
| LCU отдаёт display_name не совпадающий с DDragon name | Низкая | Fallback: оставить оригинал. Можно добавить fuzzy matching позже |

---

## Верификация после реализации

- [ ] `cargo check` проходит для всего workspace
- [ ] `cargo test -p leagueeye-server` проходит (включая новый тест фильтрации)
- [ ] Сервер запускается без ошибок
- [ ] При первом запросе коучинга загружается DDragon (видно в логах)
- [ ] В системном промпте есть блок СПРАВОЧНИК ПРЕДМЕТОВ
- [ ] В user message предметы на русском с стоимостью и тегами
- [ ] Без интернета / при ошибке DDragon — сервер работает, коучинг работает без каталога
- [ ] AI использует правильные русские названия из справочника
