# Revised Plan: `client-rust/` — Full Leptos UI, Then Canvas

## Context

The original plan interleaved UI and canvas work. The user wants a different order: **build the complete Leptos UI first** (all pages, components, state, networking, styling), then integrate the `canvas/` crate afterwards. This means the BoardPage will have a placeholder where the canvas goes, and all surrounding chrome (toolbar, panels, status bar, chat, AI, inspector) gets built to completion first.

The Phase 1 scaffold is already done on the `leptos-ui` branch: types, state structs, module stubs, 36 tests passing. This plan picks up from there.

## Key Constraint

**No canvas/ dependency yet.** The `CanvasHost` component stays as a placeholder `<div>` until the final phase. Components that read canvas state (InspectorPanel, StatusBar zoom/object count) use dummy/placeholder values. Everything else should be fully functional.

## Working Directory

All work is on the `leptos-ui` branch at `.worktrees/leptos-ui/`.

---

## Phase 2: Add Leptos + Axum Integration

**Goal:** `cargo leptos watch` serves a hello-world page with SSR + hydration.

### Steps

1. **Add Leptos dependencies to `client-rust/Cargo.toml`**
   - `leptos = "0.7"`, `leptos_meta = "0.7"`, `leptos_router = "0.7"`
   - `wasm-bindgen`, `js-sys`, `web-sys` (minimal features for now)
   - `gloo-net = "0.6"`, `gloo-timers = "0.3"`, `wasm-bindgen-futures = "0.4"`
   - Feature flags: `hydrate = ["leptos/hydrate"]`, `ssr = ["leptos/ssr"]`
   - Crate type: `["cdylib", "rlib"]`

2. **Add Leptos-Axum to `server/Cargo.toml`**
   - `leptos = { version = "0.7", features = ["ssr"] }`
   - `leptos_axum = "0.7"`
   - `client-rust = { path = "../client-rust", features = ["ssr"] }`

3. **Create `client-rust/src/app.rs`**
   - `App` component with `<Router>` and 3 routes: `/login`, `/`, `/board/:id`
   - Provide all state contexts (AuthState, UiState, BoardState, ChatState, AiState)
   - Convert state structs from plain fields to `RwSignal` fields

4. **Wire Leptos into `server/src/main.rs`**
   - Mount Leptos handler alongside existing API routes
   - Add `cargo-leptos` config to root `Cargo.toml` (`[package.metadata.leptos]`)

5. **Install `cargo-leptos`** and verify `cargo leptos watch` builds and serves

### Files Modified
- `client-rust/Cargo.toml`
- `client-rust/src/lib.rs` (add `pub mod app`)
- `client-rust/src/app.rs` (new)
- `client-rust/src/state/*.rs` (convert to RwSignal)
- `server/Cargo.toml`
- `server/src/main.rs`
- Root `Cargo.toml` (leptos metadata)

### Verification
- `cargo leptos watch` starts without errors
- Browser at `localhost:3000` shows a rendered page
- View-source shows SSR HTML (not empty body)

---

## Phase 3: Pages + Auth

**Goal:** Login, dashboard, and board page shells render. Auth flow works.

### Steps

1. **Implement `net/api.rs`** — real HTTP calls using `gloo-net`
   - `fetch_current_user()` — `GET /api/auth/me`
   - `logout()` — `POST /api/auth/logout`
   - `fetch_user_profile()` — `GET /api/users/{id}/profile`
   - `create_ws_ticket()` — `POST /api/auth/ws-ticket`
   - All gated behind `#[cfg(feature = "hydrate")]` (client-only)

2. **Implement `LoginPage`** (`pages/login.rs`)
   - GitHub OAuth button → navigates to `/api/auth/github`
   - Styled to match React `LoginPage.tsx`

3. **Implement `DashboardPage`** (`pages/dashboard.rs`)
   - Auth guard: redirect to `/login` if no user
   - Fetch board list via frame client `board:list` syscall
   - Create board dialog with `board:create` syscall
   - Render `BoardCard` for each board
   - Click board → navigate to `/board/:id`

4. **Implement `BoardPage` shell** (`pages/board.rs`)
   - Auth guard
   - Read `:id` from route params
   - CSS grid layout: toolbar (top), left panel (left), canvas placeholder (center), right panel (right), status bar (bottom)
   - Send `board:join` on mount, `board:part` on unmount

5. **Implement `BoardCard`** (`components/board_card.rs`)
   - Card UI for board list items

