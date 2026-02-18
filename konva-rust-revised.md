# Konva-Rust (Revised): Rust/Leptos Whiteboard Canvas Engine

## Context

This is not a Konva.js reimplementation for API parity. The actual goal is to replace a React + Konva front-end with a Rust + Leptos front-end, while preserving Konva-like whiteboard capabilities:

- Infinite canvas (pan/zoom) and object manipulation
- Shapes (rect/ellipse/diamond/star), edges (line/arrow), frames (later), rotation
- Text on shapes (plain text v0, rich text later), collaborative-ish via a Rust backend

This proposal assumes the project will happen; it is scoped for first-pass implementation success.

## Goals

- Rust/WASM client written in Leptos (React removed).
- Clean separation between document model (state), rendering, and input control.
- Simple, future-proof wire model (no migrations for common later features).
- Chrome/Brave/Safari support.

## Non-Goals (v0)

- No Konva API compatibility promises (names may be familiar, behavior is Konva-like).
- No performance target like “10k shapes”.
- No smart connector routing, no multi-select, no pen/highlighter, no obstacle avoidance.
- No text CRDT/merging; assume single editor at a time.

## Design Principle

Make the canvas engine imperative and self-contained. Leptos hosts it and binds UI controls/events, but does not become the scene graph.

## Architecture (Client)

- **Doc store**: `HashMap<ObjectId, BoardObject>` for the current board state (hydrated from server snapshot + live updates). Supports insert, update, and remove-by-id.
- **BoardRuntime**: owns camera, selection, tool state machine, and the render loop.
- **Renderer**: draws the doc to a single `<canvas>` (grid + shapes + edges + selection UI + text via `fillText`).

Key rule: all persisted geometry is in **world coordinates**. Camera converts world <-> screen.

### Concrete Module Split (Actionable)

This is the minimal split that prevents a rewrite when you add features later:

- `doc`: wire types and defaults (`BoardObject`, `ObjectKind`, `Props` helpers)
- `camera`: `Camera { pan, zoom }` plus `screen_to_world/world_to_screen`
- `render`: `draw(ctx, &doc, &camera, &ui_state)`
- `hit`: hit-testing primitives (node interior, resize handles, edge endpoints)
- `input`: pointer/key state machine that emits local mutations as `Action` values

Note: `net` (websocket client) is **not** part of the canvas crate. The host app (Leptos) owns the network connection and feeds snapshots/updates into the doc store.

### Crate Public API Boundary (Actionable)

The canvas crate is a pure logic + rendering library. It does not own the network, the DOM, or the application lifecycle. The host app (Leptos) drives it through a narrow API.

**Host -> Crate (inputs):**

- `Engine::new(canvas: HtmlCanvasElement)` — initialize with a canvas element
- `engine.set_viewport(width_css: f64, height_css: f64, dpr: f64)` — resize/DPR changes
- `engine.load_snapshot(objects: Vec<BoardObject>)` — hydrate from server snapshot
- `engine.apply_create(object: BoardObject)` — server broadcast: object created
- `engine.apply_update(id: ObjectId, fields: PartialBoardObject)` — server broadcast: object updated
- `engine.apply_delete(id: ObjectId)` — server broadcast: object removed
- `engine.on_pointer_down(screen_pt: Point, button: Button, modifiers: Modifiers)`
- `engine.on_pointer_move(screen_pt: Point, modifiers: Modifiers)`
- `engine.on_pointer_up(screen_pt: Point, button: Button, modifiers: Modifiers)`
- `engine.on_wheel(screen_pt: Point, delta: WheelDelta, modifiers: Modifiers)`
- `engine.on_key_down(key: Key, modifiers: Modifiers)`
- `engine.on_key_up(key: Key, modifiers: Modifiers)`
- `engine.set_tool(tool: Tool)` — select, rect, ellipse, diamond, star, line, arrow
- `engine.set_text(id: ObjectId, head: String, text: String, foot: String)` — commit from Leptos editor
- `engine.render()` — draw current state to canvas

**Crate -> Host (outputs):**

The crate returns `Action` values from input methods. The host inspects these and decides what to do (send to server, update UI chrome, etc.):

