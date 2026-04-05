# LeagueEye — Гайд по деплою (сервер + клиент)

## Архитектура

```
┌──────────────────────┐        HTTPS        ┌──────────────────────────┐
│  Windows клиент      │ ◄──────────────────► │  Ubuntu сервер           │
│  (LeagueEye.exe)     │                      │  (домашний / VPS)        │
│                      │                      │                          │
│  • UI (React)        │                      │  • Riot API + рейт-лимит │
│  • LCU детекция      │                      │  • PostgreSQL            │
│  • Overlay + хоткеи  │                      │  • AI Coach (Anthropic)  │
│  • Мини SQLite кеш   │                      │                          │
└──────────────────────┘                      └──────────────────────────┘
```

---

## Часть 1 — Сервер (Ubuntu)

### Что нужно установить

```bash
# 1. Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 2. PostgreSQL
sudo apt update
sudo apt install -y postgresql postgresql-contrib

# 3. Клонировать репозиторий
git clone <repo-url> ~/LeagueEye
cd ~/LeagueEye
```

### Настройка PostgreSQL

```bash
# Создать пользователя и базу
sudo -u postgres psql <<EOF
CREATE USER leagueeye WITH PASSWORD 'придумай_пароль';
CREATE DATABASE leagueeye OWNER leagueeye;
EOF
```

### Настройка .env сервера

```bash
cp server/.env.example server/.env
nano server/.env
```

Содержимое `server/.env`:
```env
RIOT_API_KEY=RGAPI-твой-ключ-с-developer.riotgames.com
DATABASE_URL=postgres://leagueeye:придумай_пароль@localhost/leagueeye
PORT=3000
RUST_LOG=info
```

> **Важно:** Riot API ключ разработчика истекает каждые 24 часа. Для постоянного сервера нужен Production ключ (подавать заявку на developer.riotgames.com).

### Сборка и запуск

```bash
cd ~/LeagueEye
cargo build -p leagueeye-server --release
```

Бинарник: `target/release/leagueeye-server`

#### Запуск вручную (тест)

```bash
./target/release/leagueeye-server
# Проверка: curl http://localhost:3000/health
```

#### Запуск как systemd сервис (постоянный)

```bash
sudo nano /etc/systemd/system/leagueeye.service
```

```ini
[Unit]
Description=LeagueEye Server
After=network.target postgresql.service

[Service]
Type=simple
User=твой_юзер
WorkingDirectory=/home/твой_юзер/LeagueEye/server
ExecStart=/home/твой_юзер/LeagueEye/target/release/leagueeye-server
EnvironmentFile=/home/твой_юзер/LeagueEye/server/.env
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable leagueeye
sudo systemctl start leagueeye
sudo systemctl status leagueeye    # проверить статус
journalctl -u leagueeye -f         # логи
```

### TLS (HTTPS) через Caddy

Если у тебя есть домен (или хочешь по IP с самоподписанным сертификатом):

```bash
sudo apt install -y caddy
sudo nano /etc/caddy/Caddyfile
```

**С доменом:**
```
leagueeye.твой-домен.ru {
    reverse_proxy localhost:3000
}
```

**По IP (самоподписанный сертификат):**
```
:443 {
    tls internal
    reverse_proxy localhost:3000
}
```

```bash
sudo systemctl restart caddy
```

### Открыть порт в файрволе

```bash
sudo ufw allow 3000/tcp    # без Caddy (только HTTP)
# или
sudo ufw allow 443/tcp     # с Caddy (HTTPS)
```

Если роутер — пробросить порт 3000 (или 443) на IP сервера.

### Что живёт на сервере — итого

| Файл/папка | Что делает |
|---|---|
| `server/src/main.rs` | Axum HTTP сервер, роуты, middleware |
| `server/src/riot_api.rs` | Riot API клиент с рейт-лимитером (18 req/s, 95 req/2min) |
| `server/src/db.rs` | PostgreSQL — все матчи, ранги, мастерство |
| `server/src/routes/players.rs` | Поиск игрока, мастерство, история матчей |
| `server/src/routes/matches.rs` | Детали матча |
| `server/src/routes/live.rs` | Обогащение live game данных рангами |
| `server/migrations/` | SQL миграции (запускаются автоматически) |
| `server/.env` | RIOT_API_KEY, DATABASE_URL, PORT |

### API эндпоинты сервера

