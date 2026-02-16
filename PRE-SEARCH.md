# CollabBoard Pre-Search Document

---

## Phase 1: Define Your Constraints

### 1. Scale & Load Profile

**Decision:** Design for 5-20 concurrent users per board, 1-5 boards active simultaneously, ~50 total registered users at launch. In 6 months: irrelevant — this is a portfolio/program project, not a startup.

**Why:** This is a Gauntlet program deliverable with a 7-day build window. The performance targets (5+ concurrent users, 500+ objects, 60 FPS) define the real ceiling. Designing beyond that is wasted time.

**Traffic pattern:** Spiky. Demo-driven — load hits when you present or when evaluators test it. Zero traffic otherwise.

**Real-time requirements:** Yes, WebSockets are non-negotiable. The spec demands <100ms object sync and <50ms cursor sync. SSE won't cut it for bidirectional cursor streaming. Use a single WebSocket connection per client multiplexing all message types as frames.

**Cold start tolerance:** Moderate. A 2-3 second cold start on first board load is acceptable. Evaluators won't be hitting it cold repeatedly.

**Key tradeoff:** Ignoring horizontal scaling entirely. Single-process WebSocket state means one server handles all connections. This breaks at ~100 concurrent users. That's fine — we'll never hit it.

---

### 2. Budget & Cost Ceiling

**Decision:** $0/month infrastructure target, $20/month hard ceiling (LLM API costs are the only real variable).

**Why:** Program project. Use free tiers aggressively. Fly.io free tier for compute. Neon or Supabase free tier for Postgres. The only meaningful spend is Anthropic API calls for the AI agent.

**Where to trade money for time:**
- **LLM calls:** Pay for a smarter model (Claude Sonnet) rather than spending days prompt-engineering a cheaper one. Budget ~$5-10 for development iteration and demo usage.
- **Deployment:** Fly.io free tier. Don't pay for managed WebSocket services — run your own.
- **Database:** Neon free tier (0.5 GB, branching) or Supabase free tier (500 MB). Both provide managed Postgres with no cost at this scale.

**Key tradeoff:** Running WebSockets and HTTP on a single Fly.io free-tier VM means a deploy = brief downtime. Acceptable for a demo.

---

### 3. Time to Ship

**Decision:** 24-hour MVP, then 6 remaining days for full feature set. Speed-to-market is the only priority. Maintainability is irrelevant.

**Why:** The 24-hour MVP gate is a hard requirement. Ship or fail. After that, ~6 days to layer on connectors, frames, multi-select, AI agent, and polish. There is no "long-term" — this project's useful life is the evaluation period.

**Concrete time allocation:**

| Block | Hours | Deliverable |
|-------|-------|-------------|
| Hours 0-4 | 4h | Rust project scaffold, Frame type, kernel router, Postgres schema |
| Hours 4-8 | 4h | Axum gateway: `/api/auth`, `/api/ws` with frame relay |
| Hours 8-12 | 4h | Board subsystem: object CRUD via frames, persistence |
| Hours 12-16 | 4h | React+Konva frontend: canvas, sticky notes, shapes, WS client |
| Hours 16-20 | 4h | Real-time sync: cursor broadcast, object sync, presence |
| Hours 20-22 | 2h | Auth (anonymous sessions), deploy to Fly.io |
| Hours 22-24 | 2h | Buffer for the inevitable fire |
| Days 2-3 | 16h | Connectors, text, multi-select, rotate, board CRUD UI |
| Days 4-5 | 16h | AI agent subsystem (LLM tool calling, 6+ commands) |
| Days 6-7 | 16h | Copy/paste, disconnect recovery, polish, performance |

**Key tradeoff:** No tests until day 6 at earliest. Manual testing only during the sprint. The cost of test infrastructure exceeds its value at this timeline.

---

### 4. Compliance & Regulatory Needs

**Decision:** None. Zero compliance work.

**Why:** Program project, not a production SaaS. No real user data beyond demo accounts. Every hour spent on compliance is an hour stolen from features that get evaluated.

**What you actually do:**
- Don't store passwords in plaintext (use argon2).
- Don't commit secrets to git.
- That's it.

