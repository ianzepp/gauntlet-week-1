# React-to-Leptos UI Migration Plan

## Context

This document covers the **UI layer migration** from the existing `client/` (TypeScript + React 19 + Zustand + Konva.js) to a new `client-rust/` crate (Rust + Leptos + WASM). It is a companion to `konva-rust-revised.md`, which defines the canvas engine that replaces Konva.js.

The two efforts are parallel:

- **canvas/** crate: imperative engine handling shapes, hit-testing, rendering (see `konva-rust-revised.md`)
- **client-rust/** crate: Leptos application (SSR + hydration) hosting the canvas, plus all chrome UI (toolbar, panels, pages, routing, WS client). Integrated into the existing Axum server via Leptos-Axum.

This document covers `client-rust/` only.

## Decisions

| Decision | Choice |
|---|---|
| Rendering model | SSR + hydration (Leptos-Axum integration) |
| Deployment | Integrated into the server binary via Leptos-Axum; single process serves HTML + WASM |
| State management | Separate concerns: canvas engine owns doc model, Leptos signals own UI state, bridge between them |
| Migration strategy | Parallel: both clients coexist, same server, same WS protocol, remove React when Leptos reaches parity |
| Styling | Plain CSS files (global theme + per-component stylesheets) |
| Routing | `leptos_router` with URL-based routes |
| WS/Frame protocol | Types live in `client-rust/` directly (no separate wire/ crate); canvas/ has its own types |
| Doc detail level | Component-level mapping with signal dependencies and migration notes |

## Crate Structure

With SSR, the Leptos app integrates into the Axum server. The project becomes a Cargo workspace:

```
Cargo.toml                        # workspace root
├── server/
│   ├── Cargo.toml                # Axum server + Leptos-Axum integration
│   └── src/
│       ├── main.rs               # Axum app, mounts Leptos handler + API routes
│       └── ...                   # existing server code (services, routes, db, llm)
├── client-rust/
│   ├── Cargo.toml                # Leptos lib crate (compiled to both server + WASM)
│   ├── src/
│   │   ├── lib.rs                # crate root: exports App component
│   │   ├── app.rs                # <App/> root: auth check, router
│   │   ├── pages/
│   │   │   ├── mod.rs
│   │   │   ├── login.rs          # LoginPage
│   │   │   ├── dashboard.rs      # DashboardPage
│   │   │   └── board.rs          # BoardPage (layout host)
│   │   ├── components/
│   │   │   ├── mod.rs
│   │   │   ├── toolbar.rs        # Toolbar
│   │   │   ├── left_panel.rs     # LeftPanel (tool rail + inspector)
│   │   │   ├── right_panel.rs    # RightPanel (chat/AI/boards tabs)
│   │   │   ├── tool_rail.rs      # ToolRail
│   │   │   ├── tool_strip.rs     # ToolStrip (quick-create)
│   │   │   ├── inspector_panel.rs # InspectorPanel
│   │   │   ├── mission_control.rs # MissionControl (board switcher)
│   │   │   ├── chat_panel.rs     # ChatPanel
│   │   │   ├── ai_panel.rs       # AiPanel
│   │   │   ├── status_bar.rs     # StatusBar
│   │   │   ├── board_stamp.rs    # BoardStamp (canvas overlay)
│   │   │   ├── board_card.rs     # BoardCard
│   │   │   ├── user_field_report.rs # UserFieldReport (profile popover)
│   │   │   └── canvas_host.rs    # CanvasHost (bridge to canvas/ crate)
│   │   ├── state/
│   │   │   ├── mod.rs
│   │   │   ├── auth.rs           # AuthState (user, loading)
│   │   │   ├── ui.rs             # UiState (panels, tabs, dark mode)
│   │   │   ├── board.rs          # BoardState (board_id, board_name, connection)
│   │   │   ├── chat.rs           # ChatState (messages)
│   │   │   └── ai.rs             # AiState (messages, loading)
│   │   ├── net/
│   │   │   ├── mod.rs
│   │   │   ├── types.rs          # Frame, BoardObject, Presence, User, etc.
│   │   │   ├── api.rs            # REST helpers (fetch_me, logout, ws_ticket)
│   │   │   └── frame_client.rs   # WS lifecycle, reconnect, frame dispatch
│   │   └── util/
│   │       └── dark_mode.rs      # localStorage dark mode init
│   └── styles/
│       ├── global.css            # theme, resets, dark mode, typography
│       ├── pages/                # per-page stylesheets
│       └── components/           # per-component stylesheets
├── canvas/
│   ├── Cargo.toml                # canvas engine (own types, no wire dependency)
│   └── src/ ...
```

### Cargo Workspace (root Cargo.toml)

```toml
[workspace]
members = ["server", "client-rust", "canvas"]
resolver = "2"
```

### Cargo Dependencies (client-rust/)

```toml
[package]
name = "client-rust"
edition = "2024"

[dependencies]
leptos = "0.7"
leptos_meta = "0.7"
leptos_router = "0.7"
web-sys = { version = "0.3", features = [
    "HtmlCanvasElement", "WebSocket", "MessageEvent",
    "Window", "Document", "Storage", "ResizeObserver",
    "ResizeObserverEntry", "PointerEvent", "WheelEvent",
    "KeyboardEvent", "MouseEvent",
] }
js-sys = "0.3"
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
gloo-net = "0.6"             # fetch + WebSocket wrappers
gloo-timers = "0.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde", "js"] }
canvas = { path = "../canvas" }

[features]
hydrate = ["leptos/hydrate"]
ssr = ["leptos/ssr"]
```

### Cargo Dependencies (server/ additions)

```toml
# Added to existing server/Cargo.toml
leptos = { version = "0.7", features = ["ssr"] }
leptos_axum = "0.7"
client-rust = { path = "../client-rust", features = ["ssr"] }
```

## Types (net/types.rs)

Frame protocol and domain types live directly in `client-rust/src/net/types.rs`. No separate crate — these types are only used by the Leptos client. The `canvas/` crate defines its own `BoardObject` struct (per `konva-rust-revised.md`). The `CanvasHost` bridge converts between `net::types::BoardObject` and `canvas::BoardObject`.

### Dependency Graph

```
client-rust/  (owns frame/domain types in net/types.rs)
      ↓
   canvas/    (own types; receives data via method calls)
```

### Types

```rust
// client-rust/src/net/types.rs

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Frame {
    pub id: String,
    pub parent_id: Option<String>,
    pub ts: f64,
    pub board_id: Option<String>,
    pub from: Option<String>,
    pub syscall: String,
    pub status: FrameStatus,
    pub data: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FrameStatus {
    Request, Done, Error, Cancel,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BoardObject {
    pub id: String,
    pub board_id: String,
    pub kind: String,
    pub x: f64,
    pub y: f64,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub rotation: f64,
    pub z_index: i32,
    pub props: serde_json::Value,
    pub created_by: Option<String>,
    pub version: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Presence {
    pub user_id: String,
    pub name: String,
    pub color: String,
    pub cursor: Option<Point>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Point { pub x: f64, pub y: f64 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub color: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub color: String,
    pub member_since: Option<String>,
    pub stats: ProfileStats,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfileStats {
    pub total_frames: i64,
    pub objects_created: i64,
    pub boards_active: i64,
    pub last_active: Option<String>,
    pub top_syscalls: Vec<SyscallCount>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyscallCount {
    pub syscall: String,
    pub count: i64,
}
```

## State Architecture

### Principle: Separate Concerns

The current React app uses a single Zustand store for everything. The Leptos app separates state by domain, with each domain provided as a Leptos context.

```
┌──────────────────────────────────────────────────────┐
│                    Leptos App                         │
│                                                      │
│  ┌─────────┐  ┌─────────┐  ┌──────┐  ┌──────────┐  │
│  │AuthState│  │ UiState │  │Board │  │Chat / AI │  │
│  │(signals)│  │(signals)│  │State │  │ (signals)│  │
│  └─────────┘  └─────────┘  └──┬───┘  └──────────┘  │
│                                │                     │
│                     ┌──────────┴──────────┐          │
│                     │   CanvasHost        │          │
│                     │   (bridge component)│          │
│                     └──────────┬──────────┘          │
│                                │                     │
│                     ┌──────────┴──────────┐          │
│                     │   canvas::Engine    │          │
│                     │   (imperative,      │          │
│                     │    owns doc model)  │          │
│                     └─────────────────────┘          │
│                                                      │
│  ┌────────────────────────────────────────────────┐  │
│  │              FrameClient (WS)                  │  │
│  │  feeds both Leptos signals + canvas::Engine    │  │
│  └────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────┘
```

### AuthState

```rust
// Provided at app root via provide_context
pub struct AuthState {
    pub user: RwSignal<Option<User>>,
    pub loading: RwSignal<bool>,
}
```

**Reads:** LoginPage, Toolbar, ChatPanel, UserFieldReport
**Writes:** App (on fetch_me / logout)

### UiState

```rust
pub struct UiState {
    pub dark_mode: RwSignal<bool>,
    pub active_tool: RwSignal<ToolType>,
    pub left_panel_expanded: RwSignal<bool>,
    pub left_tab: RwSignal<LeftTab>,
    pub right_panel_expanded: RwSignal<bool>,
    pub right_tab: RwSignal<RightTab>,
}
```

**Reads:** LeftPanel, RightPanel, ToolRail, StatusBar, BoardPage layout
**Writes:** ToolRail (tool select), panel toggle buttons, keyboard shortcuts

### BoardState

```rust
pub struct BoardState {
    pub board_id: RwSignal<Option<String>>,
    pub board_name: RwSignal<Option<String>>,
    pub connection_status: RwSignal<ConnectionStatus>,
    pub presence: RwSignal<HashMap<String, Presence>>,
    pub frame_client: RwSignal<Option<Rc<FrameClient>>>,
}
```

**Reads:** Toolbar (name, presence), StatusBar (connection), CanvasHost (presence cursors)
**Writes:** FrameClient handlers, board navigation

The canvas engine's doc model (`HashMap<ObjectId, BoardObject>`) lives inside `canvas::Engine`, not in Leptos signals. The CanvasHost bridge feeds server broadcasts into the engine via `apply_create/apply_update/apply_delete`.

### ChatState / AiState

```rust
pub struct ChatState {
    pub messages: RwSignal<Vec<ChatMessage>>,
}

pub struct AiState {
    pub messages: RwSignal<Vec<AiMessage>>,
    pub loading: RwSignal<bool>,
}
```

**Reads/Writes:** ChatPanel, AiPanel, FrameClient handlers

## Routing

```rust
// app.rs
#[component]
fn App() -> impl IntoView {
    // Provide all state contexts
    provide_context(AuthState::new());
    provide_context(UiState::new());
    provide_context(BoardState::new());
    provide_context(ChatState::new());
    provide_context(AiState::new());

    // Start WS connection
    spawn_local(frame_client_lifecycle());

    view! {
        <Router>
            <Routes fallback=|| view! { "Not found" }>
                <Route path=path!("/") view=DashboardPage />
                <Route path=path!("/login") view=LoginPage />
                <Route path=path!("/board/:id") view=BoardPage />
            </Routes>
        </Router>
    }
}
```

### Route Mapping

| Current (React state) | Leptos route | Component |
|---|---|---|
| `!user` → LoginPage | `/login` | `LoginPage` |
| `page == "dashboard"` | `/` | `DashboardPage` |
| `page == "board"` | `/board/:id` | `BoardPage` |

Auth guard: `DashboardPage` and `BoardPage` check `AuthState.user`. If `None` and not loading, redirect to `/login`.

## Component Migration Map

### Pages

#### LoginPage (`pages/login.rs`)

| Aspect | React | Leptos |
|---|---|---|
| Source | `pages/LoginPage.tsx` | `pages/login.rs` |
| Purpose | GitHub OAuth redirect button | Same |
| State reads | None | None |
| State writes | None | None |
| Notes | Pure presentation. Clicking the button navigates to `/api/auth/github`. |

#### DashboardPage (`pages/dashboard.rs`)

| Aspect | React | Leptos |
|---|---|---|
| Source | `pages/DashboardPage.tsx` | `pages/dashboard.rs` |
| Purpose | List boards, create board, open board | Same |
| State reads | None (fetches board list internally) | `AuthState.user` for auth guard |
| State writes | Navigates via `onOpenBoard` callback | `leptos_router::use_navigate()` to `/board/:id` |
| Props (React) → Signals (Leptos) | `onOpenBoard: (id, name) => void` | Navigation via router |
| Notes | Fetches `/api/boards` via `gloo-net`. Board list is local to component (no global state needed). Uses `BoardCard` for each item. |

#### BoardPage (`pages/board.rs`)

| Aspect | React | Leptos |
|---|---|---|
| Source | `pages/BoardPage.tsx` | `pages/board.rs` |
| Purpose | Main workspace layout: toolbar + left panel + canvas + right panel + status bar | Same |
| State reads | `AuthState`, `UiState`, `BoardState` | Same |
| State writes | Sets `BoardState.board_id` from route param, triggers `board:join` | Same |
| Layout | CSS grid: toolbar (top), left panel (left), canvas (center), right panel (right), status bar (bottom) | Same layout via plain CSS |
| Notes | Reads `:id` from route via `use_params`. On mount, updates `BoardState.board_id` and sends `board:join` via `FrameClient`. On unmount, sends `board:part`. |

### Components

#### Toolbar (`components/toolbar.rs`)

| Aspect | React | Leptos |
|---|---|---|
| Source | `components/Toolbar.tsx` | `components/toolbar.rs` |
| Purpose | Top bar: board name, presence avatars, back button, logout | Same |
| State reads | `BoardState.board_name`, `BoardState.presence`, `AuthState.user` | Same |
| State writes | Navigation (back to dashboard), logout | `use_navigate()`, `AuthState` clear |
| Notes | Presence indicators show colored dots for connected users. Logout calls REST endpoint then clears AuthState. |

#### LeftPanel (`components/left_panel.rs`)

| Aspect | React | Leptos |
|---|---|---|
| Source | `components/LeftPanel.tsx` | `components/left_panel.rs` |
| Purpose | Collapsible container for ToolRail + InspectorPanel | Same |
| State reads | `UiState.left_panel_expanded`, `UiState.left_tab` | Same |
| State writes | Toggle expand/collapse | Same |
| Children | `<ToolRail/>`, `<InspectorPanel/>` | Same |

#### RightPanel (`components/right_panel.rs`)

| Aspect | React | Leptos |
|---|---|---|
| Source | `components/RightPanel.tsx` | `components/right_panel.rs` |
| Purpose | Collapsible container with tab switcher for AI, Chat, Boards | Same |
| State reads | `UiState.right_panel_expanded`, `UiState.right_tab` | Same |
| State writes | Toggle expand, switch tab | Same |
| Children | `<AiPanel/>`, `<ChatPanel/>`, `<MissionControl/>` | Same |

#### ToolRail (`components/tool_rail.rs`)

| Aspect | React | Leptos |
|---|---|---|
| Source | `components/ToolRail.tsx` | `components/tool_rail.rs` |
| Purpose | Vertical strip of tool buttons (select, shapes, line, etc.) | Same |
| State reads | `UiState.active_tool` | Same |
| State writes | `UiState.active_tool` on click; also calls `canvas::Engine::set_tool()` via CanvasHost bridge | Same |
| Notes | Tool list changes slightly: current tools include `sticky`, `draw`, `eraser` which are not in canvas v0. v0 tools: `select`, `rect`, `ellipse`, `diamond`, `star`, `line`, `arrow`. |

#### ToolStrip (`components/tool_strip.rs`)

| Aspect | React | Leptos |
|---|---|---|
| Source | `components/ToolStrip.tsx` | `components/tool_strip.rs` |
| Purpose | Quick-create buttons with shape+color presets | Same |
| State reads | `UiState.active_tool` | Same |
| State writes | Sets tool + default props for quick creation | Same |

#### InspectorPanel (`components/inspector_panel.rs`)

| Aspect | React | Leptos |
|---|---|---|
| Source | `components/InspectorPanel.tsx` | `components/inspector_panel.rs` |
| Purpose | Edit properties of selected object (position, size, color, text) | Same |
| State reads | Selection from `canvas::Engine::selection()` + `canvas::Engine::object(id)` | Same |
| State writes | Sends property changes to engine + server | Same |
| Notes | Reads from engine via CanvasHost bridge. On property change, calls `engine.apply_update()` locally and sends `object:update` frame. |

#### MissionControl (`components/mission_control.rs`)

| Aspect | React | Leptos |
|---|---|---|
| Source | `components/MissionControl.tsx` | `components/mission_control.rs` |
| Purpose | In-board board switcher (list + navigate) | Same |
| State reads | Fetches board list internally | Same |
| State writes | Navigate to `/board/:id` | `use_navigate()` |

#### ChatPanel (`components/chat_panel.rs`)

| Aspect | React | Leptos |
|---|---|---|
| Source | `components/ChatPanel.tsx` | `components/chat_panel.rs` |
| Purpose | Real-time board chat | Same |
| State reads | `ChatState.messages`, `AuthState.user` | Same |
| State writes | `ChatState.messages` (append), sends `chat:send` frame | Same |
| Notes | Markdown rendering for messages. Use `pulldown-cmark` compiled to WASM for markdown-to-HTML, then set `inner_html`. |

#### AiPanel (`components/ai_panel.rs`)

| Aspect | React | Leptos |
|---|---|---|
| Source | `components/AiPanel.tsx` | `components/ai_panel.rs` |
| Purpose | AI prompt input, response display | Same |
| State reads | `AiState.messages`, `AiState.loading` | Same |
| State writes | `AiState` (append message, set loading), sends `ai:prompt` frame | Same |
| Notes | Replaces `react-markdown` with `pulldown-cmark`. Streaming responses arrive as frames and update the last assistant message. |

#### StatusBar (`components/status_bar.rs`)

| Aspect | React | Leptos |
|---|---|---|
| Source | `components/StatusBar.tsx` | `components/status_bar.rs` |
| Purpose | Bottom bar: connection status, zoom level, object count | Same |
| State reads | `BoardState.connection_status`, canvas engine camera (zoom), canvas engine doc (object count) | Same |
| State writes | None | None |
| Notes | Object count and zoom read from engine via CanvasHost bridge signals. |

#### BoardStamp (`components/board_stamp.rs`)

| Aspect | React | Leptos |
|---|---|---|
| Source | `components/BoardStamp.tsx` | `components/board_stamp.rs` |
| Purpose | Semi-transparent overlay label on the canvas area | Same |
| State reads | `BoardState.board_name` | Same |
| Notes | Pure presentation, positioned absolutely over the canvas. |

#### BoardCard (`components/board_card.rs`)

| Aspect | React | Leptos |
|---|---|---|
| Source | `components/BoardCard.tsx` | `components/board_card.rs` |
| Purpose | Reusable card for board list items | Same |
| Props | Board metadata (id, name, created, etc.) | Same as component props |
| Notes | Pure presentation component. |

#### UserFieldReport (`components/user_field_report.rs`)

| Aspect | React | Leptos |
|---|---|---|
| Source | `components/UserFieldReport.tsx` | `components/user_field_report.rs` |
| Purpose | User profile popover with stats | Same |
| State reads | Fetches `/api/users/:id/profile` on open | Same |
| Notes | Popover positioning via `web-sys` DOM measurements. |

#### CanvasHost (`components/canvas_host.rs`) — NEW

This component does not exist in the React app (where `Canvas.tsx` directly uses React-Konva). In Leptos, `CanvasHost` is the bridge between the Leptos UI and the imperative `canvas::Engine`.

| Aspect | Detail |
|---|---|
| Purpose | Mount `<canvas>` element, create `canvas::Engine`, wire pointer/keyboard events, bridge to/from Leptos signals |
| Creates | `canvas::Engine::new(canvas_element)` |
| Inputs from Leptos | Tool changes (`UiState.active_tool`), viewport resize, text edit commits |
| Inputs from server | `board:join` snapshot → `engine.load_snapshot()`, broadcast create/update/delete → `engine.apply_*()` |
| Outputs to Leptos | Selection changes (for InspectorPanel), camera state (for StatusBar zoom), cursor style, text edit requests |
| Outputs to server | `Action::ObjectCreated` → send `object:create` frame, `Action::ObjectUpdated` → send `object:update` frame, etc. |
| Event wiring | `onpointerdown/move/up`, `onwheel`, `onkeydown/up` on canvas → `engine.on_*()` |
| Render loop | Calls `engine.render()` via `request_animation_frame` when `Action::RenderNeeded` is returned |

Bridge signals (provided via context or stored locally):

```rust
pub struct CanvasBridge {
    /// Currently selected object ID (read by InspectorPanel)
    pub selection: RwSignal<Option<String>>,
    /// Camera zoom level (read by StatusBar)
    pub zoom: RwSignal<f64>,
    /// Object count (read by StatusBar)
    pub object_count: RwSignal<usize>,
    /// Text edit request from engine (read by CanvasHost to spawn editor overlay)
    pub edit_text_request: RwSignal<Option<EditTextRequest>>,
}
```

## Network Layer (net/)

### REST API (`net/api.rs`)

Direct port of `client/src/lib/api.ts` using `gloo-net`:

```rust
pub async fn fetch_current_user() -> Option<User> { ... }
pub async fn logout() { ... }
pub async fn fetch_user_profile(user_id: &str) -> Option<UserProfile> { ... }
pub async fn create_ws_ticket() -> Result<String, JsValue> { ... }
```

### FrameClient (`net/frame_client.rs`)

Port of `client/src/lib/frameClient.ts` + `client/src/hooks/useFrameClient.ts`.

Key differences from the React version:

- No class; instead a `spawn_local` async task that owns the WebSocket
- Reconnect logic with exponential backoff (same as current)
- Frame dispatch updates Leptos signals directly (no Zustand `getState()`)
- Object create/update/delete frames also call `canvas::Engine` methods via the CanvasBridge

```rust
pub async fn frame_client_lifecycle() {
    // 1. Create WS ticket via REST
    // 2. Connect WebSocket
    // 3. On message: parse Frame, dispatch to handlers
    // 4. On close: schedule reconnect
    // Handlers update BoardState, ChatState, AiState, and canvas::Engine
}
```

Dispatch table (same syscalls as current):

| Syscall | Handler |
|---|---|
| `session:connected` | Set `connection_status = Connected`, send `board:join` if board_id set |
| `session:disconnected` | Set `connection_status = Disconnected`, schedule reconnect |
| `board:join` (done) | `engine.load_snapshot()`, set presence |
| `board:part` | Remove presence |
| `object:create` (done) | `engine.apply_create()`, reconcile temp IDs |
| `object:update` (done) | `engine.apply_update()` |
| `object:delete` (done) | `engine.apply_delete()` |
| `cursor:moved` | Update `BoardState.presence` |
| `chat:message` | Append to `ChatState.messages` |

## Styling Approach (Plain CSS)

Plain CSS files, bundled by `cargo-leptos`. No CSS-in-Rust framework — straightforward and zero risk of framework issues.

### File Layout

```
client-rust/
├── styles/
│   ├── global.css            # theme variables, resets, dark mode, typography
│   ├── pages/
│   │   ├── login.css
│   │   ├── dashboard.css
│   │   └── board.css
│   └── components/
│       ├── toolbar.css
│       ├── left_panel.css
│       ├── right_panel.css
│       ├── tool_rail.css
│       ├── tool_strip.css
│       ├── inspector_panel.css
│       ├── chat_panel.css
│       ├── ai_panel.css
│       ├── status_bar.css
│       ├── board_stamp.css
│       ├── board_card.css
│       ├── user_field_report.css
│       └── canvas_host.css
├── index.html                # <link rel="stylesheet" href="styles/global.css" />
```

`cargo-leptos` bundles CSS via the `style-file` and `assets-dir` config. `global.css` `@import`s component stylesheets. Leptos SSR injects the `<link>` tag into the rendered HTML.

### Convention

- Each component uses a BEM-like prefix to avoid collisions (e.g., `.toolbar__title`, `.left-panel--collapsed`).
- No scoping magic — just discipline and short component names.
- All components reference CSS custom properties for theme values.

### Theme Variables

Ported from the existing `client/src/styles/global.css`. These define the retro-scientific palette:

```css
:root {
    --surface: #FAF6F0;
    --border: #D6CFC4;
    --text: #1F1A17;
    --accent: #D94B4B;
    /* ... etc ... */
}
.dark-mode {
    --surface: #1A1714;
    --border: #3D362F;
    --text: #E8E0D4;
    --accent: #E85D5D;
}
```

Many existing CSS rules from `client/src/components/*.module.css` can be ported directly — the class names change from camelCase CSS Modules to BEM kebab-case, but the property values stay the same.

### Dark Mode

Port of `initDarkMode()` — reads `localStorage`, applies `.dark-mode` class to `<html>`. Toggle writes to localStorage and toggles the class.

## Kind Name Migration

The current React client uses legacy kind names. The Leptos client and canvas engine use the new names from `konva-rust-revised.md`:

| Legacy (React) | New (Leptos/Canvas) | Notes |
|---|---|---|
| `sticky_note` | `rect` | Sticky notes become rects with default fill |
| `rectangle` | `rect` | Direct rename |
| `ellipse` | `ellipse` | Unchanged |
| `line` | `line` | Unchanged |
| `connector` | `arrow` | Connectors become arrows with endpoint props |
| `text` | (text fields on shapes) | Standalone text objects become rects with text props |

The database will be reset for this migration, so no data migration is needed. The server's kind references (e.g., AI tool definitions) will be updated to use the new names.

## Build Toolchain

### cargo-leptos

With SSR, `cargo-leptos` replaces Trunk. It builds both the server binary (with SSR support) and the client WASM in one step.

```toml
# Cargo.toml (workspace root, cargo-leptos config)
[package.metadata.leptos]
output-name = "collabboard"
site-root = "target/site"
site-pkg-dir = "pkg"
style-file = "client-rust/styles/global.css"
assets-dir = "client-rust/styles"
site-addr = "127.0.0.1:3000"
reload-port = 3001
bin-package = "server"
lib-package = "client-rust"
lib-profile-release = "wasm-release"
```

### Server Integration

The server's `main.rs` mounts the Leptos handler alongside existing API routes:

```rust
// server/src/main.rs (simplified)
use leptos_axum::LeptosRoutes;
use client_rust::app::App;

let leptos_options = get_configuration(None).unwrap().leptos_options;
let app = Router::new()
    .nest("/api", api_routes())           // existing REST + WS routes
    .leptos_routes(&leptos_options, routes::generate_route_list(App), App)
    .with_state(app_state);
```

SSR renders the initial HTML server-side. The browser then downloads the WASM bundle and hydrates. After hydration, client-side navigation takes over (no full page reloads).

### SSR Boundaries

Not all components can run on the server. Canvas, WebSocket, and DOM-dependent code must be gated behind `#[cfg(feature = "hydrate")]` or wrapped in `<Suspense>`:

- **Server-renderable:** LoginPage, DashboardPage layout, BoardPage layout, Toolbar, panels, StatusBar (static content)
- **Client-only (after hydration):** CanvasHost, FrameClient, dark mode init, ResizeObserver, pointer events

Use `create_effect` (which only runs client-side in SSR mode) for all browser API access.

### Development Workflow

- `cargo leptos watch` — builds server + WASM, watches for changes, live-reloads
- `cargo leptos build --release` — production build
- Single binary serves everything (HTML, WASM, CSS, API, WS)

## Implementation Order

This order prioritizes getting a working shell quickly, then filling in features.

### Phase 1: Skeleton + Routing + Auth

1. ~~Set up Cargo workspace (root `Cargo.toml` with `server`, `client-rust`, `canvas`)~~ **DONE**
2. Set up `client-rust/` as a Leptos lib crate with SSR + hydrate features — **Scaffold done** (rlib only, no Leptos dep yet)
3. Integrate Leptos-Axum into `server/` (mount Leptos handler alongside API routes)
4. Verify `cargo leptos watch` builds and serves a hello-world page with SSR + hydration
5. ~~Implement `net/types.rs` (Frame, BoardObject, User, etc.)~~ **DONE** — all types with serde, PartialEq, 16 round-trip tests
6. ~~Implement `net/api.rs` (REST helpers, client-only)~~ **DONE** — stubs (no gloo-net yet)
7. ~~Implement `AuthState` + auth check~~ **DONE** — struct + Default + tests
8. Implement `LoginPage` (OAuth redirect button) — **stub only** (no Leptos view logic yet)
9. Implement `DashboardPage` (board list + create) — **stub only** (no Leptos view logic yet)
10. Verify: can log in, see boards, create a board (SSR renders initial HTML, hydration takes over)

**Phase 1 scaffold status:** Crate compiles as standalone `rlib` with zero clippy warnings and 36 passing tests. All module files exist. Leptos dependency and SSR/hydrate wiring deferred to next step (requires WASM target setup).

### Phase 2: Board Layout + WS Connection

11. Implement `BoardPage` layout (toolbar + panels + canvas placeholder)
12. Implement `Toolbar` (board name, back button, logout)
13. Implement `StatusBar` (connection status)
14. Implement `net/frame_client.rs` (WS lifecycle, reconnect — client-only via `create_effect`)
15. Implement `BoardState` context, wire up `board:join` / `board:part`
16. Verify: can enter a board, see connection status, WS connects after hydration

### Phase 3: Canvas Integration

17. Implement `CanvasHost` bridge component (client-only — guarded behind hydration)
18. Wire `canvas::Engine` creation and event forwarding
19. Wire server snapshot → `engine.load_snapshot()`
20. Wire server broadcasts → `engine.apply_create/update/delete()`
21. Wire engine `Action` outputs → send frames to server
22. Verify: objects render on canvas, can create/move/delete shapes

### Phase 4: Tool UI + Inspector

23. Implement `ToolRail` with v0 tool set
24. Implement `ToolStrip` quick-create
25. Wire tool selection → `engine.set_tool()`
26. Implement `CanvasBridge` selection signal
27. Implement `InspectorPanel` reading from engine
28. Implement `LeftPanel` container with expand/collapse
29. Verify: can switch tools, select objects, edit properties

### Phase 5: Chat + AI + Right Panel

30. Implement `ChatPanel` + `ChatState`
31. Implement `AiPanel` + `AiState`
32. Implement `MissionControl` (board switcher)
33. Implement `RightPanel` container with tab switching
34. Verify: chat works, AI prompts work, can switch boards

### Phase 6: Polish + Parity

35. Implement `BoardStamp` overlay
36. Implement `BoardCard` component
37. Implement `UserFieldReport` popover
38. Dark mode toggle
39. Keyboard shortcuts (tool switching, delete, escape)
40. Text editing overlay (Leptos editor for `head/text/foot`)
41. Presence cursors display
42. Final parity check against React client

### Phase 7: Cutover

43. Run both clients in parallel, verify feature parity
44. Remove `client/` directory
45. Update Dockerfile to use `cargo leptos build --release`
46. Update `run-dev.sh` for `cargo leptos watch`

## v0 Definition of Done

- [ ] Login via GitHub OAuth works
- [ ] Dashboard lists boards, can create new board
- [ ] Board page renders with full layout (toolbar, panels, canvas, status bar)
- [ ] WebSocket connects with reconnect
- [ ] Canvas renders objects from server snapshot
- [ ] Can create, select, move, resize, rotate shapes via canvas tools
- [ ] Inspector panel shows and edits selected object properties
- [ ] Chat panel sends and receives messages
- [ ] AI panel sends prompts and displays responses
- [ ] Board switcher navigates between boards
- [ ] Dark mode toggles correctly
- [ ] URL-based routing works (back button, direct links)
- [ ] React `client/` removed, Leptos `client-rust/` is the sole frontend
