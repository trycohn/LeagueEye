# CLAUDE.md
Всегда отвечай мне на русском языке, если я явно не попрошу другой язык.
This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Dev Commands

```bash
npm run tauri dev       # Full dev environment (Rust + React hot-reload)
npm run dev             # Vite dev server only (port 5173)
npm run build           # TypeScript check + Vite build (frontend only)
npm run tauri build     # Production build with installer
npx tsc --noEmit        # Type-check without emit
```

Rust requires `src-tauri/.env` with `RIOT_API_KEY=RGAPI-...` (Riot Developer Portal key).

## Architecture

Tauri 2 desktop app: Rust backend + React 19 / TypeScript frontend. Displays League of Legends player stats, match history, and live game data.

**Data flow:** React → `invoke()` Tauri command → Rust handler → Riot API / SQLite → response → React state update

### Backend (src-tauri/src/)

- **lib.rs** — Tauri setup, DB initialization, command registration (invoke_handler)
- **commands.rs** — 11 `#[tauri::command]` async handlers (search_player, get_matches_and_stats, get_live_game, etc.)
- **riot_api.rs** — HTTP client with 2-level rate limiter (18 req/s + 95 req/2min). Releases mutex before sleeping to avoid blocking UI
- **db.rs** — SQLite schema and queries. Tables: accounts, matches (composite PK: match_id+puuid), match_participants, rank_snapshots, champion_mastery
- **models.rs** — Rust DTOs with `#[serde(rename_all = "camelCase")]` for JS interop
- **lcu.rs** — League Client detection via Windows process command-line args (port + auth token)

Tauri state: `RiotApiClient` (singleton), `SharedDb = Arc<Mutex<Db>>`, `FetchProgress` (background fetch tracking).

### Frontend (src/)

- **App.tsx** — Root component with 3 views: "home", "profile", "live". Auto-switches to "live" during champ select/in-game
- **hooks/useRiotApi.ts** — Main data hook: profile, matches, mastery, champion stats. Uses generation counter (`genRef`) to abort stale async ops and busy flag to prevent concurrent searches
- **hooks/useLiveGame.ts** — Polls live game with variable intervals: 3s (champ select), 15s (in-game), 8s (idle)
- **hooks/useChampionNames.ts** — Fetches and caches champion ID→name mapping from Data Dragon
- **lib/types.ts** — TypeScript interfaces (camelCase, mirrors Rust models)
- **lib/ddragon.ts** — Data Dragon CDN URL builders and formatting utilities

### Files That Must Stay in Sync

- `src-tauri/src/models.rs` ↔ `src/lib/types.ts` (DTO shapes)
- `src-tauri/src/lib.rs` invoke_handler ↔ `invoke()` calls in React hooks
- `src-tauri/src/db.rs` schema ↔ Rust model structs

## Conventions

- Rust: `Result<T, String>` for command error handling
- Frontend errors shown in error banner, fallback to cached data
- Custom Tailwind v4 theme in `src/index.css` using `@theme` — color tokens: `bg-primary`, `bg-card`, `accent`, `win`, `loss`, `gold`, `text-primary`, `text-secondary`, `text-muted`
- Match history uses Intersection Observer + offset-based SQL pagination (initial 15, fetches up to 500)
- Rank LP delta calculated from rank_snapshots table
