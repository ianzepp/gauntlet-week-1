# Konva-Rust: 2D Canvas Engine in Rust/WASM

## Overview

A full Rust reimplementation of the Konva.js 2D canvas library, compiled to WASM for browser delivery. Replaces both `konva` and `react-konva` with a unified Rust stack, integrating with a Rust web framework (Leptos, Dioxus, or Yew) instead of React.

## Goals

- Feature parity with Konva.js core (scene graph, shapes, events, transforms, animation, filters, caching)
- Zero JavaScript dependencies — pure Rust compiled to WASM
- Canvas 2D rendering via `web-sys` bindings initially, with `wgpu` as a future backend option
- Declarative component API through a Rust web framework, not React
- Idiomatic Rust: strong typing, no garbage collection overhead, ECS-friendly architecture where appropriate

## Architecture

### Rendering Backend

Primary target is HTML Canvas 2D via `web-sys::CanvasRenderingContext2d`. All draw calls go through a `Renderer` trait so backends are swappable. The hit-testing canvas (hidden, color-ID-based) is a second canvas instance managed per layer.

### Scene Graph

```
Stage (owns DOM container, dispatches events)
  Layer (owns scene canvas + hit canvas)
    Group (logical container, no canvas)
      Shape (concrete drawable: Rect, Circle, Text, Path, etc.)
```

All nodes share a common `Node` base via a trait hierarchy:

- `Node` — id, name, visibility, opacity, listening, transform props, event registration, caching, serialization
- `Container: Node` — children management, z-ordering, find/findOne selectors
- `Shape: Node` — fill, stroke, shadow, dash, line cap/join, custom `scene_func`/`hit_func`

Nodes are stored in a slotmap or arena allocator. Parent-child relationships use handles (IDs), not Rc/RefCell trees. This keeps borrows clean and enables efficient traversal.

### Transform System

Each node has: `x`, `y`, `scale_x`, `scale_y`, `rotation`, `skew_x`, `skew_y`, `offset_x`, `offset_y`. These compose into a 3x3 affine matrix (`[f64; 6]` or a `Transform` struct). Absolute transforms are computed by walking ancestors and multiplying. Cached/dirty-flagged to avoid redundant computation.

### Hit Detection

Dual-canvas approach matching Konva: each shape gets a unique `u32` color ID. Shapes draw to the hit canvas using their ID as the fill/stroke color. On pointer events, read the pixel at the pointer position, decode the color to a shape ID, and dispatch. This gives O(1) pixel-perfect hit testing for arbitrary shapes.

### Event System

Events are dispatched from the Stage, which binds to DOM events via `web-sys` (mouse, touch, pointer). The pipeline:

1. Native DOM event fires on the Stage's container
2. Stage normalizes coordinates (accounting for CSS scaling, scroll, devicePixelRatio)
3. Stage samples the active Layer's hit canvas at the pointer position
4. Target shape is resolved from the color ID
5. A `KonvaEvent` is constructed and fired on the target
6. Event bubbles: Shape -> Group -> ... -> Layer -> Stage
7. Drag/transform state machines update if active

Supported event types:
- Mouse: click, dblclick, mousedown/up/move/enter/leave/over/out, wheel
- Touch: touchstart/move/end, tap, dbltap
- Pointer: pointerdown/up/move/cancel/enter/leave/over/out, pointerclick, pointerdblclick
- Drag: dragstart, dragmove, dragend
- Transform: transformstart, transform, transformend

### Shapes (17 total)

| Shape | Notes |
|---|---|
| Rect | Corner radius support (per-corner) |
| Circle | Simple |
| Ellipse | Simple |
| Line | Polyline, tension-based spline smoothing |
| Arrow | Line with configurable arrowheads |
| Arc | Filled/stroked arc segments |
| Wedge | Pie slice |
| Ring | Donut (inner + outer radius) |
| RegularPolygon | N-sided |
| Star | N-pointed with inner/outer radius |
| Path | Full SVG path data parser (M, L, H, V, C, S, Q, T, A, Z + lowercase relative variants) |
| Text | Font selection, alignment, wrapping (word/char/none), ellipsis, letter-spacing, line-height, vertical alignment. Uses `web-sys` `measureText` for metrics |
| TextPath | Text rendered along an SVG path |
| Image | Bitmap rendering with crop rect |
| Label | Composite: Text + Tag with directional pointer |
| Sprite | Frame-based animation from spritesheet |
| Transformer | Interactive resize (8 handles) + rotation handle, multi-node, proportional/centered scaling, rotation snapping |

