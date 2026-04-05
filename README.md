# LeagueEye

Десктоп-приложение для просмотра статистики League of Legends. Показывает профиль игрока, историю матчей с детальной разбивкой по участникам, статистику по чемпионам, а также данные чемп-селекта и активной игры в реальном времени.

**Стек:** Tauri 2 · Rust · React 19 · TypeScript · Tailwind CSS 4 · SQLite

---

## Требования

| Инструмент | Версия |
|---|---|
| [Node.js](https://nodejs.org/) | 18+ |
| [Rust](https://rustup.rs/) | 1.77+ (через `rustup`) |
| [Tauri CLI зависимости](https://tauri.app/start/prerequisites/) | Windows: WebView2 (обычно уже есть) |

Убедись, что установлен WebView2 Runtime — он идёт вместе с Windows 10/11, но на «чистых» сборках его нужно [скачать отдельно](https://developer.microsoft.com/microsoft-edge/webview2/).

---

## Настройка API ключа

Приложение использует Riot Games API. Ключ нужно получить на [developer.riotgames.com](https://developer.riotgames.com/).

Создай файл `.env` в папке `src-tauri/` (если его нет):

```
RIOT_API_KEY=RGAPI-xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
```

> Ключ с Developer Portal действует 24 часа. Для постоянной работы нужен Production API key.

---

## Установка и запуск

### Режим разработки

```bash
# 1. Установить зависимости фронтенда
npm install

# 2. Запустить приложение в dev-режиме (hot-reload)
npm run tauri dev
```

При первом запуске Rust-зависимости скачиваются и компилируются — это займёт несколько минут.

### Production сборка

```bash
npm run tauri build
```

Готовый инсталлятор появится в `src-tauri/target/release/bundle/`.

---

## Структура проекта

```
LeagueEye/
├── src/                        # React-фронтенд
│   ├── App.tsx                 # Корневой компонент, маршрутизация экранов
│   ├── components/             # UI-компоненты
│   │   ├── MatchCard.tsx       # Строка матча (с раскрытием)
│   │   ├── MatchDetailView.tsx # Детальная таблица 10 игроков матча
│   │   ├── MatchHistory.tsx    # Список матчей с пагинацией
│   │   ├── LiveGameView.tsx    # Экран чемп-селекта / активной игры
│   │   ├── ProfileCard.tsx     # Карточка профиля
│   │   └── ...
│   ├── hooks/
│   │   ├── useRiotApi.ts       # Вызовы Tauri-команд, состояние профиля
│   │   └── useLiveGame.ts      # Поллинг Live Game
│   └── lib/
│       ├── types.ts            # TypeScript-типы
│       └── ddragon.ts          # Утилиты Data Dragon (иконки, форматирование)
│
└── src-tauri/                  # Rust-бэкенд
    ├── .env                    # RIOT_API_KEY (не коммитить)
    ├── tauri.conf.json         # Конфиг окна и бандла
    └── src/
        ├── lib.rs              # Точка входа Tauri, инициализация БД
        ├── commands.rs         # Tauri-команды (бизнес-логика)
        ├── riot_api.rs         # HTTP-клиент Riot API с rate limiter
        ├── db.rs               # SQLite: миграции, чтение/запись
        ├── models.rs           # Rust-структуры (DTO, модели)
        └── lcu.rs              # League Client API (LCU) для live данных
```

---

## Возможности

- **Поиск игрока** по Riot ID (`Имя#TAG`) с кешированием в SQLite
- **Профиль**: уровень, иконка, ранги Solo/Flex, топ чемпионы по мастерству
- **История матчей**: до 500 матчей, пагинация, статистика W/L/WR по чемпионам
- **Детали матча**: раскрываемая карточка с таблицей всех 10 участников — KDA, CS, КП%, золото, урон, предметы
- **Навигация**: клик по нику любого игрока в таблице матча открывает его профиль
- **Live Game**: автоопределение текущего аккаунта через League Client, показ чемп-селекта и активной игры с рангами игроков
- **Кеш**: все матчи сохраняются локально в SQLite, повторные запросы не делают API-вызовы

---

## База данных

SQLite хранится в папке данных приложения:

- **Windows:** `%APPDATA%\com.leagueeye.app\leagueeye.db`

Таблицы:
- `accounts` — последние просмотренные аккаунты
- `matches` — сводка по матчу для одного игрока (KDA, CS, предметы и т.д.)
- `match_participants` — все 10 участников каждого матча (полная детализация)
- `rank_snapshots` — история LP для расчёта ΔLP за матч
- `champion_mastery` — мастерство чемпионов

---

## Используемые API

| Эндпоинт | Назначение |
|---|---|
| `account/v1/accounts/by-riot-id` | Получить PUUID по Riot ID |
| `lol/summoner/v4/summoners/by-puuid` | Данные призывателя |
| `lol/league/v4/entries/by-puuid` | Ранги |
| `lol/champion-mastery/v4/.../top` | Топ мастерство |
| `lol/match/v5/matches/by-puuid/.../ids` | Список ID матчей |
| `lol/match/v5/matches/{id}` | Полные данные матча |
| `lol/spectator/v5/active-games/by-summoner` | Активная игра |
| LCU (локальный) | Чемп-селект, автоопределение аккаунта |

Rate limiter: 18 req/s, 95 req/2 min (с запасом от лимитов Riot).