**Key tradeoff:** If an evaluator asks "what about GDPR?" the answer is "this is a technical demo — here's where I'd add consent management and data deletion endpoints." Knowing the answer is more valuable than implementing it.

---

### 5. Team & Skill Constraints

**Decision:** Solo build. Rust backend (porting proven patterns from Prior), React+Konva frontend.

**Why:** The Prior project provides a battle-tested monokernel, frame protocol, and Axum gateway pattern. Porting these to a new domain (whiteboard) is faster than building from scratch in any stack — the architecture is already debugged.

**Key tradeoff:** Rust is slower to iterate on than TypeScript for the backend. But the patterns are already proven, and the type system catches entire classes of bugs at compile time that would be runtime surprises in TS. For a solo dev, fewer runtime surprises is worth the compilation cost.

---

## Phase 2: Architecture Discovery

### 6. Hosting & Deployment

**Options considered:**
1. **Fly.io** — Docker containers, native WebSocket support, long-lived processes, built-in scaling.
2. **Railway** — Simple container hosting, easy deploys, but less control.
3. **VPS** — Full control, cheap, but manual ops burden.

**Decision:** Fly.io

**Why:** Purpose-built for long-lived WebSocket connections on container infrastructure with zero DevOps overhead. `fly deploy` from a Dockerfile gives you TLS and health checks in one command. Free tier (3 shared VMs) is sufficient. Rust's small binary and low memory footprint fit the free tier well.

**CI/CD:** None for a 7-day sprint. `fly deploy` from the terminal.

**Key tradeoff:** Locked into Fly's orchestration. Acceptable for a demo.

---

### 7. Authentication & Authorization

**Decision:** Anonymous with display name for MVP; add simple email/password or GitHub OAuth for Full phase.

**Why:** The MVP gate is 24 hours. Every minute on auth is a minute not on canvas or real-time sync. For MVP: `/api/auth/connect` generates a session UUID, client picks a display name and color. The Door pattern (from Prior) handles session lifecycle — connect, join board, disconnect.

**RBAC:** Not needed. Two roles max: board owner (can delete) and collaborator (can edit). Simple `role` column on `board_members` table.

**Multi-tenancy:** Boards shared via link. Anyone with the link can collaborate (MVP). This is the Miro/Figma model.

**Key tradeoff:** Anonymous MVP means no persistent identity across sessions. Acceptable — add real auth in days 2-3.

---

### 8. Database & Data Layer

**Options considered:**
1. **SQLite (like Prior)** — Zero infrastructure, but no concurrent write support from multiple connections.
2. **PostgreSQL (managed, Neon/Supabase)** — Proper concurrent writes, JSONB for flexible object storage, full SQL.
3. **Redis + PostgreSQL** — Redis for ephemeral state, Postgres for durable.

**Decision:** PostgreSQL (managed via Neon or Supabase free tier) + in-memory board state.

**Why:** Postgres gives proper concurrent writes, JSONB columns for flexible board object properties, and a migration path to production. The `frames` table (append-only frame log, ported from Prior) provides audit trail and history replay. Live board state still lives in-memory for performance — Postgres is the durable backing store, not the hot path.

### Schema Design

```sql
-- Append-only frame log (ported from Prior's frame_db)
CREATE TABLE frames (
    seq         BIGSERIAL PRIMARY KEY,
    ts          BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM now()) * 1000),
    id          UUID NOT NULL,
    parent_id   UUID,
    syscall     TEXT NOT NULL,
    status      TEXT NOT NULL,
    board_id    UUID,
    "from"      TEXT,
    data        JSONB NOT NULL DEFAULT '{}'
);
CREATE INDEX idx_frames_board ON frames(board_id, syscall, status);

-- Relational tables
CREATE TABLE users (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL,
    color       TEXT NOT NULL DEFAULT '#4CAF50',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE boards (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL,
    owner_id    UUID NOT NULL REFERENCES users(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE board_objects (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    board_id    UUID NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    kind        TEXT NOT NULL,  -- sticky_note, rectangle, ellipse, line, connector, text, frame
    x           DOUBLE PRECISION NOT NULL DEFAULT 0,
    y           DOUBLE PRECISION NOT NULL DEFAULT 0,
    width       DOUBLE PRECISION,
    height      DOUBLE PRECISION,
    rotation    DOUBLE PRECISION NOT NULL DEFAULT 0,
    z_index     INTEGER NOT NULL DEFAULT 0,
    props       JSONB NOT NULL DEFAULT '{}',  -- color, text, points, etc.
    created_by  UUID REFERENCES users(id),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_board_objects_board ON board_objects(board_id);
```