```rust
enum Action {
    None,
    /// Object was created locally; host should send object:create
    ObjectCreated(BoardObject),
    /// Object was mutated locally; host should send object:update on gesture end
    ObjectUpdated { id: ObjectId, fields: PartialBoardObject },
    /// Object should be deleted; host should send object:delete
    ObjectDeleted { id: ObjectId },
    /// Request the host to enter text edit mode for this object
    EditTextRequested { id: ObjectId, head: String, text: String, foot: String },
    /// Cursor style hint (e.g., "grab", "crosshair", "nw-resize")
    SetCursor(String),
    /// Render is needed (call engine.render())
    RenderNeeded,
}
```

Input methods may return multiple actions (e.g., `ObjectCreated` + `RenderNeeded`). Use `Vec<Action>` or a small fixed-size return.

**Crate -> Host (queries):**

The host can read state from the crate without mutation:

- `engine.selection() -> Option<ObjectId>` — currently selected object
- `engine.camera() -> Camera` — current pan/zoom for UI display
- `engine.object(id: ObjectId) -> Option<&BoardObject>` — read an object

### Width/Height Convention

- For node shapes (`rect`, `ellipse`, `diamond`, `star`): `width` and `height` are required. If missing from the wire (server sends `null`), the crate defaults to `0.0` and skips rendering.
- For edge shapes (`line`, `arrow`): `width` and `height` are present on the wire but not authoritative. The crate uses `props.a` and `props.b` endpoints for rendering and hit-testing.

### Kind Names

The crate uses the kind names defined in this doc: `rect`, `ellipse`, `diamond`, `star`, `line`, `arrow`. The existing front-end kind names (`sticky_note`, `rectangle`, `connector`, `text`) are legacy and will be migrated as part of the TS -> Rust rewrite.

## Coordinate System (Critical v0)

Define:

- `world`: canonical persisted coordinates (objects, endpoints, cursors)
- `screen`: CSS pixels of the viewport (what pointer events give you)
- `dpr`: `window.devicePixelRatio`

BoardRuntime provides:

- `screen_to_world(screen_pt) -> world_pt`
- `world_to_screen(world_pt) -> screen_pt`

This must be correct (especially in Safari) before implementing rotation/resize/hit-testing.

### Canvas Sizing Rules (Actionable)

- Canvas element has CSS size `{w_css, h_css}`.
- Backing store is `{w_px = w_css * dpr, h_px = h_css * dpr}`.
- Before drawing, call `ctx.setTransform(dpr, 0, 0, dpr, 0, 0)` so all subsequent drawing uses CSS pixels.
- Apply camera transform in CSS pixel space:
  - `ctx.translate(camera.pan_x, camera.pan_y)`
  - `ctx.scale(camera.zoom, camera.zoom)`

Store `pan_x/pan_y` in CSS pixels, and store all object geometry in world units where 1 world unit == 1 CSS pixel at zoom 1.

## Wire Model (Server <-> Client)

The existing backend already supports a "snapshot + op stream" model:

- snapshot: `board:join` returns the list of `BoardObject`
- ops: broadcast `object:create`, `object:update`, `object:delete`

This model is sufficient for v0. The Leptos rewrite can keep using it.

### BoardObject (Canonical Fields)

Lowercase kinds on the wire (Rust can use PascalCase enums with serde renames).

```json
{
  "id": "uuid",
  "board_id": "uuid",
  "kind": "rect | ellipse | diamond | star | line | arrow",
  "x": 0,
  "y": 0,
  "width": 100,
  "height": 80,
  "rotation": 0,
  "z_index": 0,
  "props": {},
  "created_by": "uuid|null",
  "version": 1
}
```

Canonical geometry for node-like shapes is always `x/y/width/height/rotation`.

For `line`/`arrow`, endpoints are stored in `props` (future-proof), and `x/y/width/height/rotation` are present but not authoritative.

### Rotation Units (Actionable)

Use degrees on the wire (`rotation: f64`), matching Konva and your existing React/Konva client behavior.

### Props Schema (v0)

Keep `props` as JSON on the wire, but treat it as a typed schema in the client.

Common keys (all optional):

