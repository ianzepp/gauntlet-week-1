# Revised Plan: `client-rust/` — Full Leptos UI, Then Canvas

## Context

The original plan interleaved UI and canvas work. The user wants a different order: **build the complete Leptos UI first** (all pages, components, state, networking, styling), then integrate the `canvas/` crate afterwards. This means the BoardPage will have a placeholder where the canvas goes, and all surrounding chrome (toolbar, panels, status bar, chat, AI, inspector) gets built to completion first.

## Key Constraint

**No canvas/ dependency yet.** The `CanvasHost` component stays as a placeholder `<div>` until the final phase. Components that read canvas state (InspectorPanel, StatusBar zoom/object count) use dummy/placeholder values. Everything else should be fully functional.

## Reference Implementation

The React client at `client/` is the source of truth for all UI behavior, layout, and styling. Every component, interaction, and visual detail in the Leptos client must match the React client exactly unless noted otherwise.

---

## Phase 1: Scaffold — DONE

Types, state structs, module stubs, tests passing.

## Phase 2: Leptos + Axum Integration — DONE

SSR + hydration working. `cargo leptos build` succeeds.

## Phase 3: Pages + Auth — PARTIAL

**Done:** LoginPage, DashboardPage shell, BoardPage shell, BoardCard, net/api.rs REST calls, auth guard redirects.

**Remaining:**
- LoginPage: Change title from "Gauntlet" to "CollabBoard", remove subtitle
- DashboardPage: Wire `board:create` syscall (currently TODO stub), navigate to new board after create, Enter key to submit dialog, backdrop click to dismiss, add dashed "+" new board card in grid, add nav header bar matching React
- BoardPage: Send `board:join` on connect and reconnect (currently never sent), send `board:part` on unmount, cleanup state on unmount

## Phase 4: WebSocket Frame Client — PARTIAL

**Done:** Connection lifecycle, reconnect with backoff, dispatch for: `session:connected`, `board:join` (done), `object:create/update/delete` (done), `cursor:moved`, `board:part`, `chat:message` (done).

**Remaining — add to `dispatch_frame`:**
- `ai:prompt` (done/error) → append to AiState messages
- `ai:history` (done) → populate AiState messages
- `chat:history` (done) → populate ChatState messages
- `board:create` (done) → navigate to new board
- Peer `board:join` broadcast (when `client_id` present but no objects) → add presence entry
- Error status frames → log warning
- `gateway:error` → log warning

**Remaining — add outbound sends from components:**
- `board:join` from BoardPage on connect/reconnect
- `board:create` from DashboardPage dialog
- `chat:history` from ChatPanel on mount
- `ai:history` from AiPanel on mount

---

## Phase 5: Toolbar + Status Bar — NEEDS REWORK

Current state: Basic toolbar with presence chips and logout. Basic status bar with connection dot. UserFieldReport fetches profile but has no popover positioning or close behavior.

### Toolbar (`components/toolbar.rs`)
- [ ] Show local user chip alongside remote presence chips (React combines both)
- [ ] Add click handler on presence chips to open UserFieldReport popover
- [ ] Position popover relative to clicked chip (measure DOM position)
- [ ] Add full-screen backdrop that closes popover on click
- [ ] Back button: only show when on board page (React: conditional on `onBack` prop)
- [ ] Match React styling: 36px height, `--bg-nav`, flat left-border presence chips (not pills), 6px square dots (not 8px circles), IBM Plex Mono 10px uppercase for chip text

