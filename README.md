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
│  │ Toolbar  │  │ Konva Canvas │  │ Presence / Cursors    │ │
│  │ (DOM)    │  │ (WebGL)      │  │ (DOM overlay)         │ │
│  └──────────┘  └──────────────┘  └───────────────────────┘ │
│       │               │                    │                │
│       └───────────────┴────────────────────┘                │
│                       │                                     │
│              ┌────────┴────────┐                            │
│              │  Frame Client   │  WebSocket singleton       │
│              │  (send/receive) │  Zustand store mutations   │
│              └────────┬────────┘                            │
└───────────────────────┼─────────────────────────────────────┘
                        │ WebSocket (frames as JSON)
                        │
┌───────────────────────┼─────────────────────────────────────┐
│  Axum Server          │                                     │
│              ┌────────┴────────┐                            │
│              │   WS Handler    │  Parse frame, dispatch     │
│              │   (routes/ws)   │  by syscall prefix         │
│              └──┬──┬──┬──┬────┘                            │
│                 │  │  │  │                                  │
│    board:*──────┘  │  │  └──────ai:*                       │
│    object:*────────┘  └─────────cursor:*                   │
│                 │     │              │                      │
│  ┌──────────┐ ┌┴─────┴──┐ ┌────────┴───┐ ┌────────────┐  │
│  │ Board    │ │ Object   │ │ Cursor     │ │ AI         │  │
│  │ Service  │ │ Service  │ │ Service    │ │ Service    │  │
│  │          │ │          │ │ (ephemeral)│ │ (LLM call) │  │
│  └────┬─────┘ └────┬─────┘ └────────────┘ └─────┬──────┘  │
│       │            │                             │         │
│       │     ┌──────┴──────┐              ┌───────┴───────┐ │
│       │     │ Persistence │              │ LLM Client    │ │
│       │     │ (1s flush)  │              │ (Anthropic/   │ │
│       │     └──────┬──────┘              │  OpenAI)      │ │
│       │            │                     └───────────────┘ │
│  ┌────┴────────────┴──────────────────────────────────┐    │
│  │              PostgreSQL (Neon)                      │    │
│  │  users │ boards │ board_objects │ frames │ sessions │    │
│  └────────────────────────────────────────────────────┘    │
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
| `session` | `connected` | Server → client on WS connect |
| `board` | `join`, `part`, `create`, `list`, `get`, `delete`, `state` | Board lifecycle |
| `object` | `create`, `created`, `update`, `updated`, `delete`, `deleted` | Object mutations + broadcasts |
| `cursor` | `move`, `moved` | Ephemeral cursor positions |
| `ai` | `prompt` | Natural language → tool calls → mutations |

## Real-Time Sync

**Last-write-wins (LWW)**, server-authoritative:

- Server is the clock — client `ts` is ignored, server assigns from monotonic clock
- Per-object `version` (monotonic integer) — stale updates rejected
- Cursor/presence is ephemeral — broadcast only, never persisted
- Debounced persistence: dirty objects flushed to Postgres every 1 second
- On disconnect: auto-reconnect, re-join board, receive full state snapshot

## AI Agent

LLM tool calling via `ai:prompt` syscall. AI mutations flow through the same `object_service`
as human actions — all users see results in real-time.

**9 tools** (per spec):
`createStickyNote`, `createShape`, `createFrame`, `createConnector`,
`moveObject`, `resizeObject`, `updateText`, `changeColor`, `getBoardState`

**Complex commands:** "Create a SWOT analysis", "Build a user journey map",
"Set up a retrospective board" — multi-step execution with streaming results.

**Provider:** Anthropic Claude (default) or OpenAI, config-driven via `LLM_PROVIDER` env var.

## Design

Retro-scientific "Field Survey Terminal" aesthetic. See [docs/DESIGN.md](docs/DESIGN.md).

- **Fonts:** IBM Plex Mono (UI chrome) + Caveat (handwritten sticky note text)
- **Palette:** Warm earth tones — beige canvas, brown text, forest green accents
- **Rules:** Zero border-radius, zero shadows, zero gradients. Full dark mode support.

## Project Structure

```
collaboard/
├── Cargo.toml
├── .env.example
├── src/
│   ├── main.rs              # Entry: env, init, start Axum
│   ├── frame.rs             # Frame type, Status enum (from Prior)
│   ├── state.rs             # AppState, BoardObject, BoardState
│   ├── routes/
│   │   ├── mod.rs           # Router assembly
│   │   └── ws.rs            # WebSocket upgrade + frame dispatch
│   ├── services/
│   │   ├── board.rs         # Board CRUD, in-memory state, hydration
│   │   ├── object.rs        # Object mutations, LWW, broadcast
│   │   ├── cursor.rs        # Ephemeral cursor relay
│   │   ├── persistence.rs   # Debounced flush to Postgres
│   │   └── ai.rs            # LLM prompt → tool calls → mutations
│   ├── llm/
│   │   ├── mod.rs           # LlmClient trait + provider dispatch
│   │   ├── anthropic.rs     # Anthropic Messages API
│   │   ├── openai.rs        # OpenAI Chat Completions API
│   │   ├── tools.rs         # Tool definitions (provider-agnostic)
│   │   └── types.rs         # Shared LLM types
│   └── db/
│       ├── mod.rs           # Pool init
│       └── migrations/      # SQL migrations (sqlx)
├── client/                  # React frontend (TBD)
└── docs/
    ├── DESIGN.md            # Design system spec
    ├── PRE-SEARCH.md        # Architecture pre-search
    └── PRE-SEARCH.pdf
```

## Setup

### Prerequisites

- Rust 1.85+ (see `rust-toolchain.toml`)
- PostgreSQL (or Neon connection string)
- GitHub OAuth App ([register here](https://github.com/settings/applications/new))
- Anthropic or OpenAI API key

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

```bash
cargo install sqlx-cli --no-default-features --features postgres
sqlx migrate run
```

### Run

```bash
# Terminal 1: Rust backend
cargo watch -x run

# Terminal 2: React frontend (once client/ exists)
cd client && bun install && bun run dev
```

Backend serves on `:3000`. Vite dev server proxies `/api` to the backend.

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

## References

- [Architecture Pre-Search](docs/PRE-SEARCH.md)
- [Design System](docs/DESIGN.md)
- Prior project (private, architecture reference): `~/github/ianzepp/prior/`