- `fill: string` CSS color
- `stroke: string` CSS color
- `stroke_width: number`
- `head: string` plain text (may contain newlines)
- `text: string` plain text v0 (Markdown deferred to later milestone)
- `foot: string` plain text (may contain newlines)

Kind-specific keys:

- `line` / `arrow`:
  - `a: { type: "free", x: number, y: number }`
  - `b: { type: "free", x: number, y: number }`

Defaults (v0, client-side; server remains unaware):

- `fill`: `#D94B4B` (or theme default)
- `stroke`: `#1F1A17`
- `stroke_width`: `1`
- `head/text/foot`: empty strings
- `a/b`: created at tool placement time

### Text Conventions (v0)

All text fields live in `props`:

- `props.head`: plain text (can contain newlines; renderer decides presentation per kind)
- `props.text`: plain text in v0 (Markdown interpretation deferred to later milestone)
- `props.foot`: plain text (renamed from "tail")

All text fields are rendered on canvas via `fillText`. Editing is handled by a Leptos-managed editor component positioned over the shape; on commit, text is written back into `props` and the editor is removed.

Note: all props keys use snake_case to match Rust conventions.

### Edge Endpoints (v0)

Use a future-proof shape now so v1 can add attachment without migrating existing edges.

```json
{
  "props": {
    "a": { "type": "free", "x": 10, "y": 20 },
    "b": { "type": "free", "x": 200, "y": 150 }
  }
}
```

Reserved for later (not implemented v0):

- `type: "attached"` with `{ object_id, anchor }`
- `anchor` values like `n/ne/e/se/s/sw/w/nw/center`

### Style Conventions (v0)

Per-shape styling is intentionally minimal:

- `props.fill`
- `props.stroke`
- `props.stroke_width`

No dash/gradients/shadows/filters in v0.

### Syscalls and Payloads (Actionable)

This matches the existing server frame protocol. Examples show only the relevant `data` keys.

- `board:join` request:
  - `board_id` set either on `frame.board_id` or in `data.board_id`
- `board:join` done response:
  - `data.objects: BoardObject[]`

- `object:create` request:
```json
{
  "syscall": "object:create",
  "data": {
    "kind": "rect",
    "x": 100,
    "y": 100,
    "width": 240,
    "height": 140,
    "rotation": 0,
    "props": { "fill": "#FDE68A", "stroke_width": 1, "head": "Note", "text": "Hello", "foot": "" }
  }
}
```

- `object:update` request (gesture commit):
```json
{
  "syscall": "object:update",
  "data": {
    "id": "uuid",
    "x": 120,
    "y": 110,
    "rotation": 15,
    "version": 7
  }
}
```

Note: when `props` is included in an update, the server performs a **shallow merge** — keys present in the update are set, keys absent are left unchanged, and keys set to `null` are removed. This prevents concurrent edits to different props fields from clobbering each other.

## Rendering

### Canvas

- Single `<canvas>` for geometry, grid, selection handles, edges.
- Geometric hit-testing (not a Konva-style hit canvas) for v0.
- Shapes interpret `x/y/width/height/rotation` as the authoritative box.

Notes:

- `diamond` and `star` are rendered inscribed in the object box in v0.
- Optional shape parameters (e.g., star points) can be added later under `props` without changing geometry fields.

### Shape Geometry (Actionable)

All node shapes are defined inside their local axis-aligned box, then rotated by `rotation` degrees around the box center.

- `rect`: draw `Rect(x,y,w,h)` then rotate around center
- `ellipse`: draw ellipse inscribed in `w x h`
- `diamond`: 4 points at midpoints of box edges
- `star`: v0 uses a fixed 5-point star inscribed in box, with inner radius = 0.5 * outer radius (tweakable later via props)

### Draw Order (Actionable)

Within a single render pass:

1. Grid/background (not persisted)
2. Objects sorted by `(z_index, id)` ascending
3. Selection UI (bounding box, handles, endpoint handles)
4. Remote cursors/presence (optional)

### Text (Canvas Rendered)

- Render `props.head`, `props.text`, and `props.foot` on canvas via `fillText`/`measureText`.
- All text is plain text in v0 (no Markdown, no links, no HTML).
- Text is drawn within the shape's bounding box, clipped or wrapped as needed.

