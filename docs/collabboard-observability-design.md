# CollabBoard Observability View — Design Specification

## Implementation Status (February 19, 2026)

### Completed

- A new standalone workspace crate, `traces`, has been added.
- `traces` provides UI-agnostic observability primitives for frame streams:
  - prefix/category display mapping
  - default and configurable trace filtering
  - trace session grouping from parent chains
  - request/done(error/cancel) span pairing for waterfall timing
  - aggregate metrics derivation (counts, errors, pending)
  - helper extraction for common sub-label fields
- Test coverage for `traces` has been expanded to include edge cases and branch behavior.
- A hygiene ratchet test suite (modeled after `canvas`) has been added to `traces`.

### In Progress / Not Started

- No `client` integration yet (no state plumbing or rendering path using `traces`).
- No observability UI route/view has been implemented yet.
- No server history API endpoint has been added yet (`GET /api/boards/:board_id/frames`).
- No toolbar mode toggle (`◎ TRACE`) has been implemented yet.
- No playback/live controls or keyboard shortcut bindings have been implemented yet.

## Overview

A new full-width layout that replaces the center canvas column, providing trace-level observability for AI interactions and system events within CollabBoard. The view retains the top header bar and bottom status bar from the existing app shell.

This spec synthesizes three sources:
- **LangSmith** tracing UI (hierarchical run tree + waterfall + detail inspector)
- **CollabBoard** current design language (dark theme, uppercase monospace labels, inspector patterns)
- **Legacy mockup** (frame viewer with timestamp log, kernel state panels, JSON detail views, playback controls)

---

## Design Language Reference

### Extracted from Current CollabBoard

| Token | Value | Source |
|-------|-------|--------|
| Background (primary) | `#1a1a1a` – `#222222` | Canvas / Inspector bg |
| Background (inset/input) | `#111111` – `#181818` | Input fields, code blocks |
| Background (panel/card) | `#2a2a2a` – `#333333` | Inspector sections, station log |
| Text (primary) | `#e0e0e0` – `#ffffff` | Values, content |
| Text (label/muted) | `#888888` – `#999999` | Section headers, field labels |
| Text (accent) | `#ff69b4` / magenta | Border color swatch, active states |
| Font (labels) | System monospace, uppercase, ~11px | `INSPECTOR`, `POSITION / META`, `APPEARANCE` |
| Font (values) | System monospace, normal case, 13px | Input field values |
| Font (content) | Sans-serif or monospace, 13px | Text content, chat messages |
| Border style | Minimal — thin 1px `#444` or none | Between sections |
| Accent color (success) | `#4ad981` (green) | Green square object, status indicators |
| Accent color (action) | Magenta/pink | Borders, interactive elements |
| Section pattern | `UPPERCASE_LABEL` header → content below | Inspector sections |
| Status bar | Full-width, dark bg, monospace, left-aligned state indicators | `■ WHOOPS THERE IT IS | 13 OBJS` |

### Extracted from Legacy Mockup

| Token | Value | Source |
|-------|-------|--------|
| Background (primary) | Warm off-white / parchment `#f5f0e8` | Note: Inverted for dark theme |
| Font (all UI) | Monospace throughout | Headers, labels, values, logs |
| Header pattern | `SCREAMING_SNAKE_CASE` | `FRAME_ACTIVITY`, `KERNEL_STATE`, `CONNECTION_STATUS` |
| Metric display | Large number + label below | `1,289` / `FRAMES_RECEIVED` |
| Log row format | `timestamp  type  EVENT_NAME  status  scope` | Left-aligned columns |
| Status badges | `OK` (green), `DONE` (green), `...` (muted) | Inline with log rows |
| Detail panel tabs | Underlined active tab | `OVERVIEW | DATA | TRACE` |
| JSON viewer | Monospace, indented, syntax-colored | Detail panel body |
| Key-value grid | `LABEL:` left, value right-aligned | `OP: REQ`, `SCOPE: MAIN` |
| Playback controls | `‖` pause / `▶` play + frame counter | Bottom of log column |
| Context card | Bordered card with muted header + bold values | `ACTIVE_FRAME_CONTEXT` |
| Type badges | Single-letter type prefix: `N`, `T`, `W`, `-` | Log row type column |

---

