//! Reusable dial primitives for circular drag controls.
//!
//! SYSTEM CONTEXT
//! ==============
//! Encapsulates shared shell and center-button interaction wiring used by
//! compass/zoom and future board controls (size, rotation, stroke, color).

use leptos::prelude::*;

/// Shared dial shell with pointer lifecycle handlers.
pub fn dial_shell<PD, PM, PU, V>(
    class: &'static str,
    title: &'static str,
    node_ref: NodeRef<leptos::html::Div>,
    on_pointer_down: PD,
    on_pointer_move: PM,
    on_pointer_up: PU,
    content: V,
) -> impl IntoView
where
    PD: Fn(leptos::ev::PointerEvent) + Clone + 'static,
    PM: Fn(leptos::ev::PointerEvent) + Clone + 'static,
    PU: Fn(leptos::ev::PointerEvent) + Clone + 'static,
    V: IntoView + 'static,
{
    view! {
        <div
            class=class
            node_ref=node_ref
            title=title
            on:pointerdown=on_pointer_down
            on:pointermove=on_pointer_move
            on:pointerup=on_pointer_up.clone()
            on:pointercancel=on_pointer_up.clone()
            on:pointerleave=on_pointer_up
        >
            {content}
        </div>
    }
}

/// Shared center readout/reset button for dials.
pub fn dial_center_button<PD, C, D, V>(
    class: &'static str,
    title: &'static str,
    on_pointer_down: PD,
    on_click: C,
    on_dblclick: D,
    content: V,
) -> impl IntoView
where
    PD: Fn(leptos::ev::PointerEvent) + Clone + 'static,
    C: Fn(leptos::ev::MouseEvent) + Clone + 'static,
    D: Fn(leptos::ev::MouseEvent) + Clone + 'static,
    V: IntoView + 'static,
{
    view! {
        <button
            class=class
            title=title
            on:pointerdown=on_pointer_down
            on:click=on_click
            on:dblclick=on_dblclick
        >
            {content}
        </button>
    }
}

