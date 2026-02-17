# CollabBoard

Real-time collaborative whiteboard with AI agent. Built for the Gauntlet AI program (Week 1).

**Stack:** Rust/Axum + React/Konva.js + PostgreSQL + WebSockets

**Live:** *(deploy link TBD)*

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  Browser (React + Konva.js + Zustand)                       │
│                                                             │
│  ┌──────────┐  ┌──────────────┐  ┌───────────────────────┐ │
│  │ Toolbar  │  │ Konva Canvas │  │ Remote Cursors        │ │
│  │ Left/    │  │ (sticky note,│  │ (Konva arrows +       │ │
│  │ Right    │  │  rect, ellip,│  │  name labels)         │ │
│  │ Panels   │  │  grid lines) │  │                       │ │
│  └──────────┘  └──────────────┘  └───────────────────────┘ │
│       │               │                    │                │
│       └───────────────┴────────────────────┘                │
│                       │                                     │
│              ┌────────┴────────┐                            │
│              │  Frame Client   │  WebSocket singleton       │
│              │  (send/receive) │  Zustand store mutations   │
│              │  auto-reconnect │  optimistic object create  │
│              └────────┬────────┘                            │
└───────────────────────┼─────────────────────────────────────┘
                        │ WebSocket (JSON frames)
                        │
