# CollabBoard

Real-time collaborative whiteboard with AI-assisted editing.

## Stack

- Rust workspace with 5 crates: `server`, `client`, `canvas`, `frames`, `perf`
- Backend: Axum + SQLx + PostgreSQL + WebSocket frame protocol
- Frontend: Leptos SSR + WASM hydration
- Canvas engine: Rust `canvas` crate (imperative engine consumed by `client`)
- Shared wire model: `frames` crate (protobuf binary frame codec + types)
- Perf harness: `perf` crate (live E2E perf scenarios)
- Optional AI providers: Anthropic or OpenAI

## Workspace Layout

```text
gauntlet-week-1/
├── Cargo.toml                # Workspace config + cargo-leptos metadata
├── server/                   # Axum server crate (name: server)
├── client/                   # Leptos frontend crate (name: client)
├── canvas/                   # Canvas engine crate (name: canvas)
├── frames/                   # Shared frame types + protobuf codec crate
├── perf/                     # Live end-to-end perf test harness crate
├── public/                   # Static assets copied into Leptos site build
├── Dockerfile
├── docker-compose.yml
└── run-dev.sh
```

## Runtime Architecture

- Browser connects to `server` over WebSocket using binary protobuf frames.
- `server` dispatches frames by prefix (`board:*`, `object:*`, `cursor:*`, `chat:*`, `ai:*`).
- In-memory board state is authoritative for live collaboration.
- Object deltas are persisted to Postgres on a periodic dirty-flush loop.
- Frame events are persisted through a bounded async queue + batched writer.
- `frames` owns shared wire types/encoding; `client` maintains UI state and websocket lifecycle; `canvas` handles rendering/gestures.

## Core Features

- Board CRUD and switching
- Real-time object create/update/delete sync
- Presence and cursor broadcast
- Persistent board chat
- AI prompt -> tool-call -> board mutation flow
- Savepoints / rewind shelf support
- GitHub OAuth login + session cookies + one-time WS tickets

## Frame Protocol (Summary)

Each message is a frame with:

- `id`, `parent_id`, `ts`
- `syscall`, `status`
- `board_id`, `from`
- `data` (JSON-like payload encoded as protobuf `Value`)

Status flow:

- `request -> done`
- `request -> error`
- Optional `item` streaming where applicable

## Prerequisites

- Rust toolchain (see `rust-toolchain.toml`)
- PostgreSQL
- Docker + Docker Compose (recommended local run path)
- Optional: GitHub OAuth app credentials
- Optional: Anthropic/OpenAI API key for AI features

## Environment

```bash
cp .env.example .env
```

Set at minimum:

- `DATABASE_URL`
- `GITHUB_CLIENT_ID`
- `GITHUB_CLIENT_SECRET`
- `GITHUB_REDIRECT_URI`

Optional AI settings:

- `LLM_PROVIDER`
- `LLM_MODEL`
- `LLM_API_KEY_ENV`
- `ANTHROPIC_API_KEY` or `OPENAI_API_KEY`

## Run Locally

### Docker (recommended)

```bash
./run-dev.sh --build
```

or:

```bash
docker compose up --build
```

App: `http://localhost:3000`

### Rust-only workflow

Install cargo-leptos, then build/run from workspace root:

```bash
cargo leptos build
cargo run -p server
```

## Migrations

Migrations are applied automatically on startup.

Run only migrations:

```bash
cargo run -p server -- --migrate-only
```

## Testing

Run primary crate suites:

```bash
cargo test -p client
cargo test -p server
cargo test -p canvas --lib
cargo test -p frames
```

Formatting/linting:

```bash
cargo fmt --all
cargo clippy -p client -p server --all-targets
```

### Performance Tests

The `perf` crate contains live end-to-end communication benchmarks:

- WS request/response round-trip latency
- Board complexity/object create scaling
- Mass-user concurrent load

Run (against a running server):

```bash
cargo test -p perf -- --ignored --nocapture
```

Required auth context:

- Preferred: set `PERF_SESSION_TOKEN`
- Optional (single-client only): set `PERF_WS_TICKET`

Key perf env vars:

- `PERF_BASE_URL` (default `http://127.0.0.1:3000`)
- `PERF_BASELINE_REQUESTS` (default `200`)
- `PERF_COMPLEXITY_COUNTS` (default `100,500,1000`)
- `PERF_MASS_USERS` (default `25`)
- `PERF_MASS_REQUESTS_PER_USER` (default `20`)

## API Endpoints

- `GET /auth/github`
- `GET /auth/github/callback`
- `GET /api/auth/me`
- `POST /api/auth/logout`
- `POST /api/auth/ws-ticket`
- `GET /api/users/:id/profile`
- `GET /api/ws?ticket=...` (websocket upgrade)
- `GET /healthz`

## Deployment Notes

The provided `Dockerfile` builds and serves:

- `server` binary
- Leptos site output at `/app/site`

Runtime defaults:

- `STATIC_DIR=/app/site`
- `LEPTOS_SITE_ROOT=/app/site`

## References

- `docs/PRE-SEARCH.md`
- `docs/DESIGN.md`
- `client/PLAN.md`
