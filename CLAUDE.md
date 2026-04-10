# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with this repository.

Всегда отвечай мне на русском языке, если я явно не попрошу другой язык.

## Build & Dev Commands

```bash
# Client (Tauri desktop app)
npm run tauri dev       # Full dev: Rust + React hot-reload (requires server running)
npm run dev             # Vite dev server only (port 5173, no Rust)
npm run build           # Production frontend build (used by Tauri beforeBuildCommand)
npm run tauri build     # Production build with NSIS installer
npx tsc --noEmit        # TypeScript type-check

# Server
cargo run -p leagueeye-server              # Dev server (reads server/.env)
cargo build -p leagueeye-server --release  # Production build

# Tests
cargo test -p league-eye                   # Small Rust unit-test set (overlay policy)
cargo test -p leagueeye-server             # Small Rust unit-test set (live enrichment helpers)

# Workspace-wide
cargo check                                # Check all 3 crates
cargo check -p league-eye                  # Client only
cargo check -p leagueeye-server            # Server only
cargo check -p leagueeye-shared            # Shared models only
```

**Rust version:** 1.88.0 (pinned in `rust-toolchain.toml`)

**Tests:** No dedicated frontend/E2E suite. There are a few Rust unit tests in `src-tauri/src/overlay_policy.rs` and `server/src/routes/live.rs`.

## Environment Variables

**Client** (`src-tauri/.env`):
```env
LEAGUEEYE_SERVER_URL=http://localhost:3000   # or remote server IP/domain
```

**Server** (`server/.env`):
```env
RIOT_API_KEY=RGAPI-...                       # Required (developer.riotgames.com)
DATABASE_URL=postgres://user:pass@host/db    # Optional in local dev; defaults to postgres://leagueeye:leagueeye@localhost/leagueeye
PORT=3000
AI_COACH_PROVIDER=anthropic                  # Optional: anthropic | openrouter | deepseek
AI_COACH_MAX_TOKENS=1024                     # Optional
ANTHROPIC_AUTH_TOKEN=...                     # Optional
ANTHROPIC_BASE_URL=https://api.anthropic.com # Optional
ANTHROPIC_MODEL=claude-sonnet-4-20250514     # Optional
OPENROUTER_API_KEY=...                       # Optional
OPENROUTER_BASE_URL=https://openrouter.ai/api/v1 # Optional
OPENROUTER_MODEL=openai/gpt-4o-mini          # Optional
OPENROUTER_HTTP_REFERER=https://your-app.example # Optional attribution header
OPENROUTER_TITLE=LeagueEye                   # Optional attribution header
DEEPSEEK_API_KEY=...                         # Optional
DEEPSEEK_BASE_URL=https://api.deepseek.com   # Optional
DEEPSEEK_MODEL=deepseek-chat                 # Optional
```

The server supports Anthropic, OpenRouter, and DeepSeek as AI providers. If `AI_COACH_PROVIDER` is omitted, the server auto-picks the first configured provider in this order: Anthropic → OpenRouter → DeepSeek. If no provider key is set, the server still starts normally — AI coaching simply won't be available.

## Architecture

Client-server Tauri 2 app. Windows desktop client (thin) + Axum HTTP server (all heavy lifting). League of Legends player stats, match history, live game, AI coaching. Riot API region: EUW (RU).

```
React UI ──invoke()──► Tauri commands ──HTTP──► Axum server ──► Riot API / PostgreSQL / AI provider
                              │
                     LCU API (local only, curl.exe)
```

### Cargo Workspace (3 crates)

- **`shared/`** (`leagueeye-shared`) — All DTOs shared between server and client. Single source of truth for data shapes.
- **`server/`** (`leagueeye-server`) — Axum HTTP server: Riot API with rate limiter, PostgreSQL, AI Coach SSE streaming.
- **`src-tauri/`** (`league-eye`) — Thin Tauri client: LCU detection, overlay windows, keyboard hooks, HTTP proxy to server.

### Server (server/src/)

