//! Collapsible right panel with icon rail for boards, records, chat, and AI.
//!
//! ARCHITECTURE
//! ============
//! Right-side tools are organized as tabbed subviews so chat/AI/records can
//! share one rail without overlapping canvas layout logic.

use leptos::prelude::*;
#[cfg(feature = "hydrate")]
use wasm_bindgen::JsCast;

use crate::components::ai_panel::AiPanel;
use crate::components::chat_panel::ChatPanel;
use crate::components::mission_control::MissionControl;
use crate::components::rewind_shelf::RewindShelf;
use crate::components::trace_inspector::TraceInspector;
use crate::components::user_list_panel::UserListPanel;
use crate::state::ui::{RightTab, UiState, ViewMode};

/// Collapsible right sidebar with icon rail and expandable content panel.
#[component]
pub fn RightPanel() -> impl IntoView {
    let ui = expect_context::<RwSignal<UiState>>();

    let expanded = move || ui.get().right_panel_expanded;
    let active_tab = move || ui.get().right_tab;
    let trace_mode = move || ui.get().view_mode == ViewMode::Trace;
    let dragging = RwSignal::new(false);
    let drag_start_x = RwSignal::new(0.0_f64);
    let drag_start_width = RwSignal::new(320.0_f64);
    let panel_width_style = move || format!("width: {:.0}px;", ui.get().right_panel_width);

    Effect::new(move || {
        let state = ui.get();
        if state.view_mode != ViewMode::Trace && state.right_tab == RightTab::Trace {
            ui.update(|u| {
                u.right_panel_expanded = false;
                u.right_tab = RightTab::Chat;
            });
        }
    });

    let toggle_tab = move |tab: RightTab| {
        ui.update(|u| {
            if u.right_panel_expanded && u.right_tab == tab {
                u.right_panel_expanded = false;
            } else {
                u.right_panel_expanded = true;
                u.right_tab = tab;
            }
        });
    };

    let toggle_expand = move |_| {
        ui.update(|u| u.right_panel_expanded = !u.right_panel_expanded);
    };

    let on_resize_pointer_down = move |ev: leptos::ev::PointerEvent| {
        dragging.set(true);
        drag_start_x.set(f64::from(ev.client_x()));
        drag_start_width.set(ui.get().right_panel_width);
        #[cfg(feature = "hydrate")]
        {
            if let Some(target) = ev
                .target()
                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            {
                let _ = target.set_pointer_capture(ev.pointer_id());
            }
        }
    };

    let on_resize_pointer_move = move |ev: leptos::ev::PointerEvent| {
        if !dragging.get() {
            return;
        }
        let delta = drag_start_x.get() - f64::from(ev.client_x());
        let next = (drag_start_width.get() + delta).clamp(260.0, 1100.0);
        ui.update(|u| u.right_panel_width = next);
    };

    let on_resize_pointer_up = move |_ev: leptos::ev::PointerEvent| {
        dragging.set(false);
    };

    view! {
        <div class="right-panel" on:pointermove=on_resize_pointer_move on:pointerup=on_resize_pointer_up on:pointercancel=on_resize_pointer_up>
            <Show when=expanded>
                <div class="right-panel__resize-handle-rail" on:pointerdown=on_resize_pointer_down></div>
            </Show>
            <div class="right-panel__rail">
                <button
                    class="right-panel__rail-button ui-tooltip ui-tooltip--left"
                    class:right-panel__rail-button--active=move || expanded() && active_tab() == RightTab::Boards
                    on:click=move |_| toggle_tab(RightTab::Boards)
                    title="Boards"
                    attr:data-tooltip="Boards"
                >
                    <svg viewBox="0 0 20 20" aria-hidden="true">
                        <rect x="3" y="3" width="6" height="6" />
                        <rect x="11" y="3" width="6" height="6" />
                        <rect x="3" y="11" width="6" height="6" />
                        <rect x="11" y="11" width="6" height="6" />
                    </svg>
                </button>

                <button
                    class="right-panel__rail-button ui-tooltip ui-tooltip--left"
                    class:right-panel__rail-button--active=move || expanded() && active_tab() == RightTab::Records
                    on:click=move |_| toggle_tab(RightTab::Records)
                    title="Field Records"
                    attr:data-tooltip="Field Records"
                >
                    <svg viewBox="0 0 20 20" aria-hidden="true">
                        <ellipse cx="10" cy="5" rx="6" ry="2.5" />
                        <path d="M4 5 V10" />
                        <path d="M16 5 V10" />
                        <ellipse cx="10" cy="10" rx="6" ry="2.5" />
                        <path d="M4 10 V15" />
                        <path d="M16 10 V15" />
                        <ellipse cx="10" cy="15" rx="6" ry="2.5" />
                    </svg>
                </button>

                <button
                    class="right-panel__rail-button ui-tooltip ui-tooltip--left"
                    class:right-panel__rail-button--active=move || expanded() && active_tab() == RightTab::Chat
                    on:click=move |_| toggle_tab(RightTab::Chat)
                    title="Chat"
                    attr:data-tooltip="Chat"
                >
                    <svg viewBox="0 0 20 20" aria-hidden="true">
                        <rect x="3" y="3" width="14" height="10" />
                        <path d="M7 13 L7 17 L11 13" />
                    </svg>
                </button>

                <button
                    class="right-panel__rail-button ui-tooltip ui-tooltip--left"
                    class:right-panel__rail-button--active=move || expanded() && active_tab() == RightTab::Users
                    on:click=move |_| toggle_tab(RightTab::Users)
                    title="Users"
                    attr:data-tooltip="Users"
                >
                    <svg viewBox="0 0 20 20" aria-hidden="true">
                        <circle cx="7" cy="7" r="2.5" />
                        <circle cx="13.5" cy="8" r="2" />
                        <path d="M2.5 16 C3.5 12.8 5.5 11.5 7.5 11.5 C9.5 11.5 11.6 12.8 12.5 16" />
                        <path d="M11 16 C11.7 13.8 13 12.8 14.5 12.8 C16 12.8 17.2 13.8 18 16" />
                    </svg>
                </button>

                <button
                    class="right-panel__rail-button ui-tooltip ui-tooltip--left"
                    class:right-panel__rail-button--active=move || expanded() && active_tab() == RightTab::Ai
                    on:click=move |_| toggle_tab(RightTab::Ai)
                    title="Field Notes"
                    attr:data-tooltip="Field Notes"
                >
                    <svg viewBox="0 0 20 20" aria-hidden="true">
                        <path d="M10 2 L12 7 L18 7 L13 10 L15 16 L10 12 L5 16 L7 10 L2 7 L8 7 Z" />
                    </svg>
                </button>

                <Show when=trace_mode>
                    <button
                        class="right-panel__rail-button ui-tooltip ui-tooltip--left"
                        class:right-panel__rail-button--active=move || expanded() && active_tab() == RightTab::Trace
                        on:click=move |_| toggle_tab(RightTab::Trace)
                        title="Trace Detail"
                        attr:data-tooltip="Trace Detail"
                    >
                        <svg viewBox="0 0 20 20" aria-hidden="true">
                            <path d="M3 4 H17" />
                            <path d="M3 9 H12" />
                            <path d="M3 14 H10" />
                            <path d="M14 12 L17 15 L14 18" />
                        </svg>
                    </button>
                </Show>

                <div class="right-panel__rail-spacer"></div>

                <button class="right-panel__toggle ui-tooltip ui-tooltip--left" on:click=toggle_expand title="Toggle panel" attr:data-tooltip="Toggle panel">
                    {move || if expanded() { "▶" } else { "◀" }}
                </button>
            </div>

            <Show when=expanded>
                <div class="right-panel__panel" style=panel_width_style>
                    <div class="right-panel__header">
                        <span class="right-panel__title">
                            {move || match active_tab() {
                                RightTab::Chat => "Chat",
                                RightTab::Users => "Connected Users",
                                RightTab::Ai => "Field Notes",
                                RightTab::Trace => "Trace Detail",
                                RightTab::Boards => "Boards",
                                RightTab::Records => "Field Records",
                            }}
                        </span>
                        <button class="right-panel__close" on:click=move |_| ui.update(|u| u.right_panel_expanded = false)>
                            "✕"
                        </button>
                    </div>

                    <div class="right-panel__content">
                        {move || match active_tab() {
                            RightTab::Chat => view! { <ChatPanel/> }.into_any(),
                            RightTab::Users => view! { <UserListPanel/> }.into_any(),
                            RightTab::Ai => view! { <AiPanel/> }.into_any(),
                            RightTab::Trace => view! { <TraceInspector/> }.into_any(),
                            RightTab::Boards => view! { <MissionControl/> }.into_any(),
                            RightTab::Records => view! { <RewindShelf/> }.into_any(),
                        }}
                    </div>
                </div>
            </Show>
        </div>
    }
}
