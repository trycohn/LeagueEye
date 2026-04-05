# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Всегда отвечай мне на русском языке, если я явно не попрошу другой язык.

## Build & Dev Commands

```bash
# Client (Tauri desktop app)
npm run tauri dev       # Full dev: Rust + React hot-reload (requires server running)
npm run dev             # Vite dev server only (port 5173, no Rust)
npm run tauri build     # Production build with NSIS installer
npx tsc --noEmit        # TypeScript type-check

# Server
cargo run -p leagueeye-server              # Dev server (reads server/.env)
cargo build -p leagueeye-server --release  # Production build

# Workspace-wide
cargo check                                # Check all 3 crates
cargo check -p league-eye                  # Client only
cargo check -p leagueeye-server            # Server only
cargo check -p leagueeye-shared            # Shared models only
```

## Environment Variables

**Client** (`src-tauri/.env`):
```env
LEAGUEEYE_SERVER_URL=http://localhost:3000   # or remote server IP/domain
```

**Server** (`server/.env`):
```env
RIOT_API_KEY=RGAPI-...                       # Required (developer.riotgames.com)
DATABASE_URL=postgres://user:pass@host/db    # Required (PostgreSQL)
PORT=3000
ANTHROPIC_AUTH_TOKEN=...                     # Optional (AI Coach disabled without it)
ANTHROPIC_BASE_URL=https://api.anthropic.com # Optional
ANTHROPIC_MODEL=claude-sonnet-4-6            # Optional
```

## Architecture

Client-server Tauri 2 app. Windows desktop client (thin) + Axum HTTP server (all heavy lifting). League of Legends player stats, match history, live game, AI coaching.

```
React UI ──invoke()──► Tauri commands ──HTTP──► Axum server ──► Riot API / PostgreSQL / Anthropic
                              │
                     LCU API (local only)
```

### Cargo Workspace (3 crates)

- **`shared/`** (`leagueeye-shared`) — All DTOs shared between server and client. Single source of truth for data shapes.
- **`server/`** (`leagueeye-server`) — Axum HTTP server: Riot API with rate limiter, PostgreSQL, AI Coach SSE streaming.
- **`src-tauri/`** (`league-eye`) — Thin Tauri client: LCU detection, overlay, keyboard hooks, HTTP to server.

### Server (server/src/)

- **main.rs** — Axum router, PostgreSQL pool (sqlx), `AppState` with `RiotApiClient`, `Db`, optional `AiCoachConfig`
- **riot_api.rs** — Riot API HTTP client with 2-level rate limiter (18 req/s + 95 req/2min)
- **db.rs** — PostgreSQL queries (sqlx). Tables: accounts, matches, match_participants, rank_snapshots, champion_mastery
- **routes/players.rs** — GET `/api/players/{name}/{tag}`, `/api/players/{puuid}/mastery`, `/api/players/{puuid}/matches`
- **routes/matches.rs** — GET `/api/matches/{matchId}`
- **routes/live.rs** — POST `/api/live/enrich` (client sends LCU data, server adds ranks via Spectator API)
- **routes/coach.rs** — POST `/api/coach/stream` (receives `CoachingContext`, returns SSE stream from Anthropic)

### Client (src-tauri/src/)

- **lib.rs** — Tauri setup, overlay window, low-level Windows keyboard hook (Shift+E), state management
- **commands.rs** — `#[tauri::command]` handlers. LCU calls are local; everything else proxies to server
- **api_client.rs** — `ServerApiClient`: reqwest HTTP client to server. Includes `stream_coaching()` which reads SSE and emits Tauri events
- **ai_coach.rs** — `CoachState` (deduplication), `build_context_from_allgamedata()` and `build_context_champ_select()` (context builders from LCU data). Prompts and streaming live on server.
- **lcu.rs** — League Client detection (lockfile parsing), champ select, gameflow phase, Live Client Data API (localhost:2999)
- **db.rs** — Local SQLite: only `accounts` table for instant startup cache

Tauri managed state: `ServerApiClient`, `SharedDb`, `CoachState`, `ChampionNamesCache`, `LastLiveState`

### Frontend (src/)

- **App.tsx** — 3 views: "home", "profile", "live". Auto-switches to "live" during champ select/in-game
- **hooks/useRiotApi.ts** — Profile, matches, mastery, champion stats. Generation counter (`genRef`) aborts stale ops; busy flag prevents concurrent searches
- **hooks/useLiveGame.ts** — Polls live game: 3s (champ select), 15s (in-game), 8s (idle)
- **hooks/useAiCoach.ts** — Module-level persistent state (survives remount). Single global listener for `coach-stream` Tauri event
- **hooks/useChampionNames.ts** — DDragon champion ID-to-name cache
- **lib/types.ts** — TypeScript interfaces mirroring `shared/src/models.rs`
- **lib/ddragon.ts** — Data Dragon CDN URL builders, formatting utilities (DDragon version pinned)

Dual Vite entry points: `index.html` (main app) + `overlay.html` (coach overlay window).

### AI Coach Data Flow

```
Client: LCU allgamedata → build CoachingContext (ai_coach.rs)
  ↓ POST /api/coach/stream (JSON body)
Server: build_system_prompt + build_user_message → POST Anthropic Messages API (stream:true)
  ↓ SSE: CoachStreamPayload {kind: "start"|"delta"|"end"|"error", text}
Client: api_client reads SSE → app.emit("coach-stream") → React useAiCoach hook
```

### Files That Must Stay in Sync

- `shared/src/models.rs` ↔ `src/lib/types.ts` (DTO shapes, both camelCase)
- `src-tauri/src/lib.rs` invoke_handler ↔ `invoke()` calls in React hooks
- `server/src/routes/coach.rs` user message format ↔ `shared/src/models.rs` CoachingContext fields

## Conventions

- Rust commands: `Result<T, String>` for error handling
- All DTOs: `#[serde(rename_all = "camelCase")]` for JS interop
- Frontend: custom Tailwind v4 theme in `src/index.css` via `@theme` — tokens: `bg-primary`, `bg-card`, `accent`, `win`, `loss`, `gold`, `text-primary`, `text-secondary`, `text-muted`
- Match history: Intersection Observer + offset-based pagination
- Self-signed TLS: `api_client.rs` uses `danger_accept_invalid_certs` when URL starts with `https://`
- Server migrations: `server/migrations/` (sqlx, auto-run on startup)
- Deployment guide: see `DEPLOY.md`