## Layout Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│  TOP HEADER BAR (existing — board name, user, logout)              │
├──────────┬──────────────────────────┬───────────────────────────────┤
│          │                          │                               │
│  COL 1   │        COL 2             │         COL 3                 │
│  TRACE   │     EVENT LOG /          │      DETAIL                   │
│  SUMMARY │     WATERFALL            │      INSPECTOR                │
│          │                          │                               │
│  ~240px  │       flex-1             │       ~400px                  │
│  fixed   │                          │       fixed                   │
│          │                          │                               │
├──────────┴──────────────────────────┴───────────────────────────────┤
│  BOTTOM STATUS BAR (existing — board name, obj count, coordinates) │
└─────────────────────────────────────────────────────────────────────┘
```

The three-column layout mirrors LangSmith's structure but uses CollabBoard's visual vocabulary.

---

## Column 1 — Trace Summary (Left Panel, ~240px fixed)

Combines LangSmith's sidebar navigation with the legacy mockup's `KERNEL_STATE` summary cards.

### Section: TRACE_ACTIVITY

Top-level metrics displayed as large monospace numbers (legacy mockup pattern). Counts are live from the WebSocket frame stream.

```
TRACE_ACTIVITY [LIVE]

    47
    FRAMES_RECEIVED

← RATE: 3/SEC
```

### Section: FRAME_STATE

Key-value summary of frame counts by syscall prefix and status. Follows the `KERNEL_STATE` pattern.

```
FRAME_STATE

board:*          4
object:*        12
ai:*             8
chat:*           6
cursor:*        17
─────────────────
ERRORS:          0
PENDING:         2
```

### Section: CONNECTION_STATUS

```
CONNECTION_STATUS

● WEBSOCKET OPEN
□ FRAMES: 47
□ BOARD_ID: whoops1
```

### Section: ACTIVE_TRACE_CONTEXT

Bordered card (legacy mockup's `ACTIVE_FRAME_CONTEXT` pattern).

```
┌─────────────────────────────┐
│ ACTIVE_TRACE_CONTEXT        │
│   BOARD_ID: whoops1         │
│   ROOT_FRAME: af223e40      │
│   ACTORS: user:af2, server  │
└─────────────────────────────┘
```

### Section: TRACE_INDEX (scrollable list)

Selectable list of recent traces/sessions (analogous to LangSmith's left run list and the legacy mockup's session tabs).

```
TRACE_INDEX

  ● af223e40  11:48:12  3.2s
  ○ bc891f23  11:47:05  1.8s
  ○ de456a89  11:45:33  0.4s
```

Each row: status dot, session/trace ID (truncated), timestamp, total duration. Selected trace is highlighted with the accent color (magenta/pink left border or background tint).

---

## Column 2 — Event Log / Waterfall (Center, flex-1)

The primary content area. Merges LangSmith's hierarchical run tree + waterfall bars with the legacy mockup's timestamped event log.

### Header Bar

```
┌─────────────────────────────────────────────────────────────┐
│  TRACE: af223e40  ◎ID    ⟳ Refresh    ☰ Filter    Compare  │
│  ▼ Waterfall  ⚙  ↕                     Last 1 day  1filter │
├─────────────────────────────────────────────────────────────┤
│  ☐  SYSCALL                         WATERFALL         FROM  │
└─────────────────────────────────────────────────────────────┘
```

- Trace ID with copy button (corresponds to the root `frame.id` for the session)
- View toggle: `Waterfall` (default) / `Log` (flat chronological, legacy style)
- Filter controls, refresh, compare button
- Column headers: checkbox, Syscall (frame.syscall), Waterfall bar area, From (frame.from)

### Waterfall View (Default)

Hierarchical tree with inline duration bars. Each row:

```
TIMESTAMP    ☐  [indent] SYSCALL                      ████████░░  STATUS  FROM
───────────────────────────────────────────────────────────────────────────────
11:48:12.001 ☐  ai:prompt                             ████████████  done  user:af2
11:48:12.045 ☐  ├── ai:llm_request     claude-sonnet  ██████░░░░░░  done  server
11:48:13.201 ☐  │   └── ai:tool_call   fetch_weather  ██░░░░░░░░░░  done  server
11:48:13.380 ☐  │       └── object:update  rect:b3e1  █░░░░░░░░░░░  done  server
11:48:13.412 ☐  ├── ai:llm_request     claude-sonnet  ████░░░░░░░░  done  server
11:48:14.156 ☐  └── chat:message                      ███░░░░░░░░░  item  server
```

**Row anatomy (left to right):**

| Element | Width | Style | Notes |
|---------|-------|-------|-------|
| Timestamp | ~100px | Monospace, muted `#888` | `HH:MM:SS.mmm` from `frame.ts` |
| Checkbox | 20px | Standard | Batch select |
| Tree indent | variable | `├──`, `│`, `└──` glyphs | Depth via `frame.parent_id` chain |
| Prefix badge | 24px | Single letter in circle/square | Derived from syscall prefix (see type derivation) |
| Syscall | flex | Monospace, primary text | `frame.syscall` — e.g., `ai:tool_call`, `object:update` |
| Sub-label | auto | Muted, smaller | Extracted from `frame.data` — model name, tool name, object ID |
| Waterfall bar | ~200px fixed | Colored bar proportional to parent duration | Green = done, amber = slow, red = error |
| Duration | 60px | Monospace, right-aligned | Computed: matching `done`/`error` frame ts minus `request` frame ts |
| Status | 50px | Colored badge | `frame.status`: `request`, `done`, `error`, `item` |
| From | 80px | Muted monospace | `frame.from` — e.g., `user:af2`, `server` |

