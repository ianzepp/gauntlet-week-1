//! Bridge component between the Leptos UI and the imperative `canvas::Engine`.
//!
//! Mounts a `<canvas>` element. The actual engine integration is deferred
//! to Phase 9 — for now this renders a placeholder canvas element.

use leptos::prelude::*;

/// Canvas host — placeholder `<canvas>` element for the whiteboard engine.
///
/// Phase 9 will add `canvas` crate dependency, create the `Engine`, wire
/// pointer/keyboard events, and synchronize state with Leptos signals.
#[component]
pub fn CanvasHost() -> impl IntoView {
    view! {
        <canvas class="canvas-host" width="800" height="600">
            "Your browser does not support canvas."
        </canvas>
    }
}
