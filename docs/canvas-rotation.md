# Canvas View Rotation Plan

## Goal
Add optional canvas-view rotation (rotate the whole world plane in screen space) while keeping all existing behavior unchanged when rotation is `0`.

This is **camera/view rotation**, not object 3D transform. Objects remain on the same 2D plane.

## Non-Goals
- No perspective / tilt / foreshortening.
- No WebGL migration.
- No breaking API changes.

## Compatibility Contract
- Existing client code continues working with no changes.
- Default rotation is `0deg`.
- All current tests should pass unchanged.
- Any new API is additive only.

## High-Level Approach
1. Extend camera model with view rotation.
2. Make coordinate conversions rotation-aware.
3. Apply view rotation in render pipeline around viewport center.
4. Ensure input/hit-test uses rotation-correct world coordinates.
5. Add optional host/UI controls later (outside core canvas scope).

## Phase 1: Camera Math (Core, Safe)
### Changes
- `canvas/src/camera.rs`
  - Add `view_rotation_deg: f64` to `Camera`.
  - Keep existing fields (`pan_x`, `pan_y`, `zoom`) unchanged.
  - Update:
    - `screen_to_world(...)`
    - `world_to_screen(...)`
  - Use viewport-center-based rotation convention (same pivot as render).

- `canvas/src/engine.rs`
  - Ensure camera conversion calls provide required context (viewport center where needed).
  - If needed, add internal helper methods on `EngineCore` to map points.

### Invariants
- Round-trip remains stable: `world -> screen -> world` and `screen -> world -> screen`.
- With `view_rotation_deg = 0`, behavior equals current implementation.

### Tests
- `canvas/src/camera_test.rs`
  - Add round-trip tests at 0/90/180/270 and fractional angles.
  - Add mixed pan+zoom+rotation tests.

## Phase 2: Render Transform
### Changes
- `canvas/src/render.rs`
  - Current pipeline: `translate(pan) -> scale(zoom)`.
  - New pipeline conceptually:
    - translate to viewport center
    - rotate by view rotation
    - translate back
    - then apply world transform
  - Keep object-local rotation logic unchanged.

### Grid
- Update world bounds sampling for grid drawing to avoid clipping at rotated viewport corners.
- Practical approach: overscan world bounds by diagonal-derived margin.

### Tests
- Add render-oriented tests where feasible (mostly engine behavior tests if render is hard to assert directly).

## Phase 3: Input + Hit-Test Integrity
### Changes
- `canvas/src/engine.rs`
  - Pointer/wheel handlers already consume world points from camera conversion.
  - Verify all gesture states (drag, resize, rotate, pan, wheel zoom) behave correctly under non-zero view rotation.

- `canvas/src/hit.rs`
  - No model changes expected if world-point input is correct.
  - Verify handle hit priority still works with rotated view.

### Tests
- `canvas/src/engine_test.rs`
  - Add interaction scenarios with non-zero view rotation:
    - select object
    - drag object
    - resize handle
    - rotate handle
    - pan drag
    - wheel zoom around cursor

- `canvas/src/hit_test.rs`
  - Add targeted hit tests for rotated view inputs.

## Phase 4: Optional Host Integration (Client)
This is outside canvas core but required to expose the feature in UI.

### Additive API Surface (recommended)
- Add optional methods in `Engine` / `EngineCore`:
  - `set_view_rotation_deg(f64)`
  - `view_rotation_deg() -> f64`

These are additive and preserve old call sites.

### Client wiring
- `client/src/components/canvas_host.rs`
  - Add optional controls / shortcuts to set rotation.
  - Add a bottom-left "view compass" control for canvas rotation.
    - Visual: small compass/ring with a draggable handle.
    - Dragging the ring/handle rotates view continuously.
    - Click cardinal markers to snap to `0/90/180/270`.
    - Double-click center to reset to `0`.
    - Show current angle near compass while dragging.
  - Keep default at 0.

- `client/src/state/canvas_view.rs` (optional)
  - Add telemetry field for view rotation if status UI needs it.

## Risk Areas
- Gesture feel under rotation (especially panning and wheel zoom anchoring).
- Grid clipping/perf due to rotated extents.
- Subtle float drift in repeated transforms.

## Rollout Strategy
1. Land Phase 1 with strict tests.
2. Land Phase 2 behind default `0deg` (no visible change).
3. Land Phase 3 tests + fixes.
4. Expose controls only after behavior is stable.

## Validation Checklist
- Existing canvas tests pass.
- New rotation tests pass.
- Manual checks:
  - Rotation 0 behaves identically to today.
  - Rotation 90/180/270 keeps selection and drag accurate.
  - Wheel zoom still anchors under cursor.
  - Pan direction remains intuitive.
  - Compass control:
    - Drag updates view rotation smoothly.
    - Snap targets (`0/90/180/270`) are accurate.
    - Reset returns view rotation to `0`.

## Manual QA Matrix
Use these exact checks in browser before rollout:

1. Compass Drag
- Action: Drag compass knob in a full circle clockwise/counterclockwise.
- Expect: angle readout tracks smoothly without jumps.
- Expect: world (objects + grid) rotates; object data is unchanged.

2. Cardinal Snap Tolerance
- Action: Drag close to `0/90/180/270` (within a few degrees).
- Expect: rotation snaps to the nearest cardinal angle.

3. Shift Snap
- Action: Hold `Shift` while dragging compass.
- Expect: rotation snaps to 15-degree increments.

4. Reset
- Action: Double-click compass center.
- Expect: rotation becomes exactly `0`.

5. Selection + Handles
- Action: At `90` and `180`, select objects and hit resize/rotate handles.
- Expect: handle hit-tests match cursor position.

6. Drag + Pan + Zoom
- Action: Drag object, pan canvas, and wheel-zoom around cursor at `33` and `90`.
- Expect: object movement remains accurate; pan feels consistent; zoom anchors under cursor.

7. Overlay Alignment
- Action: With non-zero rotation, inspect remote cursors and YouTube overlays.
- Expect: overlay positions stay aligned with rotated world.

8. Follow/Jump Parity
- Action: User A rotates/zooms/pans; User B follows or jumps to A.
- Expect: B matches A center/zoom/rotation.

## Estimated Effort
- Phase 1: Medium
- Phase 2: Medium
- Phase 3: Medium-High
- Phase 4: Small-Medium (depends on UX choices)