Editing:

- Leptos spawns an editor component positioned over the shape when the user enters edit mode.
- On commit, text is written back into `props` and the editor is removed.

### Later: Rich Text (Post-v0)

- Markdown rendering for `props.text` (links, bold, etc.)
- DOM overlay for rendered Markdown with clickable links
- Link click vs drag disambiguation

## Input / Controller State Machine (v0)

Tools/states (v0):

- `select`: hit-test, select, drag move, resize, rotate
- `rect/ellipse/diamond/star`: create node with default props
- `line/arrow`: create edge; drag endpoints
- `pan/zoom`: wheel pan; ctrl/cmd + wheel zoom; drag-pan empty space

Hit-testing:

- Use a consistent “hit slop” (in screen pixels) for handles and thin edges.
- For rotated shapes, perform hit tests in local space (inverse-rotate pointer).

Resize/rotate:

- Define behavior in local axes (resize handles aligned to shape’s local frame).
- Rotation handle uses center pivot; snap can be added later.

### Hit Testing (Actionable)

Define a single function:

- `hit_test(world_pt) -> Option<Hit>` where `Hit` includes `{ object_id, part }`

Suggested `part` variants (v0):

- `Body` (interior of a node shape)
- `ResizeHandle(N|NE|E|SE|S|SW|W|NW)`
- `RotateHandle`
- `EdgeEndpoint(A|B)`
- `EdgeBody`

Algorithms (v0):

- Node body:
  - Convert `world_pt` into node local space by inverse-rotating about the box center.
  - Then test against the unrotated shape (rect bounds, ellipse equation, diamond polygon, star via inscribed ellipse).
- Edge endpoints:
  - Hit-test circles around `props.a` and `props.b` in screen space (convert to screen pt, compare to radius in px).
- Edge body:
  - Distance-to-segment (A->B) in world space; convert threshold from px to world units via `threshold_world = threshold_px / camera.zoom`.

### Input Commit Policy (Actionable, v0)

Because this is a portfolio project and only one user is expected to mutate at a time:

- Update locally on every pointermove for responsiveness.
- Send `object:update` on gesture end (pointerup) for:
  - node transform changes (x/y/width/height/rotation)
  - edge endpoint changes (send as props wholesale)
- Send `object:update` immediately on edit commit for text changes (props wholesale).

This keeps websocket traffic low and avoids complex reconciliation logic.

## Server Integration (v0)

Keep the current mechanics:

- Client performs local interactive updates optimistically.
- Persist at gesture boundaries (e.g., pointerup commits `object:update`).
- Server versioning stays as-is.

Important: the backend performs a shallow merge on `props` — keys present in the update are set, absent keys are unchanged, and `null` values remove keys. This prevents concurrent prop edits from clobbering each other.

### Server Compatibility Notes (Actionable)

- The server treats `kind` as an opaque string today, so new lowercase kinds like `diamond` and `star` require no schema changes.
- If AI tooling / prompts assume a fixed kind list (e.g. `sticky_note`, `connector`), those will need updating if you care about AI features in this portfolio.

## Milestones

### v0 (Core)

- Leptos app replaces React client.
- Camera pan/zoom infinite canvas, correct in Safari (DPR + pointer coords).
- Nodes: `rect`, `ellipse`, `diamond`, `star` with fill+border, move/resize/rotate.
- Edges: `line`, `arrow` with free endpoints (`props.a/props.b`), endpoint dragging.
- Text: `props.head`/`props.text`/`props.foot` rendered on canvas via `fillText`; editing via Leptos editor overlay.

### v0 Definition of Done (Actionable Checklist)

- Create, select, move, resize, rotate:
  - `rect`, `ellipse`, `diamond`, `star`
- Create and edit:
  - `line`, `arrow` with draggable endpoints
- Text:
  - head/text/foot renders correctly on canvas at different zoom levels
  - Edit mode spawns Leptos editor, commit writes back to props
- Delete:
  - Selected object can be removed; `object:delete` round-trips to server
- Persistence:
  - Create then reload board -> objects and endpoints rehydrate correctly
- Safari:
  - pointer coords and zoom are correct (no drift)