### Files Modified
- `client-rust/src/net/api.rs`
- `client-rust/src/pages/login.rs`
- `client-rust/src/pages/dashboard.rs`
- `client-rust/src/pages/board.rs`
- `client-rust/src/components/board_card.rs`

### Verification
- Can visit `/login`, click GitHub OAuth, get redirected back
- Dashboard shows board list, can create a board
- Clicking a board navigates to `/board/:id` with the grid layout

---

## Phase 4: WebSocket Frame Client

**Goal:** WebSocket connects, reconnects, dispatches frames to state.

### Steps

1. **Implement `net/frame_client.rs`**
   - `FrameClient` struct with send/receive
   - `frame_client_lifecycle()` async task
   - Connection: get ticket via REST, connect to `wss://.../api/ws?ticket=...`
   - Reconnect with exponential backoff (1s → 10s)
   - Frame dispatch table:
     - `session:connected` → set `ConnectionStatus::Connected`
     - `session:disconnected` → set `ConnectionStatus::Disconnected`
     - `board:join` (done) → load objects into `BoardState`
     - `board:part` → remove presence
     - `object:create/update/delete` → update objects map in `BoardState`
     - `cursor:moved` → update presence
     - `chat:message` → append to `ChatState`
   - Client-only: guarded behind `#[cfg(feature = "hydrate")]`

2. **Add objects map to `BoardState`**
   - `objects: RwSignal<HashMap<String, BoardObject>>`
   - `selection: RwSignal<HashSet<String>>`

3. **Wire frame client into `app.rs`**
   - Spawn via `create_effect` (client-only)

### Files Modified
- `client-rust/src/net/frame_client.rs`
- `client-rust/src/state/board.rs`
- `client-rust/src/app.rs`

### Verification
- Board page shows connection status (green dot when connected)
- Objects received from server populate `BoardState.objects`
- Reconnect works after server restart

---

## Phase 5: Toolbar + Status Bar

**Goal:** Top toolbar and bottom status bar fully functional.

### Steps

1. **Implement `Toolbar`** (`components/toolbar.rs`)
   - Board name display (from `BoardState.board_name`)
   - Presence avatars (colored dots for connected users)
   - Back button → navigate to dashboard
   - Logout button → call `api::logout()`, clear `AuthState`
   - Click user chip → show `UserFieldReport`

2. **Implement `StatusBar`** (`components/status_bar.rs`)
   - Connection status indicator (colored dot)
   - Board name
   - Object count (from `BoardState.objects.len()`)
   - Placeholder for zoom % and cursor position (canvas-dependent)

3. **Implement `UserFieldReport`** (`components/user_field_report.rs`)
   - Popover that fetches `/api/users/:id/profile`
   - Shows stats: frames, objects created, boards active, top syscalls

### Files Modified
- `client-rust/src/components/toolbar.rs`
- `client-rust/src/components/status_bar.rs`
- `client-rust/src/components/user_field_report.rs`

### Verification
- Toolbar shows board name, presence dots, back/logout buttons
- Logout clears session and redirects to login
- Status bar shows connection state and object count
- Clicking a user chip shows profile popover

---

## Phase 6: Left Panel (Tools + Inspector)

**Goal:** Left panel with tool selection and object property inspector.

### Steps

1. **Implement `LeftPanel`** (`components/left_panel.rs`)
   - Collapsible container
   - Tabs: Tools / Inspector (from `UiState.left_tab`)
   - Expand/collapse toggle

2. **Implement `ToolRail`** (`components/tool_rail.rs`)
   - Vertical strip of tool buttons: Select, Rect, Ellipse, Diamond, Star, Line, Arrow
   - Highlights active tool (reads `UiState.active_tool`)
   - Click sets `UiState.active_tool`

3. **Implement `ToolStrip`** (`components/tool_strip.rs`)
   - Shape presets (size + color)
   - "Add" button creates object via `object:create` frame
   - Uses frame client to send creation request

4. **Implement `InspectorPanel`** (`components/inspector_panel.rs`)
   - Shows properties of selected object (from `BoardState.selection` + `BoardState.objects`)
   - Editable fields: width, height, title, text, font size, background, border
   - Read-only fields: position, rotation, z-index, version, ID
   - Delete button with confirmation
   - Sends `object:update` / `object:delete` frames on change
   - No canvas bridge needed — reads from `BoardState.objects` directly