**Waterfall bar behavior:**
- Bar fills proportionally to parent trace duration
- Bar starts at the offset position relative to trace start (so child calls show their actual timing position)
- Color coding: `#4ad981` (status `done`, green), `#ff69b4` (status `error`, pink accent), `#888` (status `item`/streaming), `#e6a23c` (status `request` still pending, amber)

### Log View (Alternative — Toggle)

Flat chronological list matching legacy mockup pattern exactly:

```
14:32:05.012   board:join            request   user:af2
14:32:05.234   board:join            done      user:af2
14:32:06.456   ai:prompt             request   user:af2
14:32:07.123   ai:llm_request        request   server          ●
14:32:08.789   ai:tool_call          done      server          ◁ ▷
14:32:09.012   object:update         done      server
14:32:09.234   chat:message          item      server
14:32:10.456   ai:prompt             done      server
```

Clicking any row in either view loads its details in Column 3.

### Playback Controls (Bottom of Column 2)

From legacy mockup. Anchored to bottom of the column.

```
┌─────────────────────────────────────────────────────────────┐
│  ‖  ▶        FRAME 47 OF 1,289        FILTER: ALL  MODE: ● │
└─────────────────────────────────────────────────────────────┘
```

- Pause / Play: Freezes the live stream
- Frame counter: Current position in trace
- Filter indicator: Active filter state
- Mode indicator: `LIVE` (green dot) or `REPLAY`

---

## Column 3 — Detail Inspector (Right Panel, ~400px fixed)

Mirrors the CollabBoard Inspector panel's structure and the legacy mockup's detail view. Appears when a row is selected in Column 2.

### Header

```
┌──────────────────────────────────────────────────────────┐
│  DETAIL_INSPECTOR                                    ✕   │
├──────────────────────────────────────────────────────────┤
│  T  ai:tool_call                                         │
│     fetch_weather                                        │
│                                                          │
│     FROM: server                                         │
├──────────────────────────────────────────────────────────┤
│  OVERVIEW    DATA    TRACE                               │
└──────────────────────────────────────────────────────────┘
```

- Close button (✕) top-right
- Prefix badge (large) + `frame.syscall`
- Sub-label (extracted from `frame.data` — tool name, model name, object ID)
- `frame.from` identifier
- Tab bar: `OVERVIEW` | `DATA` | `TRACE` (underline-style active indicator, from legacy mockup)

### Tab: OVERVIEW (Default)

Key-value metadata grid (legacy mockup's `TASK:LEASE` detail pattern + LangSmith's metrics sidebar).