### v0.1 (After Core)

- Add `object:set` syscall as a multi-field patch API (set property paths to values).
  - Supports multiple sets per request.
  - `null` unsets props keys (or clears optional fields like width/height).
- Add lock syscalls (text + transform) for multi-user robustness, without blocking selection/read-only inspection.
- Optional: `object:update_many` batching for multi-select and group moves.

### Later (Additive Extensions)

- Attach endpoints: `{ type:"attached", object_id, anchor }` with `anchor` at corners/side-centers.
- Frames as grouping regions (dragging a frame moves its contained nodes).
- Multi-select, snapping, alignment guides.
- Pen/highlighter strokes.
- Smart routing / obstacle avoidance for edges.

## Main v0 Risks (and How to Contain Them)

- **Coordinate correctness (Safari/DPR)**: implement camera and conversion utilities first; use debug overlays early.
- **Rotation/resize math**: always convert pointer into local space for interactions; keep transforms explicit.
- **Text UX**: render plain text on canvas; Leptos editor for edit mode; Markdown deferred.
- **Hit-testing feel**: add screen-space slop for edges/handles; make selection/drag behavior deterministic.

## Implementation Order (Actionable)

This order is intentionally chosen to flush out coordinate and interaction bugs early.

1. Camera + DPR-correct canvas sizing + grid rendering.
2. World<->screen conversions + debug overlay (show world coords under cursor).
3. Doc store + snapshot hydration + broadcast apply (create/update/delete).
4. Render `rect` and selection UI (body hit-test + drag-move).
5. Implement rotate + resize in local space (handles + hit-test).
6. Add `ellipse`, `diamond`, `star` rendering + hit-test.
7. Add `line` endpoints (`props.a/props.b`) + endpoint dragging + arrowheads.
8. Add canvas text rendering (head/text/foot via `fillText`) + Leptos edit overlay + commit to server.
9. Safari checks and polish.

---

## Implementation Status

_Last updated after input state machine edge-case hardening._

### Module Status

| Module   | Status         | Tests | Notes |
|----------|----------------|-------|-------|
| `doc`    | **Done**       | Yes   | `BoardObject`, `ObjectKind`, `DocStore`, `Props`, `PartialBoardObject` all implemented and tested. |
| `camera` | **Done**       | Yes   | `Point`, `Camera`, `screen_to_world`, `world_to_screen`, `screen_dist_to_world`. |
| `consts` | **Done**       | —     | Shared numeric constants extracted into `consts.rs`. |
| `hit`    | **Done**       | Yes   | All geometry primitives, composite `hit_test()`, resize/rotate handle positions. 99 geometry tests. |
| `input`  | **Done**       | Yes   | `Tool`, `Modifiers`, `Button`, `Key`, `WheelDelta`, `UiState`, `InputState` (all variants). |
| `engine` | **Logic done, render stub** | Yes | `EngineCore` has full input handling, server event APIs, queries. 360+ tests including edge-case hardening. `Engine` wraps `EngineCore` + `HtmlCanvasElement`; `Engine::render()` is `todo!()`. |
| `render` | **Stub**       | No    | `draw()` signature exists, body is `todo!()`. |

### What Is Implemented (Matches Design)

