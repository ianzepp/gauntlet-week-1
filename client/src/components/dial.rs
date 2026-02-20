//! Reusable dial primitives for circular drag controls.
//!
//! SYSTEM CONTEXT
//! ==============
//! Encapsulates shared shell and center-button interaction wiring used by
//! compass/zoom and future board controls (size, rotation, stroke, color).

use leptos::prelude::*;

/// Shared dial shell with pointer lifecycle handlers.
pub fn dial_shell<PD, PM, PU, V>(
    class: String,
    title: String,
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
    class: String,
    title: String,
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

/// Shared compass dial with cardinal snap buttons and center readout.
#[component]
pub fn CompassDial<PD, PM, PU, N, E, S, W, RPD, RC, RD>(
    #[prop(into)] class: String,
    #[prop(into)] disabled_class: String,
    #[prop(into)] title: String,
    #[prop(into)] readout_title: String,
    #[prop(into)] knob_class: String,
    #[prop(optional)] node_ref: NodeRef<leptos::html::Div>,
    disabled: Signal<bool>,
    readout: Signal<String>,
    knob_style: Signal<String>,
    on_pointer_down: PD,
    on_pointer_move: PM,
    on_pointer_up: PU,
    on_snap_n: N,
    on_snap_e: E,
    on_snap_s: S,
    on_snap_w: W,
    on_readout_pointer_down: RPD,
    on_readout_click: RC,
    on_readout_dblclick: RD,
) -> impl IntoView
where
    PD: Fn(leptos::ev::PointerEvent) + Clone + 'static,
    PM: Fn(leptos::ev::PointerEvent) + Clone + 'static,
    PU: Fn(leptos::ev::PointerEvent) + Clone + 'static,
    N: Fn(leptos::ev::MouseEvent) + Clone + 'static,
    E: Fn(leptos::ev::MouseEvent) + Clone + 'static,
    S: Fn(leptos::ev::MouseEvent) + Clone + 'static,
    W: Fn(leptos::ev::MouseEvent) + Clone + 'static,
    RPD: Fn(leptos::ev::PointerEvent) + Clone + 'static,
    RC: Fn(leptos::ev::MouseEvent) + Clone + 'static,
    RD: Fn(leptos::ev::MouseEvent) + Clone + 'static,
{
    let root_class = move || {
        if disabled.get() {
            format!("{class} {disabled_class}")
        } else {
            class.clone()
        }
    };
    let on_snap_pointer_down = move |ev: leptos::ev::PointerEvent| {
        ev.stop_propagation();
    };

    view! {
        <div
            class=root_class
            node_ref=node_ref
            title=title
            on:pointerdown=on_pointer_down
            on:pointermove=on_pointer_move
            on:pointerup=on_pointer_up.clone()
            on:pointercancel=on_pointer_up.clone()
            on:pointerleave=on_pointer_up
        >
            <button class="canvas-compass__snap canvas-compass__snap--n" on:pointerdown=on_snap_pointer_down on:click=on_snap_n disabled=move || disabled.get()>
                "N"
            </button>
            <button class="canvas-compass__snap canvas-compass__snap--e" on:pointerdown=on_snap_pointer_down on:click=on_snap_e disabled=move || disabled.get()>
                "E"
            </button>
            <button class="canvas-compass__snap canvas-compass__snap--s" on:pointerdown=on_snap_pointer_down on:click=on_snap_s disabled=move || disabled.get()>
                "S"
            </button>
            <button class="canvas-compass__snap canvas-compass__snap--w" on:pointerdown=on_snap_pointer_down on:click=on_snap_w disabled=move || disabled.get()>
                "W"
            </button>
            {dial_center_button(
                "canvas-compass__reset".to_owned(),
                readout_title.clone(),
                on_readout_pointer_down,
                on_readout_click,
                on_readout_dblclick,
                view! { {move || readout.get()} },
            )}
            <div class="canvas-compass__knob-track" style=move || knob_style.get()>
                <div class=knob_class></div>
            </div>
        </div>
    }
}

/// Shared zoom dial with marker, ticks, and center readout.
#[component]
pub fn ZoomDial<PD, PM, PU, RPD, RC, RD>(
    #[prop(into)] class: String,
    #[prop(into)] disabled_class: String,
    #[prop(into)] title: String,
    #[prop(into)] readout_title: String,
    #[prop(into)] knob_class: String,
    #[prop(optional)] node_ref: NodeRef<leptos::html::Div>,
    disabled: Signal<bool>,
    readout: Signal<String>,
    knob_style: Signal<String>,
    on_pointer_down: PD,
    on_pointer_move: PM,
    on_pointer_up: PU,
    on_readout_pointer_down: RPD,
    on_readout_click: RC,
    on_readout_dblclick: RD,
) -> impl IntoView
where
    PD: Fn(leptos::ev::PointerEvent) + Clone + 'static,
    PM: Fn(leptos::ev::PointerEvent) + Clone + 'static,
    PU: Fn(leptos::ev::PointerEvent) + Clone + 'static,
    RPD: Fn(leptos::ev::PointerEvent) + Clone + 'static,
    RC: Fn(leptos::ev::MouseEvent) + Clone + 'static,
    RD: Fn(leptos::ev::MouseEvent) + Clone + 'static,
{
    let root_class = move || {
        if disabled.get() {
            format!("{class} {disabled_class}")
        } else {
            class.clone()
        }
    };
    let on_marker_pointer_down = move |ev: leptos::ev::PointerEvent| {
        ev.stop_propagation();
    };

    view! {
        <div
            class=root_class
            node_ref=node_ref
            title=title
            on:pointerdown=on_pointer_down
            on:pointermove=on_pointer_move
            on:pointerup=on_pointer_up.clone()
            on:pointercancel=on_pointer_up.clone()
            on:pointerleave=on_pointer_up
        >
            <button class="canvas-zoom-wheel__marker" title="100%" on:pointerdown=on_marker_pointer_down>
                "1"
            </button>
            <span class="canvas-zoom-wheel__tick canvas-zoom-wheel__tick--n"></span>
            <span class="canvas-zoom-wheel__tick canvas-zoom-wheel__tick--ne"></span>
            <span class="canvas-zoom-wheel__tick canvas-zoom-wheel__tick--e"></span>
            <span class="canvas-zoom-wheel__tick canvas-zoom-wheel__tick--se"></span>
            <span class="canvas-zoom-wheel__tick canvas-zoom-wheel__tick--sw"></span>
            <span class="canvas-zoom-wheel__tick canvas-zoom-wheel__tick--w"></span>
            <span class="canvas-zoom-wheel__tick canvas-zoom-wheel__tick--nw"></span>
            {dial_center_button(
                "canvas-zoom-wheel__readout".to_owned(),
                readout_title.clone(),
                on_readout_pointer_down,
                on_readout_click,
                on_readout_dblclick,
                view! { {move || readout.get()} },
            )}
            <div class="canvas-zoom-wheel__knob-track" style=move || knob_style.get()>
                <div class=knob_class></div>
            </div>
        </div>
    }
}

/// Shared color dial with center color picker and lightness readout.
#[component]
pub fn ColorDial<PD, PM, PU, CPD, CI>(
    #[prop(into)] class: String,
    #[prop(into)] disabled_class: String,
    #[prop(into)] title: String,
    #[prop(into)] readout_title: String,
    #[prop(into)] knob_class: String,
    #[prop(optional)] node_ref: NodeRef<leptos::html::Div>,
    disabled: Signal<bool>,
    readout: Signal<String>,
    knob_style: Signal<String>,
    color_value: Signal<String>,
    on_pointer_down: PD,
    on_pointer_move: PM,
    on_pointer_up: PU,
    on_center_pointer_down: CPD,
    on_color_input: CI,
) -> impl IntoView
where
    PD: Fn(leptos::ev::PointerEvent) + Clone + 'static,
    PM: Fn(leptos::ev::PointerEvent) + Clone + 'static,
    PU: Fn(leptos::ev::PointerEvent) + Clone + 'static,
    CPD: Fn(leptos::ev::PointerEvent) + Clone + 'static,
    CI: Fn(leptos::ev::Event) + Clone + 'static,
{
    let root_class = move || {
        if disabled.get() {
            format!("{class} {disabled_class}")
        } else {
            class.clone()
        }
    };

    view! {
        <div
            class=root_class
            node_ref=node_ref
            title=title
            on:pointerdown=on_pointer_down
            on:pointermove=on_pointer_move
            on:pointerup=on_pointer_up.clone()
            on:pointercancel=on_pointer_up.clone()
            on:pointerleave=on_pointer_up
        >
            <span class="canvas-color-dial__tick canvas-color-dial__tick--n"></span>
            <span class="canvas-color-dial__tick canvas-color-dial__tick--ne"></span>
            <span class="canvas-color-dial__tick canvas-color-dial__tick--e"></span>
            <span class="canvas-color-dial__tick canvas-color-dial__tick--se"></span>
            <span class="canvas-color-dial__tick canvas-color-dial__tick--s"></span>
            <span class="canvas-color-dial__tick canvas-color-dial__tick--sw"></span>
            <span class="canvas-color-dial__tick canvas-color-dial__tick--w"></span>
            <span class="canvas-color-dial__tick canvas-color-dial__tick--nw"></span>
            <div
                class="canvas-color-dial__readout"
                title=readout_title
                on:pointerdown=on_center_pointer_down
            >
                <span class="canvas-color-dial__value">{move || readout.get()}</span>
                <input
                    class="canvas-color-dial__picker"
                    type="color"
                    prop:value=move || color_value.get()
                    on:input=on_color_input
                    disabled=move || disabled.get()
                />
            </div>
            <div class="canvas-color-dial__knob-track" style=move || knob_style.get()>
                <div class=knob_class></div>
            </div>
        </div>
    }
}