### StatusBar (`components/status_bar.rs`)
- [ ] Add cursor position display (placeholder `(0, 0)` until canvas integration)
- [ ] Add viewport center display (placeholder `(0, 0)`)
- [ ] Zoom display reads from state (placeholder "100%")
- [ ] Add user chip with color dot (authenticated user's name + color)
- [ ] Match React styling: 24px height, `--bg-status-bar`, mono 11px uppercase, 6px square dots, section dividers

### UserFieldReport (`components/user_field_report.rs`)
- [ ] Add avatar image display (`<img>` when `avatar_url` is Some)
- [ ] Add `member_since` display in "Field Agent" badge line
- [ ] Add `last_active` row in stats
- [ ] Add backdrop element for click-to-close
- [ ] Position as fixed popover (not inline), clamped to viewport
- [ ] Match React styling: flat rectangle (no border-radius), warm bg, Caveat name font, mono stats

---

## Phase 6: Left Panel (Tools + Inspector) — NEEDS REWORK

Current state: Flat tabbed panel (Tools/Inspector tabs). ToolRail has wrong tool set with unicode glyphs. ToolStrip has wrong colors and no shape presets. InspectorPanel is read-only.

### LeftPanel (`components/left_panel.rs`)
- [ ] Restructure to 52px icon rail + 160px expandable inspector panel (React pattern)
- [ ] Remove tab-based layout — tools are always in the rail, inspector is the expandable panel
- [ ] Rail toggle button at bottom of rail to expand/collapse inspector
- [ ] ToolStrip positioned as fixed flyout at left:52px (not inline)

### ToolRail (`components/tool_rail.rs`)
- [ ] Match React tool set: select, sticky, rectangle, ellipse (disabled), line (disabled), connector (disabled), text (disabled), draw (disabled), eraser (disabled)
- [ ] Port SVG icons from React (inline SVG paths, 20x20, stroke-width 1.5, stroke-linecap square)
- [ ] Add separator groups between tool categories
- [ ] Active indicator: `::after` pseudo-element, 2px wide `--accent-green` bar on left edge
- [ ] Disabled tools: opacity 0.3, no click handler, "coming soon" tooltip
- [ ] Sticky/Rectangle clicks open ToolStrip flyout (don't set tool directly)
- [ ] Update `Tool` enum in `state/ui.rs` to match: Select, Sticky, Rectangle, Ellipse, Line, Connector, Text, Draw, Eraser

### ToolStrip (`components/tool_strip.rs`)
- [ ] Add shape presets: Square (120x120), Tall (100x160), Wide (200x100)
- [ ] Match React color presets: Red #D94B4B, Blue #4B7DD9, Green #4BAF6E (not current 6 colors)
- [ ] Square swatches (no border-radius), 28x28, active: `--accent-green` border
- [ ] Send correct `props` fields: `backgroundColor`, `borderColor`, `borderWidth` (not just `fill`)
- [ ] Place objects at viewport center (use placeholder center until canvas, e.g. 400,300)
- [ ] Map tool types: sticky → `"sticky_note"`, rectangle → `"rectangle"`
- [ ] Optimistic local add: insert object into BoardState + set selection immediately
- [ ] Close strip after adding object
- [ ] "Add" button styling: mono 10px, 600 weight, uppercase

### InspectorPanel (`components/inspector_panel.rs`)
- [ ] Add editable inputs: width, height (numeric, commit on blur/Enter)
- [ ] Add text inputs: title (for sticky notes), body textarea, font size
- [ ] Add appearance inputs: background color picker, border color picker, border width
- [ ] Send `object:update` frame on each field commit
- [ ] Read-only fields: position X/Y, rotation, z-index, version, ID (truncated to 8 chars)
- [ ] Multi-selection: show "N objects selected" count
- [ ] Delete button with `object:delete` frame
- [ ] Empty state: "No selection / Double click an object to inspect it"
- [ ] Match React styling: `auto 1fr` grid, mono 10px labels uppercase, 24px input height

---

## Phase 7: Right Panel (Chat + AI + Boards) — NEEDS REWORK

Current state: Flat tabbed panel (Chat/AI/Boards tabs). ChatPanel sends messages but no history loading or auto-scroll. AiPanel sends prompts but no response handling. MissionControl is a link list.

### RightPanel (`components/right_panel.rs`)
- [ ] Restructure to 52px icon rail + 320px expandable content panel (React pattern)
- [ ] Port SVG tab icons from React (grid, speech bubble, star)
- [ ] Click active tab to collapse panel
- [ ] Panel header with tab title + close (✕) button
- [ ] Rename "AI" tab label to "Field Notes"

### ChatPanel (`components/chat_panel.rs`)
- [ ] Send `chat:history` frame on mount (once per board)
- [ ] Handle `chat:history` response in `dispatch_frame` → populate ChatState
- [ ] Auto-scroll to bottom on new messages (`scrollIntoView`)
- [ ] Placeholder text: "Message as {username}..."
- [ ] Disable send button when input is empty
- [ ] Empty state: "No messages yet"
- [ ] Match React styling: mono fonts, no border-radius on input, green send button

### AiPanel (`components/ai_panel.rs`)
- [ ] Send `ai:history` frame on mount (once per board)
- [ ] Add `ai:prompt` response handler in `dispatch_frame` → append to AiState
- [ ] Add `mutations` field to `AiMessage` struct, display "N objects modified" badge
- [ ] Error role handling (red styling, border-left)
- [ ] Markdown rendering for assistant messages (basic: `<pre>` formatted or lightweight parser)
- [ ] Disable input and send button while loading
- [ ] Auto-scroll to bottom on new messages
- [ ] Pulse animation on "Thinking..." indicator
- [ ] Match React styling: ruled-paper background (`repeating-linear-gradient`), Caveat font for user messages, mono for assistant, full markdown CSS

### MissionControl (`components/mission_control.rs`)
- [ ] Use BoardCard components instead of link list
- [ ] Pass `active` prop to highlight current board
- [ ] Scrolling container with hidden scrollbar
- [ ] Board data: use same fetch approach as DashboardPage

---

## Phase 8: CSS + Styling — NEEDS FULL REWRITE

Current state: `styles/main.css` uses a completely wrong design system (dark navy/rose palette, system sans-serif, border-radius everywhere). Must be replaced entirely with the React client's design tokens and component styles.

### Design Token Port (`styles/main.css`)
- [ ] Port all CSS custom properties from `client/src/styles/global.css`:
  - Canvas: `--canvas-bg`, `--canvas-grid`, `--canvas-grid-major`
  - Backgrounds: `--bg-primary`, `--bg-secondary`, `--bg-nav`, `--bg-status-bar`
  - Text: `--text-primary`, `--text-secondary`, `--text-tertiary`, `--text-nav`, `--text-nav-active`
  - Accents: `--accent-green`, `--accent-error`
  - Borders: `--border-default`, `--border-subtle`
  - Object palette: `--obj-cream` through `--obj-moss` (8 colors)
  - User colors: `--user-0` through `--user-7` (8 colors)
  - Typography: `--font-mono` (IBM Plex Mono stack), `--font-script` (Caveat)
  - Spacing: `--space-xs` through `--space-xl`
  - Z-index: `--z-canvas-ui`, `--z-chrome`, `--z-modal`
  - Geometry: `--radius: 0`, `--shadow: none`
- [ ] Port dark mode overrides (18 token remaps under `.dark-mode`)
- [ ] Add Google Fonts import: IBM Plex Mono (400/500/600/700) + Caveat (400/700)

### Base Element Styles
- [ ] Body: `font-family: var(--font-mono)`, `font-size: 13px`, `overflow: hidden`, `background: var(--bg-primary)`
- [ ] Button/input/textarea/select: `border-radius: 0`, `font: inherit`, `border: 1px solid var(--border-default)`, `background: var(--bg-secondary)`
- [ ] Button hover/disabled states
- [ ] Input/textarea focus: `border-color: var(--text-primary)`
- [ ] Scrollbar: 8px wide, transparent track, `var(--border-default)` thumb
- [ ] Links: `var(--accent-green)`, hover `var(--text-primary)`

### Component Style Port
Port each React CSS module to BEM classes. Key visual requirements:
- [ ] **All border-radius: 0** (React uses `--radius: 0` everywhere)
- [ ] **No box-shadows** (React uses `--shadow: none`)
- [ ] **Toolbar**: `--bg-nav`, presence chips with 2px left-border (not pill), 6px square dots
- [ ] **LeftPanel**: 52px rail `--bg-nav` + 160px panel `--bg-secondary`
- [ ] **RightPanel**: 52px rail `--bg-nav` + 320px panel `--bg-secondary`
- [ ] **ToolRail**: `::after` green left-bar active indicator, SVG icons, separators
- [ ] **ToolStrip**: 36px horizontal bar, square swatches, mono add button
- [ ] **InspectorPanel**: `auto 1fr` grid, mono 10px uppercase labels
- [ ] **StatusBar**: 24px, `--bg-status-bar`, mono 11px uppercase, dividers
- [ ] **BoardStamp**: top-right position, solid `--bg-primary` bg, opacity 0.75, "Station Log" label, Caveat title
- [ ] **ChatPanel**: mono fonts, no border-radius, green send button
- [ ] **AiPanel**: ruled-paper background, Caveat user messages, mono assistant, markdown styles, pulse animation
- [ ] **BoardCard**: aspect-ratio 4/3, Caveat 22px name, mono 9px ID, no border-radius, active/mini variants
- [ ] **DashboardPage**: full-viewport flex, `--bg-nav` header 36px, dashed "+" card
- [ ] **UserFieldReport**: flat rectangle, avatar, Caveat name, mono stats
- [ ] **MissionControl**: scrolling BoardCard container
- [ ] **Dialog**: flat, Caveat input, underline-only style, green border buttons

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

## Execution Order for Remaining Work

CSS first so we can visually verify everything, then structural changes, then wiring:

1. **Phase 8** — CSS full rewrite (design tokens, base styles, all component styles)
2. **Phase 6** — Left panel restructure (rail+panel, tool rail, tool strip, inspector)
3. **Phase 7** — Right panel restructure (rail+panel, chat history, AI responses)
4. **Phase 5** — Toolbar + status bar polish (popover, presence, info display)
5. **Phase 3+4 remaining** — WebSocket sends (board:join, board:create, history loads) + dispatch handlers

## Implementation Notes

- **Each phase is a separate commit** (or small set of commits)
- **Follow project conventions:** `*_test.rs` files, no panics in lib code, `cargo fmt` + `cargo clippy` + `cargo test` before each commit
- **CSS-in-Rust:** No framework — plain CSS file with BEM naming, CSS custom properties for theme
- **Client-only code** gated behind `#[cfg(feature = "hydrate")]`
- **Verification after each phase:** `cargo check` (both hydrate+wasm32 and ssr), `cargo test --workspace`, `docker compose up --build` and visually compare against React client on port 3000