```
FRAME_IDENTIFIER / SYSCALL

┌──────────────────────────────────────┐
│        ai:tool_call                  │
│                                      │
│   FROM: server                       │
└──────────────────────────────────────┘

FRAME_METRICS
─────────────────────────────────────

ID                      a1b2c3d4
PARENT_ID               af223e40
TS              11:48:13.201 PST
STATUS                   ● done
BOARD_ID                 whoops1

COMPUTED_METRICS
─────────────────────────────────────

DURATION                   0.18s    (done.ts - request.ts)
TOKENS                       70    (from frame.data if ai:*)
EST_COST               $0.0001    (from frame.data if ai:*)

DATA_PREVIEW
─────────────────────────────────────

{
  "tool": "fetch_weather",
  "args": { "city": "Salt Lake City" },
  "result": "Light rain"
}
```

**Prefix badge colors** (derived from `frame.syscall` prefix, mapped to CollabBoard palette):

| Syscall Prefix | Badge Color | Letter | Examples |
|----------------|-------------|--------|----------|
| `board:*` | `#5b9bd5` (blue) | B | `board:join`, `board:leave`, `board:create` |
| `object:*` | `#e6a23c` (amber) | O | `object:create`, `object:update`, `object:delete` |
| `ai:*` | `#4ad981` (green, from existing CollabBoard) | A | `ai:prompt`, `ai:llm_request`, `ai:tool_call` |
| `chat:*` | `#888888` (muted) | C | `chat:message`, `chat:history` |
| `cursor:*` | `#b388ff` (purple) | U | `cursor:move`, `cursor:broadcast` |
| `save:*` | `#ff69b4` (pink, existing accent) | S | `save:create`, `save:rewind` |

**Status badge colors** (from `frame.status`):

| Status | Color | Display |
|--------|-------|---------|
| `request` | `#e6a23c` (amber) | Pulsing dot — in-flight |
| `done` | `#4ad981` (green) | Solid dot |
| `error` | `#ff69b4` (pink) | Solid dot |
| `item` | `#888888` (muted) | Streaming indicator `...` |

### Tab: DATA

Collapsible Input/Output sections with JSON viewer (LangSmith pattern + legacy mockup JSON display).

```
INPUT ∨
─────────────────────────────────────

  HUMAN

  What's the weather like in
  Salt Lake City?


OUTPUT ∨
─────────────────────────────────────

  AI

  fetch_weather   CALLED  ⟩

  {
    "city": "Salt Lake City"
  }

  ──────────────────────────
  JSON ∨
```

**For Tool-type events:**

```
INPUT ∨
─────────────────────────────────────

  ▸ input

    Salt Lake City


OUTPUT ∨
─────────────────────────────────────

  ▸ output

    Light rain
```

**Design details:**
- Collapsible sections use `∨` / `∧` toggles
- JSON blocks use monospace font in an inset dark background (`#111`)
- The `CALLED` badge is interactive — clicking navigates to the linked tool execution row in Column 2
- `HUMAN` and `AI` role labels are uppercase, muted color, with content indented below

### Tab: TRACE

Shows the parent/child hierarchy for this specific event, as a mini-tree. Allows navigating up to the parent or down to children without scrolling the main waterfall.

```
TRACE_HIERARCHY
─────────────────────────────────────

  A  ai:prompt                  3.2s
  └── A  ai:llm_request         1.20s
      └── A  ai:tool_call ←HERE 0.18s
          └── O  object:update  0.01s
```

The `←HERE` indicator marks the currently inspected event. Clicking any other node navigates to it.

---

## Interaction Patterns

### Selection & Navigation

1. **Click row in Column 2** → Column 3 loads that event's detail
2. **Click `CALLED` badge in Column 3 DATA tab** → Scrolls Column 2 to the linked event and selects it
3. **Click node in Column 3 TRACE tab** → Same behavior as above
4. **Click trace in Column 1 TRACE_INDEX** → Loads that trace into Column 2, clears Column 3

### Filtering

- Filter button in Column 2 header opens a dropdown with syscall prefix checkboxes: `☑ ai:* ☑ object:* ☑ board:* ☑ chat:* ☐ cursor:*`
- Active filter count shown as badge (e.g., `1 filter`)
- Status filter row: `☑ request ☑ done ☑ error ☐ item` (item/streaming frames are high-volume, off by default)
- Status bar updates to reflect filtered count: `■ TRACE | 12 / 47 FRAMES | FILTER: ai:*+object:*`

### Live Mode