- **main.rs** — Axum router, PostgreSQL pool (sqlx), auto-runs migrations, `AppState` with `RiotApiClient`, `Db`, optional `AiCoachConfig` (Anthropic / OpenRouter / DeepSeek, plus optional OpenRouter attribution headers)
- **riot_api.rs** — Riot API HTTP client with 2-level rate limiter (18 req/s + 95 req/2min), 429 retry logic
- **db.rs** — PostgreSQL queries (sqlx). Tables: accounts, matches, match_participants, rank_snapshots, champion_mastery. Includes dashboard aggregate queries (best players by role, top winrate champions)
- **routes/players.rs** — GET `/api/players/{game_name}/{tag_line}`, `/api/players/{puuid}/mastery`, `/api/players/{puuid}/matches`. Smart caching: serves cached data immediately, fetches fresh matches in background
- **routes/matches.rs** — GET `/api/matches/{match_id}` — cache-first, falls back to Riot API
- **routes/live.rs** — POST `/api/live/enrich` (client sends LCU data, server adds ranks via Spectator API + league entries, and hydrates missing Riot IDs / puuids when possible)
- **routes/coach.rs** — POST `/api/coach/stream` (receives `CoachingContext`, returns SSE stream from Anthropic / OpenRouter / DeepSeek). Russian-language system prompts with champion-specific guidance
- **routes/global.rs** — GET `/api/global/dashboard` — aggregate stats from PostgreSQL

### Client (src-tauri/src/)

- **lib.rs** — Tauri entry, 3 webviews (main + overlay + gold-overlay), system tray with Russian "Закрыть" menu, main window hides to tray on close. Installs Windows low-level keyboard hook (WH_KEYBOARD_LL) for Shift+E and gates overlays by current League window visibility
- **commands.rs** — 16 `#[tauri::command]` handlers. LCU calls are local; everything else proxies to server via HTTP. Includes live-game assembly, AI coach requests, overlay eligibility, and gold comparison data
- **api_client.rs** — `ServerApiClient`: reqwest HTTP client to server. `stream_coaching()` reads SSE line-by-line and emits Tauri `coach-stream` events. Self-signed TLS: `danger_accept_invalid_certs` for `https://` URLs
- **ai_coach.rs** — `CoachState` (dedup guard). `build_context_from_allgamedata()` (in-game rich stats) and `build_context_champ_select()` (draft phase) — transform LCU data into `CoachingContext` with champion meta info
- **lcu.rs** — League Client detection: lockfile parsing across C:/D:/E:/F:/ drives + PowerShell process fallback. Uses `curl.exe` (not reqwest) for LCU API to bypass TLS issues. Champ select session, gameflow phase, Live Client Data API (localhost:2999), fullscreen mode checks
- **league_window.rs** — Windows foreground-window monitor. Emits `league-window-visibility`, hides overlays when League loses focus, blocks overlays when fullscreen gameplay should suppress them
- **overlay_policy.rs** — Minimal overlay policy: overlays are allowed only during `InProgress` (unit-tested)
- **models.rs** — Client-local structs for LCU / Live Client API payloads used by the Tauri layer
- **db.rs** — Local SQLite: only `accounts` table for instant startup cache. All match data moved to server PostgreSQL

Tauri managed state: `ServerApiClient`, `SharedDb`, `CoachState`, `ChampionNamesCache`, `ItemCostCache`, `LastLiveState`

### Frontend (src/)

Triple Vite entry points: `index.html` (main app), `overlay.html` (coach overlay), `gold-overlay.html` (gold comparison overlay).