┌───────────────────────┼─────────────────────────────────────┐
│  Axum Server          │                                     │
│              ┌────────┴────────┐                            │
│              │   WS Handler    │  Parse frame, dispatch     │
│              │   (routes/ws)   │  by syscall prefix         │
│              └──┬──┬──┬──┬──┬─┘                            │
│                 │  │  │  │  │                               │
│   board:*───────┘  │  │  │  └───ai:*                       │
│   object:*─────────┘  │  └──────chat:*                     │
│   cursor:*─────────────┘                                    │
│                 │     │              │                       │
│  ┌──────────┐ ┌┴─────┴──┐ ┌────────┴───┐ ┌────────────┐   │
│  │ Board    │ │ Object   │ │ Cursor     │ │ AI Agent   │   │
│  │ Service  │ │ Service  │ │ (ephemeral │ │ (multi-turn│   │
│  │ (hydrate,│ │ (LWW     │ │  broadcast)│ │  tool-use  │   │
│  │  evict,  │ │  version │ │            │ │  loop)     │   │
│  │  flush)  │ │  check)  │ │            │ │            │   │
│  └────┬─────┘ └────┬─────┘ └────────────┘ └─────┬──────┘  │
│       │            │                             │          │
│       │     ┌──────┴──────┐              ┌───────┴───────┐  │
│       │     │ Persistence │              │ LLM Client    │  │
│       │     │ (100ms obj  │              │ (Anthropic/   │  │
│       │     │  flush +    │              │  OpenAI)      │  │
│       │     │  frame log) │              │ + rate limit  │  │
│       │     └──────┬──────┘              └───────────────┘  │
│       │            │                                        │
│  ┌────┴────────────┴──────────────────────────────────┐     │
│  │              PostgreSQL (Neon)                      │     │
│  │  users │ boards │ board_objects │ frames │ sessions │     │
│  │                  + ws_tickets                       │     │
│  └────────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────┘
```

## Frame Protocol

Every message between client and server is a **Frame**:

```json
{
  "id": "uuid",
  "parent_id": "uuid | null",
  "ts": 1708000000000,
  "syscall": "object:create",
  "status": "request",
  "board_id": "uuid",
  "from": "user-uuid",
  "data": { "kind": "sticky_note", "x": 100, "y": 200 }
}
```

**Status lifecycle:** `request → item* → done` (success) or `request → error` (failure).

**Syscall routing:**

| Prefix | Syscalls | Description |
|--------|----------|-------------|
| `session` | `connected`, `disconnected` | Server → client on WS connect/disconnect |
| `board` | `join`, `part`, `create`, `list`, `delete` | Board lifecycle + CRUD |
| `object` | `create`, `update`, `delete` | Object mutations (broadcast to all board clients) |
| `cursor` | `moved` | Ephemeral cursor positions (broadcast, not persisted) |
| `chat` | `message`, `history` | Persistent board chat |
| `ai` | `prompt`, `history` | Natural language → tool calls → mutations |

## Real-Time Sync

**Last-write-wins (LWW)**, server-authoritative:

- Server is the clock — client `ts` is ignored, server stamps all frames
- Per-object `version` (monotonic integer) — stale updates rejected
- Optimistic creates: client renders immediately with a temp ID, reconciles on server `done` response
- Cursor/presence is ephemeral — broadcast to peers only, never persisted
- Object persistence: dirty objects flushed to Postgres every 100ms (batch upsert)
- Frame persistence: append-only event log via bounded queue (8192 capacity, best-effort)
- Board hydration: first client to join loads objects from Postgres into memory; last client to leave flushes and evicts
- On disconnect: exponential backoff reconnect (1s → 2s → 4s → ... max 10s), fresh WS ticket, re-join board, full state snapshot

## AI Agent

Multi-turn LLM tool-calling agent via `ai:prompt` syscall. AI mutations flow through the same
`object_service` as human actions — all board clients see results in real-time.

**9 tools:** `createStickyNote`, `createShape`, `createFrame`, `createConnector`,
`moveObject`, `resizeObject`, `updateText`, `changeColor`, `getBoardState`

**Multi-turn context:** Last 20 frames (10 exchanges) loaded from the `frames` table per prompt, giving the LLM conversational memory within a board.

**Grid context:** Client sends an 8x8 "battleship grid" overlay (A1–H8) mapped to the current viewport, so the LLM can reference spatial positions by cell label.

**Rate limiting:** Sliding window — 10 req/min per client, 20 req/min global, 50k tokens/hr per client. All configurable via env.

**Complex commands:** "Create a SWOT analysis", "Build a user journey map",
"Set up a retrospective board" — multi-step execution (up to 10 tool iterations per prompt).

**Provider:** Anthropic Claude (default) or OpenAI, config-driven via `LLM_PROVIDER` env var. Both optional — server starts without them, AI endpoints return 503.

## Design

Retro-scientific "Field Survey Terminal" aesthetic. See [docs/DESIGN.md](docs/DESIGN.md).

- **Fonts:** IBM Plex Mono (UI chrome) + Caveat (handwritten sticky note text)
- **Palette:** Warm earth tones — beige canvas, brown text, forest green accents
- **Rules:** Zero border-radius, zero shadows, zero gradients. Full dark mode support.

## Project Structure

```
collaboard/
├── server/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs                # Entry: env, DB pool, Axum server
│       ├── frame.rs               # Frame type, Status enum, serialization
│       ├── state.rs               # AppState, BoardObject, BoardState
│       ├── rate_limit.rs          # Sliding-window rate limiter (per-client + global)
│       ├── routes/
│       │   ├── mod.rs             # Router assembly + SPA fallback
│       │   ├── auth.rs            # GitHub OAuth, session endpoints, WS tickets
│       │   ├── users.rs           # User profile + aggregate stats
│       │   └── ws.rs              # WebSocket upgrade + frame dispatch loop
│       ├── services/
│       │   ├── ai.rs              # Multi-turn LLM agent (tool-use loop)
│       │   ├── auth.rs            # GitHub code exchange, user upsert
│       │   ├── board.rs           # Board CRUD, hydration, eviction, broadcast
│       │   ├── object.rs          # Object mutations, LWW version check
│       │   ├── persistence.rs     # 100ms object flush + frame log worker
│       │   └── session.rs         # Session CRUD, WS ticket issue/consume
│       ├── llm/
│       │   ├── mod.rs             # LlmClient trait + provider dispatch
│       │   ├── anthropic.rs       # Anthropic Messages API
│       │   ├── openai.rs          # OpenAI Chat Completions API
│       │   ├── tools.rs           # 9 tool definitions (provider-agnostic)
│       │   └── types.rs           # Shared LLM types
│       └── db/
│           ├── mod.rs             # Pool init + migration runner
│           └── migrations/
│               ├── 001_users.sql
│               ├── 002_boards.sql
│               ├── 003_board_objects.sql
│               ├── 004_frames.sql
│               └── 005_sessions.sql
├── client/
│   ├── package.json               # Bun + Vite + React + Konva
│   └── src/
│       ├── App.tsx                # Root: auth check, page routing, dark mode
│       ├── main.tsx               # Vite entry point
│       ├── pages/
│       │   ├── LoginPage.tsx      # GitHub OAuth redirect button
│       │   ├── DashboardPage.tsx  # Board list + create board
│       │   └── BoardPage.tsx      # Board workspace (toolbar + canvas + panels)
│       ├── canvas/
│       │   ├── Canvas.tsx         # Konva Stage: pan/zoom, grid, object render, cursors
│       │   ├── StickyNote.tsx     # Draggable/resizable sticky note + text edit
│       │   ├── Shape.tsx          # Rectangle + Ellipse (draggable/resizable)
│       │   └── TextEditor.tsx     # HTML textarea overlay for inline text editing
│       ├── components/
│       │   ├── Toolbar.tsx        # Top bar: board name, presence chips, dark mode
│       │   ├── LeftPanel.tsx      # Collapsible tool rail + inspector
│       │   ├── ToolRail.tsx       # Tool buttons (select, sticky, rect, ellipse, ...)
│       │   ├── InspectorPanel.tsx # Selection inspector: position, size, color
│       │   ├── RightPanel.tsx     # Collapsible right rail: boards, chat, AI
│       │   ├── MissionControl.tsx # In-board board list / switcher
│       │   ├── ChatPanel.tsx      # Persistent board chat
│       │   ├── AiPanel.tsx        # AI prompt input + response display
│       │   ├── StatusBar.tsx      # Bottom bar: connection, object count, zoom
│       │   ├── BoardStamp.tsx     # Canvas overlay stamp
│       │   ├── BoardCard.tsx      # Reusable board card (full + mini variants)
│       │   └── UserFieldReport.tsx # User profile popover with stats
│       ├── hooks/
│       │   ├── useFrameClient.ts  # WS lifecycle, reconnect, frame dispatch
│       │   ├── useAI.ts           # AI prompt sender + grid context builder
│       │   └── useCanvasSize.ts   # ResizeObserver for canvas container
│       ├── lib/
│       │   ├── frameClient.ts     # FrameClient class (WS singleton, pub/sub)
│       │   ├── api.ts             # REST helpers (auth, profile, WS ticket)
│       │   ├── grid.ts            # 8x8 grid utilities for AI spatial context
│       │   └── types.ts           # Shared TypeScript types
│       └── store/
│           └── board.ts           # Zustand store (objects, presence, selection, UI)
├── docs/
│   └── DESIGN.md                  # Design system spec (theme, palette, typography)
├── Dockerfile                     # Multi-stage: Bun build → Rust build → slim runtime
├── docker-compose.yml             # App + Postgres 16 for local dev
├── fly.toml                       # Fly.io: region dfw, auto-migrate, health check
├── run-dev.sh                     # Local dev runner (docker-compose wrapper)
└── .env.example
```

## Authentication

Two-step WS auth to prevent cookie leakage over WebSocket:

1. **GitHub OAuth** → server exchanges code, upserts user, sets `HttpOnly` session cookie (30-day TTL)
2. **WS ticket** → client calls `POST /api/auth/ws-ticket` (requires session cookie) → receives a one-time 16-byte ticket (30s TTL)
3. **WS upgrade** → client connects to `/api/ws?ticket=<ticket>` → server atomically consumes ticket via `DELETE ... RETURNING user_id`

All HTTP API routes use an `AuthUser` extractor that validates the session cookie. GitHub OAuth and LLM are both optional — server starts without them; endpoints return 503 if unconfigured.

## Feature Status

### Implemented
- Infinite canvas with pan (scroll) and zoom (Ctrl+wheel, 0.1x–5x)
- Sticky notes (create, drag, resize, rotate, edit text, 8 color swatches)
- Rectangle and ellipse shapes (create, drag, resize, rotate)
- Grid background (20px minor, 100px major lines)
- Real-time sync: object create/update/delete broadcast to all board clients
- Optimistic creates with temp-ID reconciliation
- Remote cursors with arrow + name label (50ms throttle, broadcast-only)
- Presence awareness (toolbar chips, join/part events)
- Board CRUD (create, list, delete) via WS frames + Dashboard UI
- Board chat (persistent, with history load)
- AI agent (9 tools, multi-turn, multi-provider, rate-limited)
- AI chat panel with prompt input and response rendering
- Inspector panel (position, size, rotation, color picker)
- Dark mode (CSS class toggle, persisted in localStorage)
- User profile popover with aggregate stats from frame log
- GitHub OAuth authentication with CSRF protection
- Docker + Fly.io deployment pipeline
- Backend test suite (frame, state, rate_limit, board, object, AI, WS)

### Not yet implemented
- Frame objects (server creates them, but no frontend renderer)
- Connector objects (server creates them, but no frontend renderer)
- Line, text, draw, eraser tools (toolbar buttons visible but disabled)
- Multi-select (rubber band selection)
- Copy/paste
- Viewport culling (render only visible objects)
- URL-based routing (currently state-based page switching in App.tsx)

## Setup

### Prerequisites

- Rust 1.85+ (see `rust-toolchain.toml`)
- Bun (frontend package manager)
- PostgreSQL (or Neon connection string)
- GitHub OAuth App ([register here](https://github.com/settings/applications/new))
- Anthropic or OpenAI API key (optional — AI features disabled without it)

### Environment

```bash
cp .env.example .env
# Edit .env with your values:
#   DATABASE_URL=postgres://...
#   LLM_PROVIDER=anthropic
#   LLM_MODEL=claude-sonnet-4-20250514
#   LLM_API_KEY=sk-ant-...
```

### Database

Migrations run automatically on server startup. To run them manually:

```bash
cargo install sqlx-cli --no-default-features --features postgres
sqlx migrate run --source server/src/db/migrations
```

### Run

```bash
# Full stack (app + Postgres) via Docker
cp .env.example .env
./run-dev.sh
# or: docker-compose up
```

App serves on `http://localhost:3000`.

