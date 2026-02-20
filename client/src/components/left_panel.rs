//! Collapsible left panel containing a tool rail and inspector panel.
//!
//! ARCHITECTURE
//! ============
//! Keeps creation tools and selection inspector co-located so editing controls
//! remain spatially consistent with canvas interactions.

use leptos::prelude::*;
#[cfg(feature = "hydrate")]
use wasm_bindgen::JsCast;

use crate::components::inspector_panel::InspectorPanel;
use crate::components::tool_rail::ToolRail;
use crate::state::ui::UiState;

const INSPECTOR_ENABLED: bool = false;

/// Left sidebar with a fixed tool rail and expandable inspector panel.
#[component]
pub fn LeftPanel() -> impl IntoView {
    let ui = expect_context::<RwSignal<UiState>>();

    let expanded = move || ui.get().left_panel_expanded;
    let dragging = RwSignal::new(false);
    let drag_start_x = RwSignal::new(0.0_f64);
    let drag_start_width = RwSignal::new(160.0_f64);
    let panel_width_style = move || format!("width: {:.0}px;", ui.get().left_panel_width);

    let toggle_expand = move |_| {
        ui.update(|u| u.left_panel_expanded = !u.left_panel_expanded);
    };

    let on_resize_pointer_down = move |ev: leptos::ev::PointerEvent| {
        dragging.set(true);
        drag_start_x.set(f64::from(ev.client_x()));
        drag_start_width.set(ui.get().left_panel_width);
        #[cfg(feature = "hydrate")]
        {
            if let Some(target) = ev.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
                let _ = target.set_pointer_capture(ev.pointer_id());
            }
        }
    };

    let on_resize_pointer_move = move |ev: leptos::ev::PointerEvent| {
        if !dragging.get() {
            return;
        }
        let delta = f64::from(ev.client_x()) - drag_start_x.get();
        let next = (drag_start_width.get() + delta).clamp(120.0, 900.0);
        ui.update(|u| u.left_panel_width = next);
    };

    let on_resize_pointer_up = move |_ev: leptos::ev::PointerEvent| {
        dragging.set(false);
    };

    view! {
        <div class="left-panel" on:pointermove=on_resize_pointer_move on:pointerup=on_resize_pointer_up on:pointercancel=on_resize_pointer_up>
            <Show when=move || INSPECTOR_ENABLED && expanded()>
                <div class="left-panel__panel" style=panel_width_style>
                    <div class="left-panel__header">
                        <span class="left-panel__title">"Inspector"</span>
                        <button class="left-panel__close" on:click=toggle_expand>
                            "✕"
                        </button>
                    </div>
                    <div class="left-panel__content">
                        <InspectorPanel/>
                    </div>
                </div>
            </Show>
            <Show when=move || INSPECTOR_ENABLED && expanded()>
                <div class="left-panel__resize-handle-rail" on:pointerdown=on_resize_pointer_down></div>
            </Show>

            <ToolRail/>
        </div>
    }
}