- When `MODE: LIVE`, new events append to the bottom of Column 2 and auto-scroll
- Clicking `‖` (pause) freezes the view for inspection
- Column 1 metrics continue updating in real-time regardless of pause state
- The `RATE: n/SEC` indicator pulses with a subtle animation when actively receiving

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate rows in Column 2 |
| `←` / `→` | Collapse / Expand tree nodes |
| `Enter` | Select row → load in Column 3 |
| `Space` | Toggle pause/play |
| `Esc` | Close Column 3 detail panel |
| `F` | Focus filter input |
| `1` / `2` / `3` | Switch Column 3 tabs (Overview/Data/Trace) |

---

## Status Bar Integration

The existing bottom status bar adapts to show observability context:

**Normal (canvas) mode:**
```
■ WHOOPS THERE IT IS | 13 OBJS | (-, -) (0, 0) 100%
```

**Observability mode:**
```
■ WHOOPS THERE IT IS | TRACE: af223e40 | 47 FRAMES | FILTER: ALL | MODE: LIVE ● | LATENCY: 1.2s avg
```

---

## Responsive Behavior

| Viewport | Column 1 | Column 2 | Column 3 |
|----------|----------|----------|----------|
| ≥1400px | Visible (240px) | flex-1 | Visible (400px) |
| 1000–1399px | Collapsible (icon rail, 48px) | flex-1 | Visible (360px) |
| <1000px | Hidden (toggle) | flex-1 | Overlay drawer (100%) |

Column 1 collapses to an icon rail showing only the metric numbers. Column 3 becomes a slide-over panel on narrow viewports.

---

## State Machine: View Modes

```
         ┌──────────┐
    ┌───►│  IDLE    │ (no trace selected)
    │    └────┬─────┘
    │         │ select trace
    │    ┌────▼─────┐
    │    │  VIEWING │ (trace loaded, Column 2 populated)
    │    └────┬─────┘
    │         │ select event
    │    ┌────▼──────┐
    │    │ INSPECTING│ (Column 3 open with event detail)
    │    └────┬──────┘
    │         │ Esc / close
    │    ┌────▼─────┐
    └────┤  VIEWING │
         └──────────┘

  Live mode is orthogonal — any state can be LIVE or PAUSED.
```

---

## Data Model — Aligned to Frame Protocol

The observability view consumes the **existing frame protocol** directly. No new data structures are needed — frames already contain all required fields.

### Core Frame (Existing — From Server)

Per the README, each WebSocket message is already a frame with:

```rust
/// Existing frame structure — no changes needed
struct Frame {
    id: String,          // unique frame ID (UUIDv7 or similar)
    parent_id: Option<String>,  // links child frames to parent (tree hierarchy)
    ts: String,          // ISO 8601 timestamp with ms precision
    syscall: String,     // e.g., "ai:prompt", "object:update", "board:join"
    status: String,      // "request" | "done" | "error" | "item"
    board_id: String,    // which board this frame belongs to
    from: String,        // origin — e.g., "user:af223e40", "server"
    data: serde_json::Value,  // JSON payload — varies by syscall
}
```

### Syscall Prefix → Display Category

The view derives display categories from the syscall prefix at render time. No enum stored — just a prefix match:

```rust
fn prefix_category(syscall: &str) -> (&str, &str, &str) {
    // Returns: (letter, label, hex_color)
    match syscall.split(':').next().unwrap_or("") {
        "board"  => ("B", "BOARD",  "#5b9bd5"),
        "object" => ("O", "OBJECT", "#e6a23c"),
        "ai"     => ("A", "AI",     "#4ad981"),
        "chat"   => ("C", "CHAT",   "#888888"),
        "cursor" => ("U", "CURSOR", "#b388ff"),
        "save"   => ("S", "SAVE",   "#ff69b4"),
        _        => ("-", "OTHER",  "#666666"),
    }
}
```

### Computed Values (Client-Side)

These are derived at render time, not stored:

```rust
/// Duration: match a "done"/"error" frame to its "request" frame via same parent_id + syscall
/// duration_ms = done_frame.ts - request_frame.ts

/// Tree depth: walk parent_id chain to root (parent_id == None)

/// Tokens / cost: extract from frame.data when syscall starts with "ai:"
///   e.g., data.tokens, data.cost_usd, data.model

/// Sub-label: extract from frame.data based on syscall:
///   "ai:llm_request"  → data.model (e.g., "claude-sonnet-4-20250514")
///   "ai:tool_call"    → data.tool  (e.g., "fetch_weather")
///   "object:update"   → data.id    (e.g., "rect:b3e1a4")
///   "chat:message"    → data.from  (e.g., "IANZEPP")
```