`run-dev.sh` reuses existing images for faster startup. Force a rebuild when
you change code/dependencies:

```bash
./run-dev.sh --build
```

## Fly.io Deployment

`Dockerfile` builds both the Rust server and the React client. Axum serves the
compiled client from `STATIC_DIR=/app/client/dist` in the runtime image.

### First-time setup

```bash
fly launch --no-deploy
```

Set required secrets:

```bash
fly secrets set \
  DATABASE_URL="postgres://..." \
  GITHUB_CLIENT_ID="..." \
  GITHUB_CLIENT_SECRET="..." \
  GITHUB_REDIRECT_URI="https://<your-app>.fly.dev/auth/github/callback"
```

Optional AI secrets:

```bash
fly secrets set \
  LLM_PROVIDER="anthropic" \
  LLM_API_KEY_ENV="ANTHROPIC_API_KEY" \
  ANTHROPIC_API_KEY="sk-ant-..."
```

### Deploy

```bash
fly deploy
```

`fly.toml` runs `release_command = "collaboard --migrate-only"` so DB migrations
complete before machines are promoted.

### Important scaling note

Realtime board state is process-local today. Keep a single machine until shared
cross-instance broadcast/state is implemented:

```bash
fly scale count 1
```

## HTTP API

| Route | Method | Auth | Description |
|-------|--------|------|-------------|
| `/auth/github` | GET | No | Redirect to GitHub OAuth |
| `/auth/github/callback` | GET | No | Exchange code, set session cookie |
| `/api/auth/me` | GET | Session | Return current user |
| `/api/auth/logout` | POST | Session | Delete session, clear cookie |
| `/api/auth/ws-ticket` | POST | Session | Issue one-time WS upgrade ticket |
| `/api/users/:id/profile` | GET | Session | User profile + aggregate stats |
| `/api/ws?ticket=` | GET | Ticket | WebSocket upgrade |
| `/healthz` | GET | No | Health check |

All other paths serve the React SPA (`client/dist/index.html`).

## References

- [Architecture Pre-Search](docs/PRE-SEARCH.md)
- [Design System](docs/DESIGN.md)