### Filters (20 total)

Pixel-level `ImageData` manipulation on cached nodes: Blur, Brighten, Contrast, Emboss, Enhance, Grayscale, HSL, HSV, Invert, Kaleidoscope, Mask, Noise, Pixelate, Posterize, RGB, RGBA, Sepia, Solarize, Threshold. Each filter is a function `fn(&mut [u8], width, height)` operating on raw RGBA pixel buffers.

### Animation

Two tiers:
- **AnimationLoop** — raw `requestAnimationFrame` callback loop via `web-sys`. Provides `delta_time` and `frame_count`. User controls what changes per frame.
- **Tween** — declarative property animation. Interpolates any numeric node attribute from current to target over a duration. Supports 14 easing functions (Linear, EaseIn/Out/InOut, Back, Elastic, Bounce, Strong variants), color interpolation, array interpolation (e.g., line points), yoyo mode, pause/resume/seek.

### Caching

Any node can call `.cache()` to render to an offscreen canvas buffer. Subsequent redraws use `drawImage` from the buffer. Filters apply to the cached pixel data. Dirty flags trigger re-cache when properties change.

### Serialization

`serde` for JSON serialization/deserialization of the full scene graph. Each node type is tagged with its shape kind for reconstruction. `clone()` does a deep copy of any subtree.

### Property System

Rust structs with typed fields replace Konva's factory-generated getters/setters. Change detection via dirty flags on each property group (transform, visual, geometry). Property changes emit events for framework reactivity.

## Web Framework Integration

Instead of react-konva's custom React reconciler, provide a component library for a Rust web framework. Leading candidate: **Leptos** (SSR support, fine-grained reactivity, strong WASM story).

Components map 1:1 to scene graph nodes:

```rust
#[component]
fn MyCanvas() -> impl IntoView {
    let (x, set_x) = signal(100.0);
    view! {
        <Stage width=800 height=600>
            <Layer>
                <Rect x=x y=50.0 width=100.0 height=80.0 fill="red"
                    on:click=move |_| set_x.update(|x| *x += 10.0) />
                <Circle x=300.0 y=200.0 radius=50.0 fill="blue" draggable=true />
                <Text x=10.0 y=10.0 text="Hello" font_size=24 />
            </Layer>
        </Stage>
    }
}
```

The framework integration layer:
- Creates/updates/destroys Konva-Rust nodes in response to reactive signal changes
- Maps framework event handlers to Konva event subscriptions
- Provides `NodeRef` for imperative access to underlying node handles
- Handles Stage mounting/unmounting to the DOM

## Crate Structure

```
konva-rust/
  crates/
    konva-core/       # Scene graph, Node, Container, transforms, events, serialization
    konva-shapes/     # All 17 shape implementations
    konva-filters/    # All 20 pixel filters
    konva-animation/  # AnimationLoop + Tween
    konva-renderer/   # Renderer trait + Canvas2D web-sys backend
    konva-leptos/     # Leptos component bindings (or konva-dioxus, etc.)
  examples/
    basic/            # Simple shapes demo
    drag-and-drop/    # Drag example
    transformer/      # Interactive resize/rotate
    filters/          # Filter showcase
    animation/        # Tween demos
    performance/      # Stress test (10k shapes)
```

## Implementation Order

1. **konva-core** — Node trait, Transform, scene graph tree, property system, dirty flags
2. **konva-renderer** — Canvas2D backend via web-sys, dual-canvas hit testing
3. **konva-shapes** — Rect, Circle, Ellipse, Line first, then Text, Path, Transformer last
4. **konva-core events** — DOM event binding, hit resolution, bubbling, drag state machine
5. **konva-animation** — AnimationLoop, then Tween with easings
6. **konva-filters** — Pixel manipulation on cached buffers
7. **konva-leptos** — Component macros, reactive prop binding, event mapping
8. **Transformer** — Last due to complexity (resize handles, multi-node, rotation snapping)

## Open Questions

- **Arena vs ECS**: slotmap-style arena is simpler and sufficient; full ECS (bevy_ecs) is overkill unless we want plugin extensibility
- **Text shaping**: rely on browser `measureText` via web-sys, or pull in a Rust text layout crate for native targets later?
- **GPU backend**: `wgpu` as a future renderer? Would enable native desktop targets but adds significant complexity
- **Multi-threading**: web workers for filter computation on large images?
- **Framework choice**: Leptos is the current leading candidate but Dioxus has a stronger cross-platform story