### Files Modified
- `client-rust/src/components/left_panel.rs`
- `client-rust/src/components/tool_rail.rs`
- `client-rust/src/components/tool_strip.rs`
- `client-rust/src/components/inspector_panel.rs`

### Verification
- Left panel expands/collapses
- Tool rail highlights active tool, click switches
- ToolStrip creates objects (visible in inspector even without canvas)
- Inspector shows selected object properties, edits send frames

---

## Phase 7: Right Panel (Chat + AI + Boards)

**Goal:** Right panel with all three tabs fully functional.

### Steps

1. **Implement `RightPanel`** (`components/right_panel.rs`)
   - Collapsible container with tab rail: Chat, AI, Boards
   - Tab switching from `UiState.right_tab`

2. **Implement `ChatPanel`** (`components/chat_panel.rs`)
   - Message list from `ChatState.messages`
   - Text input + send button
   - Fetch `chat:history` on board load
   - Send `chat:message` frame
   - Messages show user name + color

3. **Implement `AiPanel`** (`components/ai_panel.rs`)
   - Message list from `AiState.messages`
   - Text input + send button
   - Fetch `ai:history` on board load
   - Send `ai:prompt` frame (without grid context for now — canvas-dependent)
   - Show loading state ("Thinking...")
   - Render markdown responses (use `pulldown-cmark` or plain text initially)

4. **Implement `MissionControl`** (`components/mission_control.rs`)
   - Board list (same data as DashboardPage but compact)
   - Click navigates to `/board/:id`

### Files Modified
- `client-rust/src/components/right_panel.rs`
- `client-rust/src/components/chat_panel.rs`
- `client-rust/src/components/ai_panel.rs`
- `client-rust/src/components/mission_control.rs`

### Verification
- Right panel tab switching works
- Chat sends and receives messages in real-time
- AI panel sends prompts and displays responses
- Board switcher navigates between boards

---

## Phase 8: Polish + Styling

**Goal:** Dark mode, keyboard shortcuts, styling parity with React client.

### Steps

1. **Implement `util/dark_mode.rs`**
   - Read localStorage (`gauntlet_week_1_dark`)
   - Auto-detect system preference on first load
   - Apply `.dark-mode` class to `<html>`
   - Toggle function writes localStorage + updates class

2. **Implement `BoardStamp`** (`components/board_stamp.rs`)
   - Semi-transparent overlay showing board name + stats

3. **Create CSS files** (`client-rust/styles/`)
   - Port theme variables from `client/src/styles/global.css`
   - Component-specific styles (BEM naming)
   - Dark mode overrides

4. **Keyboard shortcuts**
   - Delete key → delete selected object (with confirmation)
   - Escape → deselect all
   - Tool shortcuts (later, when canvas is wired)

### Files Modified
- `client-rust/src/util/dark_mode.rs`
- `client-rust/src/components/board_stamp.rs`
- `client-rust/styles/` (new CSS files)

### Verification
- Dark mode toggles via button
- Persists across page refresh
- All components styled to match React client
- BoardStamp overlay renders on board page

---

## Phase 9: Canvas Integration (Future)

**Goal:** Replace the placeholder `<div>` with the `canvas/` crate engine.

This phase is **deferred** and will be planned separately. High-level:

1. Add `canvas = { path = "../canvas" }` dependency
2. Implement `CanvasHost` — mount `<canvas>`, create `canvas::Engine`
3. Wire pointer/keyboard events to engine
4. Wire `board:join` snapshot → `engine.load_snapshot()`
5. Wire server broadcasts → `engine.apply_create/update/delete()`
6. Wire engine actions → send frames to server
7. Wire `InspectorPanel` to read from engine (replace BoardState reads)
8. Wire `StatusBar` zoom/cursor from engine
9. Wire `AiPanel` grid context from viewport
10. Remote cursor rendering

---

## Implementation Notes

- **All work on `leptos-ui` branch** in `.worktrees/leptos-ui/`
- **Each phase is a separate commit** (or small set of commits)
- **Follow project conventions:** `*_test.rs` files, no panics in lib code, `cargo fmt` + `cargo clippy` + `cargo test` before each commit
- **State structs convert from plain fields to `RwSignal`** in Phase 2
- **CSS-in-Rust:** No framework — plain CSS files with BEM naming, CSS custom properties for theme
- **Client-only code** gated behind `#[cfg(feature = "hydrate")]` or `create_effect`