| Эндпоинт | Метод | Описание |
|---|---|---|
| `/health` | GET | Проверка что сервер работает |
| `/api/players/{name}/{tag}` | GET | Поиск игрока по Riot ID |
| `/api/players/{puuid}/mastery` | GET | Топ-10 мастерство чемпионов |
| `/api/players/{puuid}/matches` | GET | История матчей + статистика |
| `/api/players/{puuid}/matches?offset=15&limit=15` | GET | Пагинация матчей |
| `/api/matches/{matchId}` | GET | Детали матча (10 игроков) |
| `/api/live/enrich` | POST | Клиент шлёт LCU данные → сервер добавляет ранги |

---

## Часть 2 — Клиент (Windows)

### Что нужно для сборки

- Node.js 18+
- Rust (rustup)
- Visual Studio Build Tools (C++ workload)

### Настройка .env клиента

`src-tauri/.env`:
```env
LEAGUEEYE_SERVER_URL=http://ip-твоего-сервера:3000
ANTHROPIC_AUTH_TOKEN=твой-ключ-если-нужен-ai-coach
ANTHROPIC_BASE_URL=https://api.anthropic.com
ANTHROPIC_MODEL=claude-sonnet-4-6
```

> Если сервер за Caddy с HTTPS: `LEAGUEEYE_SERVER_URL=https://leagueeye.твой-домен.ru`

### Сборка EXE

```bash
npm install
npm run tauri build
```

Готовый установщик: `src-tauri/target/release/bundle/nsis/LeagueEye_0.1.0_x64-setup.exe`

### Распространение

Отдать `LeagueEye_0.1.0_x64-setup.exe` друзьям. Им нужно:
1. Установить программу
2. Запустить League of Legends
3. LeagueEye автоматически определит аккаунт через League Client

### Что живёт в клиенте — итого

| Файл | Что делает |
|---|---|
| `src-tauri/src/api_client.rs` | HTTP клиент к серверу (заменяет Riot API) |
| `src-tauri/src/commands.rs` | Tauri команды: LCU локально + HTTP к серверу |
| `src-tauri/src/lcu.rs` | Детекция League Client (lockfile, champ select, gameflow) |
| `src-tauri/src/ai_coach.rs` | AI coaching (стриминг через Anthropic API) |
| `src-tauri/src/db.rs` | Мини SQLite — только кеш последнего аккаунта |
| `src-tauri/src/lib.rs` | Tauri setup, overlay, keyboard hook |
| `src/` | React фронтенд (без изменений) |

### Что клиент делает локально (без сервера)

- Определяет запущен ли League Client
- Читает lockfile для авторизации в LCU API
- Получает данные champ select (пики, баны, таймер)
- Получает данные in-game через Live Client Data API (localhost:2999)
- Показывает overlay окно по Shift+E
- Кеширует последний аккаунт для мгновенного запуска

### Что клиент запрашивает у сервера

- Поиск игрока → ранги, уровень, иконка
- История матчей → KDA, CS, предметы, LP дельта
- Детали матча → все 10 игроков
- Мастерство чемпионов
- Live game enrichment → ранги всех игроков в текущей игре

---

## Часть 3 — Shared крейт

| Файл | Что содержит |
|---|---|
| `shared/src/models.rs` | Все DTO/структуры (используются и сервером, и клиентом) |
| `shared/src/lib.rs` | Реэкспорт моделей |

Если меняешь структуру данных — меняй в `shared/src/models.rs`, обе стороны автоматически обновятся.

Также нужно держать в синхронизации `src/lib/types.ts` (TypeScript типы для фронтенда).

---

## Быстрый старт (всё на одной машине для теста)

```bash
# Терминал 1 — сервер
cd LeagueEye
cp server/.env.example server/.env
# Отредактировать server/.env — вписать RIOT_API_KEY и DATABASE_URL
cargo run -p leagueeye-server

# Терминал 2 — клиент
cd LeagueEye
# src-tauri/.env уже содержит LEAGUEEYE_SERVER_URL=http://localhost:3000
npm run tauri dev
```

---

## Обновление

### Сервер
```bash
cd ~/LeagueEye
git pull
cargo build -p leagueeye-server --release
sudo systemctl restart leagueeye
```

### Клиент
```bash
cd LeagueEye
git pull
npm run tauri build
# Раздать новый EXE
```

---

## Устранение проблем

| Проблема | Решение |
|---|---|
| `connection refused` на клиенте | Проверь что сервер запущен и порт открыт: `curl http://ip:3000/health` |
| `API-ключ истёк` на сервере | Обнови RIOT_API_KEY в `server/.env`, `sudo systemctl restart leagueeye` |
| Миграции не запустились | Проверь DATABASE_URL, права пользователя PostgreSQL |
| Нет данных в live game | Убедись что League Client запущен и ты в игре/champ select |
| Нет рангов в live game | Сервер недоступен — клиент покажет данные без рангов (fallback) |