### Real-Time Sync Approach

**Last-write-wins (LWW) with object-level granularity.** Not CRDTs, not OT.

- The server holds authoritative state in-memory. Client sends a request frame (`object:update`), server applies it, timestamps it, persists to Postgres, broadcasts item frames to all other clients.
- If two users drag the same object, last one wins. This is how Miro works.
- Cursor positions and presence are purely ephemeral — broadcast and forget, never persisted.

### Persistence Strategy

| Data | Storage | Lifetime |
|------|---------|----------|
| Cursor positions | In-memory, broadcast via WS frames | Ephemeral |
| Presence (who's online) | In-memory HashMap per board | Ephemeral |
| Board objects (live) | In-memory HashMap per board | Duration of active session |
| Board objects (durable) | Postgres `board_objects` table | Permanent |
| Frame audit log | Postgres `frames` table | Permanent |
| Users, boards metadata | Postgres relational tables | Permanent |

On board load: hydrate in-memory state from `board_objects`. On mutation: update in-memory, persist to Postgres (debounced ~1s), broadcast frame to peers. On last user disconnect: final flush.

**Key tradeoff:** In-memory state means server restart loses unsaved changes (mitigated by 1s debounce flush). Acceptable for a 5-user demo.

---

### 9. Backend / API Architecture

**Decision:** Rust monokernel with Axum gateway (ported from Prior).

**Why:** The Prior project already has a debugged monokernel, frame protocol, and Axum WebSocket gateway. Porting these patterns to a new domain saves days of architecture work. The monokernel gives clean subsystem isolation without microservice complexity.

### Monokernel Design

```
Kernel::new()
  -> register("door")   -> Door subsystem (session lifecycle, board membership)
  -> register("board")  -> Board subsystem (CRUD, object management)
  -> register("object") -> Object subsystem (create, move, resize, delete board objects)
  -> register("cursor") -> Cursor subsystem (ephemeral position broadcast)
  -> register("ai")     -> AI subsystem (LLM tool calling)
  -> start()            -> Spawn routing loop
```

The kernel routes frames by syscall prefix. Each subsystem receives a channel endpoint. No subsystem knows about any other — they communicate only through the kernel.

### Frame Protocol

```rust
pub struct Frame {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub ts: i64,                              // millis since epoch
    pub syscall: String,                      // "object:create", "cursor:move", etc.
    pub status: Status,                       // request | item | done | error | cancel
    pub board_id: Option<Uuid>,
    pub from: Option<String>,                 // user attribution
    pub data: HashMap<String, serde_json::Value>,
}
```

Status lifecycle (identical to Prior):
- `Request -> Item* -> Done` (success with 0+ results)
- `Request -> Error` (failure)
- `Cancel` (abort by parent_id)

### API Surface

Two HTTP endpoints only:

**`/api/auth/*`** — REST endpoints for session management:
- `POST /api/auth/connect` — create anonymous session, returns `{ session, user_id }`
- `POST /api/auth/login` — email/password (Full phase)

**`/api/ws`** — Single WebSocket endpoint, bidirectional frames:

```
Client connects: GET /api/ws?token=<session_token>
Server sends:    { status: "item", syscall: "door:connected", data: { session: "uuid" } }

Client sends:    { syscall: "door:join", data: { board_id: "uuid" } }
Server sends:    { status: "item", syscall: "board:state", data: { objects: [...], users: [...] } }
Server sends:    { status: "done", syscall: "door:join" }

Client sends:    { syscall: "object:create", data: { kind: "sticky_note", x: 100, y: 200, props: { text: "Hello", color: "#FFEB3B" } } }
Server broadcasts: { status: "item", syscall: "object:created", data: { id: "uuid", kind: "sticky_note", ... } }

Client sends:    { syscall: "cursor:move", data: { x: 450, y: 300 } }
Server broadcasts: { status: "item", syscall: "cursor:moved", data: { user_id: "uuid", x: 450, y: 300 } }

Client sends:    { syscall: "ai:prompt", data: { prompt: "Create a SWOT analysis" } }
Server streams:  { status: "item", syscall: "object:created", data: { ... } }  // repeated for each object
Server sends:    { status: "done", syscall: "ai:prompt" }
```

### AI Agent Integration

The AI subsystem is registered as `"ai"` in the kernel. When it receives a request:

1. Serialize current board state from the board subsystem
2. Call Anthropic Claude with board state + user prompt + tool definitions
3. For each tool call returned, emit a request frame to the appropriate subsystem (`object:create`, `object:move`, etc.)
4. Each subsystem processes the mutation normally — persists, broadcasts to clients
5. When all tool calls complete, emit `done` on the original `ai:prompt` frame

The AI agent is just another subsystem making mutations through the kernel. No special path.

### Subsystem Syscalls

| Prefix | Syscalls | Description |
|--------|----------|-------------|
| `door` | `connect`, `disconnect`, `join`, `part` | Session lifecycle, board membership |
| `board` | `create`, `list`, `get`, `delete`, `state` | Board CRUD and state hydration |
| `object` | `create`, `update`, `delete`, `lock`, `unlock` | Board object mutations |
| `cursor` | `move` | Ephemeral cursor position broadcast |
| `ai` | `prompt` | Natural language -> tool calls -> board mutations |

**Key tradeoff:** Rust compilation is slower than TS hot-reload. Mitigated by `cargo watch` and the fact that the architecture is already proven from Prior.

---

### 10. Frontend Framework & Rendering

**Decision:** React (Vite SPA) + Konva.js (via react-konva)

**Why React:** The app shell (toolbar, sidebar, board list, presence indicators, AI chat panel) is standard UI work. React ships that fast.

**Why Konva:** Sweet spot for a whiteboard. Scene graph, built-in drag-and-drop, `Transformer` (resize/rotate handles out of the box), hit detection, event bubbling, text editing. Performance is solid for 500 objects.

### State Management

```
┌─────────────────────────────────────────┐
│  Zustand Store (board state)            │
│  - objects: Map<id, BoardObject>        │
│  - presence: Map<userId, CursorPos>     │
│  - selection: Set<objectId>             │
│  - viewport: { x, y, scale }           │
├─────────────────────────────────────────┤
│  Frame Client (WebSocket singleton)     │
│  - receives server frames               │
│  - updates Zustand store by syscall     │
│  - sends request frames to server       │
├─────────────────────────────────────────┤
│  React renders from Zustand             │
│  Konva Stage reads objects from store   │
└─────────────────────────────────────────┘
```

The Frame Client speaks the same protocol as the server. Incoming frames are dispatched by `syscall` to store update handlers. Outgoing user actions are serialized as request frames.

**Zustand** over Redux/MobX: minimal boilerplate, works outside React components (the frame client can update the store directly), selectors prevent unnecessary re-renders.

### 60 FPS Strategy

1. **Layer separation:** Static objects on one layer, actively-dragged object on another, cursors on a third.
2. **Viewport culling:** Only render objects within the visible viewport.
3. **Throttle cursor broadcasts:** 30 Hz max (every 33ms). Render remote cursors with CSS transforms (DOM overlay), not on Konva canvas.
4. **Batch store updates:** Accumulate incoming frames for 16ms (one animation frame), then apply all at once.
5. **`React.memo` everything:** Each canvas object component memoized.

**Key tradeoff:** Konva's performance ceiling is lower than PixiJS (~1000-2000 objects vs. ~50,000). For 500 objects this is fine.

---

### 11. Third-Party Integrations

**Decision:** Anthropic Claude (claude-sonnet-4-20250514) via the Rust `anthropic` crate or direct HTTP calls.

**Why:** Claude's tool-use/function-calling is reliable. Sonnet balances cost and quality: fast enough for interactive use (~1-2s), smart enough for spatial reasoning. From Rust, use `reqwest` + manual JSON serialization to the Anthropic Messages API — avoids depending on a potentially immature Rust SDK.

### AI Agent Tool Definitions (6+ commands)

| Tool | Description |
|------|-------------|
| `create_objects` | Create sticky notes, shapes, or text objects |
| `move_objects` | Reposition objects by ID |
| `update_objects` | Change properties (color, text, size) |
| `delete_objects` | Remove objects by ID |
| `organize_layout` | Arrange objects in grid, cluster, or tree |
| `summarize_board` | Read all text, produce summary as new sticky note |
| `group_by_theme` | Cluster objects by semantic similarity, color-code |

### Other External Services

- **PostgreSQL** — Neon or Supabase free tier (managed).
- **Anthropic API** — Only external API call.
- No Redis, no S3, no CDN. Static frontend assets served by Axum or a CDN fronting the Fly.io container.

### Pricing

- **Anthropic:** Sonnet is $3/M input, $15/M output. Each AI command ~$0.01-0.05. Rate limit: 10 AI commands/min/board. Budget: ~$5-10 total.
- **Fly.io:** Free tier. Cost: $0.
- **Neon/Supabase:** Free tier. Cost: $0.
- **Domain/TLS:** Fly provides `*.fly.dev` with TLS for free.

**Key tradeoff:** Single LLM provider, no fallback. Acceptable for a demo.

---

## Summary: The Full Stack

| Layer | Choice |
|-------|--------|
| **Backend runtime** | Rust |
| **HTTP framework** | Axum |
| **Architecture** | Monokernel with frame-routed subsystems (ported from Prior) |
| **Wire protocol** | Frames: `{ id, parent_id, syscall, status, data }` over WebSocket |
| **Database** | PostgreSQL (Neon/Supabase free tier) |
| **Real-time state** | In-memory HashMap, LWW, server-authoritative |
| **Frontend** | React (Vite SPA) + Konva.js (react-konva) + Zustand |
| **Auth** | Anonymous sessions via Door subsystem (MVP), real auth (Full) |
| **AI** | Anthropic Claude Sonnet, tool calling via AI subsystem |
| **Hosting** | Fly.io (single container) |
| **CI/CD** | `fly deploy` from terminal |

**Total external dependencies:** Anthropic API + managed Postgres. Everything else runs in a single Rust binary on a single Fly.io VM.

---

## Phase 3: Post-Stack Refinement

### 12. Security Vulnerabilities

#### WebSocket Authentication

Token-based WS auth (identical to Prior's pattern): validate session on upgrade, verify board membership via Door subsystem before allowing frame relay. Session token as query parameter for MVP; short-lived upgrade tickets for production.

#### XSS on Canvas Text

Konva renders to `<canvas>`, inherently XSS-safe. Remaining vectors:
- **Text input fields** — overlay `<textarea>` is real DOM. Sanitize on save.
- **Board names/labels** — rendered in React. Default React escaping.
- Enforce max text length (10,000 chars) in the object subsystem.

#### CORS

Axum tower-http CORS layer with explicit origin allowlist. Not `*`.

#### AI Agent Security

**Rate limiting at three layers:**

| Layer | Limit |
|-------|-------|
| Per-user (Door enforced) | 10 AI requests/min |
| Total Anthropic calls | 20 calls/min |
| Token budget | 50k tokens/user/hour |

**Prompt injection defense:** Wrap user input in XML tags. Tool definitions are narrow — one per board operation, no generic tools.

#### Dependency Choices (Rust)

- `axum` + `tokio` — battle-tested async web stack
- `sqlx` — compile-time checked SQL queries against Postgres
- `serde` / `serde_json` — serialization
- `uuid` — ID generation
- `reqwest` — HTTP client for Anthropic API
- `tower-http` — CORS, compression, tracing middleware
- `jsonwebtoken` — JWT (or `jose` crate)

---

### 13. File Structure & Project Organization

Cargo workspace with two crates (kernel library + binary) plus a `client/` directory for the React frontend.

```
collaboard/
├── Cargo.toml                  # Workspace: kernel, server
├── .env.example
├── Dockerfile
├── fly.toml
│
├── kernel/                     # Core library crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs              # Module declarations
│       ├── frame.rs            # Frame type, Status, builders
│       ├── kernel/
│       │   ├── mod.rs          # Kernel struct, register(), start()
│       │   └── router.rs       # Frame dispatch by syscall prefix
│       ├── door/
│       │   ├── mod.rs          # Session lifecycle
│       │   ├── connect.rs      # door:connect
│       │   ├── disconnect.rs   # door:disconnect
│       │   └── join.rs         # door:join (enter board)
│       ├── board/
│       │   ├── mod.rs          # Board CRUD subsystem
│       │   ├── state.rs        # In-memory board state manager
│       │   └── persist.rs      # Postgres read/write
│       ├── object/
│       │   ├── mod.rs          # Object mutation subsystem
│       │   ├── create.rs
│       │   ├── update.rs
│       │   └── delete.rs
│       ├── cursor/
│       │   └── mod.rs          # Ephemeral cursor broadcast
│       ├── ai/
│       │   ├── mod.rs          # AI subsystem: prompt -> tool calls -> mutations
│       │   ├── tools.rs        # Tool definitions for Claude
│       │   └── client.rs       # Anthropic API client (reqwest)
│       └── db/
│           ├── mod.rs          # Pool init, migrations
│           └── migrations/     # SQL migration files
│
├── server/                     # Binary crate (thin wrapper)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs             # Entry: env, init kernel, start Axum
│       ├── gateway.rs          # Axum router: /api/auth/*, /api/ws
│       └── ws.rs               # WebSocket handler: frame relay
│
├── client/                     # React frontend (separate, not a Rust crate)
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   ├── index.html
│   └── src/
│       ├── main.tsx
│       ├── App.tsx
│       ├── pages/
│       │   ├── LoginPage.tsx
│       │   ├── DashboardPage.tsx
│       │   └── BoardPage.tsx
│       ├── canvas/
│       │   ├── Canvas.tsx
│       │   ├── StickyNote.tsx
│       │   ├── Shape.tsx
│       │   ├── Connector.tsx
│       │   ├── SelectionManager.tsx
│       │   └── Toolbar.tsx
│       ├── hooks/
│       │   ├── useFrameClient.ts   # WebSocket + Frame protocol
│       │   ├── useBoardState.ts    # Zustand store
│       │   ├── useAuth.ts
│       │   └── useAI.ts
│       ├── lib/
│       │   ├── frame.ts            # Frame type (mirrors Rust)
│       │   └── api.ts              # REST client for /api/auth
│       └── styles/
│           └── global.css
│
└── spec/                       # Integration tests
    └── (Rust tests inline + client tests via bun:test)
```

### Shared Types

The Frame type is defined in Rust (`kernel/src/frame.rs`) and mirrored in TypeScript (`client/src/lib/frame.ts`). Keep them manually in sync — there are only ~20 lines.

```typescript
// client/src/lib/frame.ts
export interface Frame {
  id: string;
  parent_id?: string;
  ts: number;
  syscall: string;
  status: "request" | "item" | "done" | "error" | "cancel";
  board_id?: string;
  from?: string;
  data: Record<string, unknown>;
}
```

---

### 14. Naming Conventions & Code Style

**Rust (backend):**

| Category | Convention | Example |
|----------|-----------|---------|
| Modules | snake_case | `board/persist.rs` |
| Structs/Enums | PascalCase | `Frame`, `Status`, `BoardObject` |
| Functions | snake_case | `handle_create()` |
| Constants | UPPER_SNAKE_CASE | `CHANNEL_BUFFER` |
| Error types | PascalCase with `Error` suffix | `BoardError`, `DoorError` |
| Tests | `#[cfg(test)]` inline modules | `mod tests { ... }` |

**TypeScript (frontend):**

| Category | Convention | Example |
|----------|-----------|---------|
| React components | PascalCase `.tsx` | `StickyNote.tsx` |
| Hooks | camelCase, `use` prefix | `useFrameClient.ts` |
| Types/interfaces | PascalCase, no `I` prefix | `BoardObject`, `Frame` |
| Directories | kebab-case | `canvas/` |

**Tooling:**
- Rust: `cargo fmt` + `cargo clippy` (standard, no config needed)
- TypeScript: Biome for lint + format

---

### 15. Testing Strategy

**Coverage target:** 40-50% for a 7-day sprint. Focus on logic that is hard to debug manually.

**Priority 1 — Must have:**

| What | Type | Why |
|------|------|-----|
| Frame routing (kernel dispatch) | Unit (Rust) | Wrong prefix silently drops frames |
| Object CRUD (subsystem) | Integration (Rust) | Validates Postgres schema + business logic |
| LWW conflict resolution | Unit (Rust) | Timestamp bugs corrupt state |
| Auth/Door session lifecycle | Unit (Rust) | Broken auth blocks all development |

**Priority 2 — Should have:**

| What | Type | Why |
|------|------|-----|
| AI tool dispatch | Unit (Rust, mocked) | Validates tool execution without API credits |
| Multi-client WS sync | Integration (Rust) | Core product promise |
| Frame client (TS) | Unit (bun:test) | Validates client-side frame dispatch |

**Priority 3 — Skip for MVP:**
- Canvas rendering tests
- E2E browser tests
- Performance/load tests

**Test commands:**
```bash
# Rust
cargo test                    # all tests
cargo test -p kernel          # kernel crate only
cargo test -- --test-threads=1  # serial for DB tests

# TypeScript
cd client && bun test
```

---

### 16. Recommended Tooling & DX

**Rust backend:**

| Tool | Purpose |
|------|---------|
| `cargo watch -x run` | Auto-restart on file changes |
| `cargo clippy` | Lint |
| `cargo fmt` | Format |
| `sqlx-cli` | Postgres migrations (`sqlx migrate run`) |
| `tracing` + `tracing-subscriber` | Structured logging with span context |
| `wscat` | CLI WebSocket testing |

**TypeScript frontend:**

| Tool | Purpose |
|------|---------|
| Vite | Build + HMR |
| Biome | Lint + format |
| Zustand | State management |
| Bun | Package manager + test runner |

**Dev workflow:**
```bash
# Terminal 1: Rust server with auto-restart
cargo watch -x run

# Terminal 2: Vite dev server with proxy
cd client && bun run dev
```

Vite dev server proxies `/api` to the Rust server:
```typescript
// client/vite.config.ts
server: {
  proxy: {
    "/api": "http://localhost:3000",
  },
}
```

**Debugging WebSockets:**
- Chrome DevTools > Network > WS tab for frame inspection
- `wscat -c "ws://localhost:3000/api/ws?token=dev-token"` for CLI testing
- Structured `tracing` spans in Rust: one span per frame, includes syscall + board_id + user_id

---

## Decision Summary

| Decision | Choice | Key Tradeoff |
|----------|--------|--------------|
| Scale | 5-20 users, single server | No horizontal scaling |
| Budget | $0 infra, ~$10 LLM | Single VM, brief deploy downtime |
| Timeline | 24hr MVP, 7 days total | No tests until day 6 |
| Compliance | None | Not production-grade |
| Backend | Rust monokernel + Axum (from Prior) | Slower iteration than TS, but proven patterns |
| Wire protocol | Frames over WebSocket | Must keep Rust/TS Frame types in sync |
| Database | PostgreSQL (Neon/Supabase free) | External dependency, but proper SQL |
| Sync | LWW, server-authoritative | No CRDT, same-object conflicts go to last writer |
| Frontend | React + Konva.js + Zustand | Konva ceiling ~1-2K objects |
| Auth | Anonymous MVP via Door subsystem | No persistent identity at MVP |
| AI | Claude Sonnet via AI subsystem | Single provider, no fallback |
| Hosting | Fly.io free tier | One process, one VM |