### Trace Session (Client-Side Grouping)

A "trace" in the observability view is a client-side grouping of frames, not a server concept:

```rust
/// A trace session groups frames for display purposes.
/// Simplest approach: group by board_id + time window,
/// or use a root frame (e.g., a "board:join" with no parent_id) as the anchor.
struct TraceSession {
    root_frame_id: String,       // the frame with no parent_id that anchors this trace
    board_id: String,
    frames: Vec<Frame>,          // all frames in this session, ordered by ts
    started_at: String,          // first frame ts
    ended_at: Option<String>,    // last frame ts (None if live)
}

/// Aggregate metrics computed from frames:
impl TraceSession {
    fn total_frames(&self) -> usize { self.frames.len() }

    fn total_tokens(&self) -> u64 {
        self.frames.iter()
            .filter(|f| f.syscall.starts_with("ai:") && f.status == "done")
            .filter_map(|f| f.data.get("tokens")?.as_u64())
            .sum()
    }

    fn total_cost(&self) -> f64 {
        self.frames.iter()
            .filter(|f| f.syscall.starts_with("ai:") && f.status == "done")
            .filter_map(|f| f.data.get("cost_usd")?.as_f64())
            .sum()
    }

    fn error_count(&self) -> usize {
        self.frames.iter().filter(|f| f.status == "error").count()
    }
}
```

### Frame Persistence (Existing Infrastructure)

Per the README:
- Frame events are already persisted through a **bounded async queue + batched writer**
- This existing pipeline is the replay/history source — no new persistence needed
- The observability view queries historical frames via a new API endpoint (see below)

### New API Endpoint (Server-Side Addition)

One new endpoint to fetch historical frames for a board:

```
GET /api/boards/:board_id/frames?after=<ISO_TS>&before=<ISO_TS>&syscall=<prefix>&limit=<n>
```

Query parameters:
- `after` / `before` — time range filter on `frame.ts`
- `syscall` — prefix filter (e.g., `ai:` returns all `ai:*` frames)
- `limit` — max frames returned (default 1000)
- `status` — filter by status (e.g., `done,error` to exclude `item` noise)

Returns: JSON array of frames, ordered by `ts` ascending.

The live WebSocket connection continues to deliver frames in real-time — the API endpoint is only for loading history when entering the observability view or scrubbing to a past time range.

### Request → Done Span Pairing

The waterfall view needs to pair `request` frames with their corresponding `done`/`error` frames to compute durations and draw span bars. The pairing logic:

```rust
/// Two frames form a span if:
///   1. They share the same parent_id (or both are root frames for the same syscall)
///   2. They have the same syscall
///   3. One has status "request", the other has status "done" or "error"
///
/// Edge cases:
///   - "item" frames are intermediate streaming frames — they don't close a span
///   - Some syscalls may only emit "done" without a prior "request" (fire-and-forget)
///     These render as instant events (zero-width bar at a single timestamp)
///   - A "request" without a matching "done"/"error" is an open/pending span
///     (animated pulsing bar that extends to current time)
```

---

## Implementation Notes

- The observability view is toggled from the existing toolbar (new `◎ TRACE` toggle button)
- **No new WebSocket connection needed** — the view subscribes to the same board WebSocket, consuming the existing frame stream. The only difference is rendering: canvas view interprets `object:*` frames as mutations, observability view displays all frames as log rows
- **No new persistence needed** — the existing bounded async queue + batched writer already persists frames. The new `/api/boards/:board_id/frames` endpoint queries this same store
- Column 2's waterfall bars are pure CSS: `width` as percentage of parent span duration, `margin-left` as percentage offset from trace start time
- Column 3 reuses the same panel/section Leptos components as the existing Inspector (same padding, font sizes, label patterns)
- The `FRAME n OF m` counter in playback controls maps to index position within `TraceSession.frames`
- `cursor:*` frames are filtered out by default (high frequency, low signal) but can be toggled on
- The `item` status frames (streaming) are also filtered by default to reduce noise
