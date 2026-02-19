# CollabBoard

A real-time collaborative whiteboard where multiple users draw, chat, and let AI rearrange things on a shared canvas — built entirely in Rust, from the server all the way down to the browser.

## Why This Exists

This is a one-week sprint project for [The Gauntlet](https://www.yourfirstclient.com/the-gauntlet). The brief: build a collaborative whiteboard with AI integration. The twist I gave myself: do it all in Rust — server, frontend, canvas engine, wire protocol — no JavaScript runtime anywhere in the stack.

## What It Does

**Draw together in real time.** Open a board, grab a tool, start sketching. Rectangles, ellipses, diamonds, stars, lines, arrows, sticky notes, frames that group objects — the usual whiteboard toolkit. Other users on the same board see every stroke as it happens.

**Talk to the AI.** Type a prompt like "arrange these sticky notes in a grid" or "add a red diamond labeled URGENT." The AI reads your board state, decides which tools to call, and mutates objects directly. You watch it happen live.

**Chat, follow, rewind.** Each board has persistent chat. You can follow another user's camera (pan, zoom, rotation sync) or jump to their viewport. Savepoints let you rewind the board to an earlier state.

## Tech Stack

- **Rust** end to end (edition 2024, compiled to WASM for the browser)
- **Axum** 0.8 + **SQLx** 0.8 + **PostgreSQL**
- **Leptos** 0.8 (SSR + WASM hydration — zero JavaScript runtime)
- **Prost** for protobuf frame encoding
- **GitHub OAuth** for auth, session cookies + one-time WS tickets
- **Anthropic** or **OpenAI** for AI features (configurable)
- **Docker Compose** for local development

## The Crates

Five Rust crates in a Cargo workspace. Each one has a job.

---

### `server` — The Backend

Axum HTTP server, WebSocket hub, and persistence layer. 37 source files, roughly half of which are dedicated test modules.

**WebSocket dispatch** is the core of the server. Every message is a `Frame` (see the `frames` crate below), and the server routes by syscall prefix — `board:*`, `object:*`, `cursor:*`, `chat:*`, `ai:*`. Handler functions never touch the socket directly. Instead they return an `Outcome` enum — `Broadcast`, `Reply`, `ReplyStream`, `BroadcastExcludeSender`, and a few others — and a single dispatch layer decides who gets what. This keeps handlers pure and testable.

**Two-speed persistence.** Board objects and frame events take separate paths to Postgres, each tuned to their traffic pattern:
- *Object dirty flush* — a 100ms interval loop snapshots all changed objects and batch-upserts them, with a version guard so objects modified again during I/O stay dirty for the next cycle.
- *Frame log queue* — a bounded async channel (8,192 capacity) with a batched writer that flushes up to 128 frames per transaction every 5ms. Ephemeral frames (cursors, drags) are never enqueued.

**AI integration** runs a tool-call loop: snapshot the board, build a system prompt with all current objects inlined, then iterate up to 10 LLM turns. Nine tools — `createStickyNote`, `createShape`, `createFrame`, `createConnector`, `moveObject`, `resizeObject`, `updateText`, `changeColor`, `getBoardState` — let the AI read and mutate the board directly. Each mutation broadcasts to all peers in real time as it happens. Works with Anthropic (Claude) or OpenAI-compatible backends.

**Auth** uses GitHub OAuth with session cookies. WebSocket upgrades require a single-use ticket consumed via `DELETE ... RETURNING` — the ticket row is gone after one use, so replay is impossible by construction.

---

### `client` — The Frontend

A Leptos 0.8 application that renders on the server (SSR) and hydrates in the browser as a WASM binary. Pure Rust, no TypeScript, no JS framework. 46 source files across pages, components, state, and networking.

**Pages:** Dashboard (board grid with canvas-rendered preview thumbnails), Board (the workspace), Login (GitHub OAuth).

**State** flows through eight `RwSignal` contexts — auth, board objects, board list, UI preferences, chat, AI conversation, canvas telemetry, and the frame sender — provided at the app root so every component can subscribe without prop drilling.

**WebSocket lifecycle** is a persistent connection loop with exponential backoff (1s to 10s ceiling). On connect, it fetches a one-time auth ticket via REST, opens the socket, and runs three concurrent tasks: an outbound sender, an inbound dispatcher, and a 20-second heartbeat that doubles as a presence refresh.

**`CanvasHost`** is where the Rust/WASM boundary lives. It mounts a `canvas::Engine` into a `<canvas>` element, feeds it object snapshots from reactive state, routes pointer/wheel/keyboard events through the engine, and translates the returned `Action` values into outbound protocol frames. Incoming peer drag events are smoothed with a three-tier lerp based on inter-frame timing. Cursor presence is broadcast with deadband filtering to minimize network chatter.

**Viewport rotation** is controlled by a draggable compass widget with cardinal snapping (within 6 degrees), shift-snap at 15-degree steps, and N/E/S/W jump buttons. Follow-camera sync includes center, zoom, and rotation.

---

### `canvas` — The Engine

A from-scratch 2D whiteboard engine. Compiles to native Rust for testing, compiles to WASM for the browser. 382 tests, zero browser dependencies in the core logic.

The key design decision: `EngineCore` contains all application logic — document mutations, camera math, gesture state transitions — with no dependency on `web-sys` or `wasm-bindgen`. The WASM `Engine` wrapper just holds an `HtmlCanvasElement`, forwards DOM events to `EngineCore`, and calls `render()`. This is why 382 tests run in a normal `cargo test` without a browser.

**Document model** (`doc`). A `DocStore` backed by a `HashMap` of `BoardObject` entries with z-ordered iteration. Eight shape kinds: Rect, Ellipse, Diamond, Star, Line, Arrow, Frame (groups children), and Youtube (embedded video tile). `PartialBoardObject` supports sparse field updates with JSON-level prop merging.

**Camera** (`camera`). An infinite canvas with pan, zoom (0.1x–10x), and viewport rotation. `screen_to_world` and `world_to_screen` handle the full rotation + scale + translation pipeline. Zoom-toward-cursor keeps the world point under the pointer fixed.

**Hit testing** (`hit`). 101 dedicated geometry tests. Runs in two passes: selected-object handles first (resize anchors, rotation handle, edge endpoints), then all objects in reverse z-order. Each shape type has its own containment math — unit-circle test for ellipses, taxicab norm for diamonds, 10-vertex ray-cast for stars. All tests operate in local (rotation-cancelled) space.

**Input state machine** (`input` + `engine`). Seven gesture states: Idle, Panning, DraggingObject, DrawingShape, ResizingObject, RotatingObject, DraggingEdgeEndpoint. Pointer events drive transitions; the engine returns `Action` values — `ObjectCreated`, `ObjectUpdated`, `ObjectDeleted`, `EditTextRequested`, `SetCursor`, `RenderNeeded` — and the host decides what to do with them.

**Renderer** (`render`). Canvas2D drawing in four layers: clear + transform, dot grid (hidden below 0.2x zoom), objects in z-order, and selection UI (handles, rotation knob). All fallible Canvas2D calls propagate as `Result`.

---

### `frames` — The Wire Protocol

A small, shared crate that both `server` and `client` depend on. It defines one type — `Frame` — and two functions — `encode_frame` and `decode_frame`.

A `Frame` carries an id, optional parent id, timestamp, board id, sender, syscall name, status, and a flexible `serde_json::Value` data payload. The status lifecycle is `Request → Item* → Done | Error | Cancel`, where `Item` enables streaming responses (e.g., `board:join` streams all existing objects as individual items before a final `Done` with the count).

On the wire, frames are binary protobuf via Prost. The `data` field round-trips through a recursive `serde_json::Value ↔ prost_types::Value` conversion. 13 tests cover encoding round-trips, malformed input, edge cases like NaN normalization, and JSON serialization of status variants.

---

### `perf` — The Load Harness

End-to-end performance tests that hit a live running server over real HTTP and WebSocket connections. Three benchmark scenarios:

- **Round-trip latency** — fires 200 sequential `board:list` requests and reports min/max/avg/p50/p95/p99 and throughput.
- **Board complexity** — creates boards with 100, 500, and 1,000 objects to measure how creation time scales.
- **Concurrent users** — spawns 25 parallel clients on the same board, synchronized with a barrier, each firing 20 requests simultaneously.

Auth bootstrapping supports three modes: a pre-issued WS ticket, a session token that mints tickets via REST, or a dev bypass endpoint for local testing. Results print as both a human-readable table and a `JSON:` line for CI pipelines.

All tests are `#[ignore]` — run them with `cargo test -p perf -- --ignored --nocapture` against a live server.

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

681 tests across the workspace:

```bash
cargo test -p canvas --lib   # 382 canvas engine tests
cargo test -p server         # 204 server tests
cargo test -p client         # 80 frontend component tests
cargo test -p frames         # 15 wire protocol codec tests
cargo fmt --all && cargo clippy -p client -p server --all-targets
```

## Environment Variables

**Required:** `DATABASE_URL`

**For GitHub login:** `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`, `GITHUB_REDIRECT_URI`

**For AI features:** `LLM_PROVIDER` (`anthropic` or `openai`), `LLM_MODEL`, and the corresponding `ANTHROPIC_API_KEY` or `OPENAI_API_KEY`

See `.env.example` for the full list.