- **Wire types**: `BoardObject` with all canonical fields, serde roundtrip, lowercase kind names on wire.
- **Doc store**: `HashMap<ObjectId, BoardObject>` with insert, remove, `apply_partial` (shallow merge on props), `load_snapshot`, `sorted_objects` by `(z_index, id)`.
- **Camera**: pan/zoom in CSS pixels, `screen_to_world`/`world_to_screen` conversions.
- **Hit testing**: All v0 algorithms implemented — rect/ellipse/diamond/star body tests in local space (inverse-rotate pointer), edge body via distance-to-segment, edge endpoints via point-near-point, resize handles (8 anchors), rotate handle. Composite `hit_test()` checks selected object handles first, then all objects in reverse draw order.
- **Input state machine**: All v0 tools and states — Select (hit-test → drag/resize/rotate/edge-endpoint-drag/pan), shape tools (Rect/Ellipse/Diamond/Star create node), edge tools (Line/Arrow create edge), pan (middle button or empty-space drag), wheel pan/zoom with ctrl/meta modifier. Escape cancels gestures, Delete/Backspace removes selected object.
- **Action emission**: `ObjectCreated` on drawing completion, `ObjectUpdated` on gesture end (drag/resize/rotate/edge-endpoint), `ObjectDeleted` on delete key, `SetCursor` on pan start, `RenderNeeded` on visual state changes.
- **Input commit policy**: Local updates on every pointermove, `ObjectUpdated` emitted on pointerup (gesture end) — matches design.
- **Public API boundary**: `Engine` delegates all methods to `EngineCore`. All designed Host→Crate inputs are implemented. All Crate→Host queries are implemented.
- **Props schema**: `fill`, `stroke`, `stroke_width`, `head`, `text`, `foot` all supported. Edge endpoints use `{ type: "free", x, y }` format.
- **Coordinate system**: All persisted geometry in world coordinates. Camera converts world↔screen correctly.
- **Constants**: `MIN_SHAPE_SIZE` (2.0), `ZOOM_FACTOR` (1.1), `ZOOM_MIN` (0.1), `ZOOM_MAX` (10.0), `HANDLE_RADIUS_PX` (8.0), `ROTATE_HANDLE_OFFSET_PX` (24.0), `STAR_INNER_RATIO` (0.5).

### What Is Stubbed

- **`render::draw()`**: Signature matches design (`ctx, &doc, &camera, &ui_state`), body is `todo!()`. No grid, shape, edge, selection UI, or text rendering yet.
- **`Engine::render()`**: Calls `draw()` which is `todo!()`.
- **Canvas sizing**: `set_viewport` stores `width_css`, `height_css`, `dpr` but no canvas backing store resize or `ctx.setTransform(dpr, ...)` yet (that lives in render).
- **Text rendering**: `set_text` writes to props and emits `ObjectUpdated`, but no `fillText` rendering on canvas.
- **`EditTextRequested` action**: Defined in `Action` enum but never emitted (no double-click-to-edit gesture).

### Deviations from Original Design

1. **`EngineCore` split**: The design describes a single `Engine` struct. Implementation splits into `EngineCore` (all logic, no browser dependencies) and `Engine` (wraps `EngineCore` + `HtmlCanvasElement`). This enables full test coverage without WASM/browser — all 360+ tests run in native `cargo test`.

2. **Star hit-testing uses polygon winding, not inscribed ellipse**: The design suggested "star via inscribed ellipse" for hit-testing. Implementation uses a proper 10-vertex polygon (alternating outer/inner points) with ray-casting, which is more accurate.

3. **Edge endpoint hit-testing in world space, not screen space**: The design said "hit-test circles around `props.a` and `props.b` in screen space." Implementation converts the screen-space handle radius to world space (`HANDLE_RADIUS_PX / zoom`) and tests in world space. Equivalent result, simpler code.

4. **Resize uses absolute delta from grab point, not incremental**: The original implementation computed incremental delta per move (`world_pt - last_world`) applied to original dimensions, which was buggy (only last move's delta survived). Fixed to use total delta from `start_world` (the initial grab point), matching how drag works.

5. **`consts.rs` module**: Not in the original module split. Added to share numeric constants (`HANDLE_RADIUS_PX`, `ZOOM_FACTOR`, `MIN_SHAPE_SIZE`, etc.) across `hit`, `engine`, and future `render`.

6. **`BoardRuntime` naming**: The design mentions "BoardRuntime" as the owner of camera/selection/tool state. Implementation uses `EngineCore` for this role — same responsibility, different name.

7. **Empty-space drag-to-pan on Select tool**: Not explicitly specified in the design's tool list, but implemented — clicking empty space with the Select tool starts a pan gesture (same as middle-button pan). This is standard whiteboard UX.

8. **Tool resets to Select after shape/edge creation**: Implemented but not explicitly stated in the design. After drawing a shape or edge, the active tool automatically resets to Select.

9. **Tiny shape discard**: Shapes below `MIN_SHAPE_SIZE` (2.0) in both width and height are deleted on pointerup. Edges are always kept (even zero-length). Not detailed in the design but follows standard drawing tool behavior.