- **App.tsx** — 3 views: "home", "profile", "live". Auto-switches to "live" during champ select/in-game (1.5s debounce to avoid flicker). Shows/hides overlay windows automatically, but only when overlay eligibility and League window visibility both allow it
- **hooks/useRiotApi.ts** — Profile, matches, mastery, champion stats. State machine with `genRef` (aborts stale ops), `busyRef` (prevents concurrent searches), `lastSearchRef` (caches last result)
- **hooks/useLiveGame.ts** — Adaptive polling: 2s (champ select), 5s (in-game), 3s (idle). Guards against overlapping polls with `pollingRef` / `requestIdRef`
- **hooks/useOverlayLifecycle.ts** — 500ms polling of Tauri overlay eligibility (`get_overlay_eligibility`)
- **hooks/useAiCoach.ts** — Module-level persistent state (survives remount). Single global listener for `coach-stream` Tauri event. Accumulates streaming text into messages
- **hooks/useChampionNames.ts** — DDragon champion ID-to-name cache
- **components/OverlayApp.tsx** — Coach overlay UI. Auto-resizes the window, listens for `hotkey-coach-trigger`, Shift+drag to move
- **components/GoldOverlayApp.tsx** — Gold comparison overlay UI. Polls `get_gold_comparison`, auto-resizes, Shift+drag to move
- **lib/types.ts** — TypeScript interfaces mirroring `shared/src/models.rs` (camelCase)
- **lib/ddragon.ts** — Data Dragon CDN URL builders (version pinned), tier/rank/position formatters, spell ID→name map

### AI Coach Data Flow

```
Client: LCU allgamedata → build CoachingContext (ai_coach.rs)
  ↓ POST /api/coach/stream (JSON body)
Server: build_system_prompt + build_user_message → POST configured AI provider (stream:true)
  ↓ SSE: CoachStreamPayload {kind: "start"|"delta"|"end"|"error", text}
Client: api_client reads SSE → app.emit("coach-stream") → React useAiCoach hook
```

### Server Routes

| Method | Route | Description |
|---|---|---|
| GET | `/health` | Health check |
| GET | `/api/global/dashboard` | Aggregate stats (best players, top champions) |
| GET | `/api/players/{game_name}/{tag_line}` | Search player by Riot ID |
| GET | `/api/players/{puuid}/mastery` | Top champion mastery |
| GET | `/api/players/{puuid}/matches` | Match history + champion stats (paginated) |
| GET | `/api/matches/{match_id}` | Match detail (all 10 players) |
| POST | `/api/live/enrich` | Enrich LCU data with ranks |
| POST | `/api/coach/stream` | SSE AI coaching |

### Files That Must Stay in Sync

- `shared/src/models.rs` ↔ `src/lib/types.ts` (DTO shapes, both camelCase)
- `src-tauri/src/lib.rs` invoke_handler ↔ `invoke()` calls in React hooks
- `src-tauri/src/league_window.rs` emitted `league-window-visibility` event ↔ listener in `src/App.tsx`
- `server/src/routes/coach.rs` user message format ↔ `shared/src/models.rs` CoachingContext fields

## Conventions

- Rust commands: `Result<T, String>` for error handling
- All DTOs: `#[serde(rename_all = "camelCase")]` for JS interop
- Frontend: custom Tailwind v4 theme in `src/index.css` via `@theme` — tokens: `bg-primary`, `bg-card`, `accent`, `win`, `loss`, `gold`, `text-primary`, `text-secondary`, `text-muted`
- Match history: Intersection Observer + offset-based pagination
- Overlay windows are shown only when both conditions are true: current gameflow phase passes `overlay_policy` and the League window is foreground-visible
- Self-signed TLS: `api_client.rs` uses `danger_accept_invalid_certs` when URL starts with `https://`
- Server migrations: `server/migrations/` (sqlx, auto-run on startup)
- Deployment guide: see `DEPLOY.md`

## Tauri Window Config

- **Main window**: 1280×780, min 1024×600, centered, resizable
- **Overlay**: 420×200, transparent, always-on-top, no decorations, skip taskbar. Auto-resized from React content
- **Gold overlay**: 280×300 initial size, same properties as overlay. Auto-resized from React content
- Both overlay windows are hidden automatically when League loses foreground visibility or overlay policy blocks them
- Left-click tray icon restores window, right-click shows "Закрыть" menu
