# CollabBoard

A real-time collaborative whiteboard where multiple users draw, chat, and let AI rearrange things on a shared canvas — built entirely in Rust, from the server all the way down to the browser.

## Why This Exists

This is a one-week sprint project for [The Gauntlet](https://www.yourfirstclient.com/the-gauntlet). The brief: build a collaborative whiteboard with AI integration. The twist I gave myself: do it all in Rust — server, frontend, canvas engine, wire protocol — no JavaScript runtime anywhere in the stack.

## What It Does

**Draw together in real time.** Open a board, grab a tool, start sketching. Rectangles, ellipses, diamonds, stars, lines, arrows, text elements, frames that group objects, and embedded YouTube tiles — the full whiteboard toolkit. Other users on the same board see every stroke as it happens.

**Talk to the AI.** Type a prompt like "arrange these sticky notes in a grid" or "add a red diamond labeled URGENT." The AI reads your board state, decides which tools to call, and mutates objects directly. You watch it happen live.

**Chat, follow, rewind.** Each board has persistent chat. You can follow another user's camera (pan, zoom, rotation sync) or jump to their viewport. Savepoints let you rewind the board to an earlier state.

## Live Demo

Deployed on Railway (as of 2026-02-20, URL may change):
**https://gauntlet-week-1-production.up.railway.app/**

## Prerelease Video Demos

Week 1 Day 2 (MVP):
**https://www.loom.com/share/613a53b528ae431a81873c41583a11c2**

Week 1 Day 5 (Early Release):
**https://www.loom.com/share/5d38b03c2450418ab11c29e0bbd28e52**

## Tech Stack

- **Rust** end to end (edition 2024, compiled to WASM for the browser)
- **Axum** 0.8 + **SQLx** 0.8 + **PostgreSQL**
- **Leptos** 0.8 (SSR + WASM hydration — zero JavaScript runtime)
- **Prost** 0.13 for protobuf frame encoding
- **GitHub OAuth** + **email access codes** for auth, session cookies + one-time WS tickets
- **Anthropic** or **OpenAI** for AI features (configurable)
- **Docker Compose** for local development

## Project Stats

*Generated with `scripts/stats-all.sh` on 2026-02-21. 382 commits over 6 days (~64/day).*

### Lines of Code

| Crate | Source | Test | Total | Test% |
|-------|-------:|-----:|------:|------:|
| server | 8,241 | 5,765 | 14,006 | 41.2% |
| canvas | 3,188 | 5,709 | 8,897 | 64.2% |
| client | 13,165 | 2,749 | 15,914 | 17.3% |
| frames | 218 | 392 | 610 | 64.3% |
| perf | 460 | 238 | 698 | 34.1% |
| traces | 549 | 904 | 1,453 | 62.2% |
| **Total** | **25,821** | **15,757** | **41,578** | **37.9%** |

### Functions

| Crate | Source Files | Test Files | Pub Fn | Priv Fn | Total |
|-------|------------:|-----------:|-------:|--------:|------:|
| server | 25 | 19 | 58 | 75 | 133 |
| canvas | 8 | 6 | 88 | 56 | 144 |
| client | 64 | 33 | 146 | 116 | 262 |
| frames | 1 | 1 | 3 | 5 | 8 |
| perf | 1 | 1 | 4 | 6 | 10 |
| traces | 1 | 2 | 17 | 7 | 24 |
| **Total** | **100** | **62** | **316** | **265** | **581** |

### Tests

| Crate | #[test] | Passed | Failed |
|-------|--------:|-------:|-------:|
| server | 202 | 276 | 0 |
| canvas | 409 | 409 | 0 |
| client | 214 | 214 | 0 |
| frames | 26 | 26 | 0 |
| perf | 0 | 0 | 0 |
| traces | 64 | 64 | 0 |
| **Total** | **915** | **989** | **0** |

### Code Coverage

| Crate | Covered | Total | Line% |
|-------|--------:|------:|------:|
| server | 2,602 | 5,388 | 48.3% |
| canvas | 1,122 | 1,885 | 59.5% |
| client | 1,368 | 5,586 | 24.5% |
| frames | 86 | 174 | 49.4% |
| traces | 317 | 318 | 99.7% |
| **Total** | **5,495** | **13,351** | **41.2%** |

### Documentation Coverage

| Crate | Code Lines | `///` Doc | `//!` Mod | `//` Comment | Blank | Doc% |
|-------|----------:|----------:|----------:|-------------:|------:|-----:|
| server | 6,595 | 394 | 202 | 238 | 812 | 9.0% |
| canvas | 2,389 | 309 | 60 | 91 | 339 | 15.4% |
| client | 11,056 | 729 | 373 | 53 | 954 | 10.0% |
| frames | 162 | 31 | 5 | 2 | 18 | 22.2% |
| perf | 335 | 75 | 6 | 1 | 43 | 24.2% |
| traces | 395 | 93 | 4 | 0 | 57 | 24.6% |
| **Total** | **20,932** | **1,631** | **650** | **385** | **2,223** | **10.9%** |

### Dependencies

| Crate | Deps | Dev Deps | Total |
|-------|-----:|---------:|------:|
| server | 24 | 1 | 25 |
| canvas | 5 | 0 | 5 |
| client | 21 | 0 | 21 |
| frames | 5 | 0 | 5 |
| perf | 8 | 0 | 8 |
| traces | 3 | 0 | 3 |
| **Total** | **66** | **1** | **67** |

### Most-Changed Files

| Commits | File |
|--------:|------|
| 62 | `server/src/routes/ws.rs` |
| 36 | `server/src/services/ai.rs` |
| 36 | `client/src/components/canvas_host.rs` |
| 26 | `client/src/net/frame_client.rs` |
| 25 | `client/src/pages/board.rs` |
| 25 | `client-rust/src/components/canvas_host.rs` |
| 23 | `client-rust/src/net/frame_client.rs` |
| 22 | `server/src/routes/ws_test.rs` |
| 20 | `canvas/src/engine.rs` |
| 18 | `server/src/services/board.rs` |

## The Crates

Six Rust crates in a Cargo workspace. Each one has a job.

---

### `server` — The Backend

Axum HTTP server, WebSocket hub, and persistence layer.

**WebSocket dispatch** is the core of the server. Every message is a `Frame` (see the `frames` crate below), and the server routes by syscall prefix — `board:*`, `object:*`, `cursor:*`, `chat:*`, `ai:*`. Handler functions never touch the socket directly. Instead they return an `Outcome` enum — `Broadcast`, `Reply`, `ReplyStream`, `BroadcastExcludeSender`, and a few others — and a single dispatch layer decides who gets what. This keeps handlers pure and testable.

**Two-speed persistence.** Board objects and frame events take separate paths to Postgres, each tuned to their traffic pattern:
- *Object dirty flush* — a 100ms interval loop snapshots all changed objects and batch-upserts them, with a version guard so objects modified again during I/O stay dirty for the next cycle.
- *Frame log queue* — a bounded async channel (8,192 capacity) with a batched writer that flushes up to 128 frames per transaction every 5ms. Ephemeral frames (cursors, drags) are never enqueued.

**AI integration** runs a tool-call loop: snapshot the board, build a system prompt with all current objects inlined, then iterate up to 10 LLM turns. Nine tools — `createStickyNote`, `createShape`, `createFrame`, `createConnector`, `moveObject`, `resizeObject`, `updateText`, `changeColor`, `getBoardState` — let the AI read and mutate the board directly. Each mutation broadcasts to all peers in real time as it happens. Works with Anthropic (Claude) or OpenAI-compatible backends.

**Auth** supports two methods: GitHub OAuth and email access codes (6-character codes delivered via Resend or returned in the response for dev workflows). WebSocket upgrades require a single-use ticket consumed via `DELETE ... RETURNING` — the ticket row is gone after one use, so replay is impossible by construction.

---

### `client` — The Frontend

A Leptos 0.8 application that renders on the server (SSR) and hydrates in the browser as a WASM binary. Pure Rust, no TypeScript, no JS framework.

**Pages:** Dashboard (board grid with canvas-rendered preview thumbnails), Board (the workspace), Login (GitHub OAuth + email).

**State** flows through eight `RwSignal` contexts — auth, board objects, board list, UI preferences, chat, AI conversation, canvas telemetry, and the frame sender — provided at the app root so every component can subscribe without prop drilling.

**WebSocket lifecycle** is a persistent connection loop with exponential backoff (1s to 10s ceiling). On connect, it fetches a one-time auth ticket via REST, opens the socket, and runs three concurrent tasks: an outbound sender, an inbound dispatcher, and a 20-second heartbeat that doubles as a presence refresh.

**`CanvasHost`** is where the Rust/WASM boundary lives. It mounts a `canvas::Engine` into a `<canvas>` element, feeds it object snapshots from reactive state, routes pointer/wheel/keyboard events through the engine, and translates the returned `Action` values into outbound protocol frames. Incoming peer drag events are smoothed with a three-tier lerp based on inter-frame timing. Cursor presence is broadcast with deadband filtering to minimize network chatter.

**Viewport rotation** is controlled by a draggable compass widget with cardinal snapping (within 6 degrees), shift-snap at 15-degree steps, and N/E/S/W jump buttons. Follow-camera sync includes center, zoom, and rotation.

---

### `canvas` — The Engine

A from-scratch 2D whiteboard engine. Compiles to native Rust for testing, compiles to WASM for the browser. Zero browser dependencies in the core logic.

The key design decision: `EngineCore` contains all application logic — document mutations, camera math, gesture state transitions — with no dependency on `web-sys` or `wasm-bindgen`. The WASM `Engine` wrapper just holds an `HtmlCanvasElement`, forwards DOM events to `EngineCore`, and calls `render()`. This is why the full test suite runs in a normal `cargo test` without a browser.

**Document model** (`doc`). A `DocStore` backed by a `HashMap` of `BoardObject` entries with z-ordered iteration. Nine shape kinds: Rect, Ellipse, Diamond, Star, Line, Arrow, Text, Frame (groups children), and Youtube (embedded video tile). `PartialBoardObject` supports sparse field updates with JSON-level prop merging.

**Camera** (`camera`). An infinite canvas with pan, zoom (0.1x–10x), and viewport rotation. `screen_to_world` and `world_to_screen` handle the full rotation + scale + translation pipeline. Zoom-toward-cursor keeps the world point under the pointer fixed.

**Hit testing** (`hit`). Dedicated geometry tests. Runs in two passes: selected-object handles first (resize anchors, rotation handle, edge endpoints), then all objects in reverse z-order. Each shape type has its own containment math — unit-circle test for ellipses, taxicab norm for diamonds, 10-vertex ray-cast for stars. All tests operate in local (rotation-cancelled) space.

**Input state machine** (`input` + `engine`). Seven gesture states: Idle, Panning, DraggingObject, DrawingShape, ResizingObject, RotatingObject, DraggingEdgeEndpoint. Pointer events drive transitions; the engine returns `Action` values — `ObjectCreated`, `ObjectUpdated`, `ObjectDeleted`, `EditTextRequested`, `SetCursor`, `RenderNeeded` — and the host decides what to do with them.

**Renderer** (`render`). Canvas2D drawing in four layers: clear + transform, dot grid (hidden below 0.2x zoom), objects in z-order, and selection UI (handles, rotation knob). All fallible Canvas2D calls propagate as `Result`.

---

### `frames` — The Wire Protocol

A small, shared crate that both `server` and `client` depend on. It defines one type — `Frame` — and two functions — `encode_frame` and `decode_frame`.

A `Frame` carries an id, optional parent id, timestamp, board id, sender, syscall name, status, and a flexible `serde_json::Value` data payload. The status lifecycle is `Request → Item* → Done | Error | Cancel`, where `Item` enables streaming responses (e.g., `board:join` streams all existing objects as individual items before a final `Done` with the count).

On the wire, frames are binary protobuf via Prost 0.13. The `data` field round-trips through a recursive `serde_json::Value ↔ prost_types::Value` conversion.

---

### `traces` — Observability Primitives

Shared trace and event primitives for CollabBoard's observability UI. Intentionally avoids UI framework dependencies so it can be used by `client` (Leptos) or any other renderer.

Provides syscall prefix-to-display mapping, default trace filtering policy (hides `cursor:*` and `item` frames by default), frame-to-session grouping by parent chain, request/done span pairing for waterfall timing, aggregate metrics (counts, errors, pending), and sub-label extraction for common syscall payloads.

---

### `perf` — The Load Harness

End-to-end performance tests that hit a live running server over real HTTP and WebSocket connections. Three benchmark scenarios:

- **Round-trip latency** — fires 200 sequential `board:list` requests and reports min/max/avg/p50/p95/p99 and throughput.
- **Board complexity** — creates boards with 100, 500, and 1,000 objects to measure how creation time scales.
- **Concurrent users** — spawns 25 parallel clients on the same board, synchronized with a barrier, each firing 20 requests simultaneously.

Auth bootstrapping supports three modes: a pre-issued WS ticket, a session token that mints tickets via REST, or a dev bypass endpoint for local testing. Results print as both a human-readable table and a `JSON:` line for CI pipelines.

All tests are `#[ignore]` — run them with `cargo test -p perf -- --ignored --nocapture` against a live server.

---

### `cli` — API + WS Automation

A command-line client for scripting against CollabBoard over HTTP and websocket frames.

- REST subcommands target:
- `GET /api/board`
- `GET /api/board/:id`
- `GET /api/board/:id/objects`
- `POST /api/board/:id/objects`
- `GET /api/board/:id/objects/:object_id`
- `PATCH /api/board/:id/objects/:object_id`
- `DELETE /api/board/:id/objects/:object_id`
- Websocket subcommand `ws stream-create` joins a board and mass-streams `object:create` frames.

Run examples:

```bash
cargo run -p cli -- api board list --session-token "$SESSION"
cargo run -p cli -- api object create <board-id> --data '{"kind":"sticky_note","x":0,"y":0,"width":160,"height":100,"rotation":0,"props":{"text":"hello"}}' --session-token "$SESSION"
cargo run -p cli -- ws stream-create --board-id <board-id> --count 1000 --wait-for-ack false --session-token "$SESSION"
```

The CLI also reads `COLLAB_BASE_URL`, `COLLAB_SESSION_TOKEN`, and `COLLAB_WS_TICKET`.

---

## Quick Start

```bash
# clone and configure
cp .env.example .env
# edit .env with your DATABASE_URL and optionally GitHub OAuth + LLM keys

# run with docker (recommended)
docker compose up --build

# or run natively
cargo leptos build && cargo run -p server
```

The app serves at `http://localhost:3000`. Migrations run automatically on startup.

## Testing

```bash
cargo test -p canvas --lib   # canvas engine tests (no browser required)
cargo test -p server         # server tests
cargo test -p client --lib   # frontend component tests
cargo test -p frames --lib   # wire protocol codec tests
cargo test -p traces --lib   # observability primitive tests
cargo fmt --all && cargo clippy -p client -p server --all-targets
```

## Environment Variables

**Required:** `DATABASE_URL`

**For GitHub login:** `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`, `GITHUB_REDIRECT_URI`

**For email login:** `AUTH_EMAIL_CODE_IN_RESPONSE` (set `false` in production), `RESEND_API_KEY`, `RESEND_FROM`

**For AI features:** `LLM_PROVIDER` (`anthropic` or `openai`), `LLM_MODEL`, and the corresponding `ANTHROPIC_API_KEY` or `OPENAI_API_KEY`

See `.env.example` for the full list including tuning knobs for WS queue capacity, persistence intervals, AI rate limits, and frame batch sizes.
