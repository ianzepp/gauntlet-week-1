//! Top bar displaying board name, presence avatars, back button, and logout.

use std::collections::HashSet;

use leptos::prelude::*;
use leptos_router::hooks::use_location;

use crate::components::user_field_report::UserFieldReport;
use crate::state::auth::AuthState;
use crate::state::board::BoardState;
use crate::state::ui::UiState;

/// Top toolbar for the board page.
#[component]
pub fn Toolbar() -> impl IntoView {
    let auth = expect_context::<RwSignal<AuthState>>();
    let board = expect_context::<RwSignal<BoardState>>();
    let ui = expect_context::<RwSignal<UiState>>();
    let location = use_location();

    let board_name = move || {
        board
            .get()
            .board_name
            .unwrap_or_else(|| "Untitled".to_owned())
    };

    let active_report = RwSignal::new(None::<(String, i32)>);

    let all_users = move || {
        let mut seen = HashSet::new();
        let mut users = Vec::<(String, String, String)>::new();

        if let Some(user) = auth.get().user {
            seen.insert(user.id.clone());
            users.push((user.id, user.name, user.color));
        }

        for p in board.get().presence.values() {
            if seen.insert(p.user_id.clone()) {
                users.push((p.user_id.clone(), p.name.clone(), p.color.clone()));
            }
        }

        users
    };

    let on_logout = move |_| {
        #[cfg(feature = "hydrate")]
        {
            leptos::task::spawn_local(async move {
                crate::net::api::logout().await;
                auth.update(|a| a.user = None);
                if let Some(w) = web_sys::window() {
                    let _ = w.location().set_href("/login");
                }
            });
        }
    };

    let close_report = Callback::new(move |_| active_report.set(None));

    view! {
        <div class="toolbar">
            <Show when=move || location.pathname.get().starts_with("/board/")>
                <a href="/" class="toolbar__back" title="Back to dashboard">
                    "←"
                </a>
            </Show>

            <span class="toolbar__board-name">{board_name}</span>
            <span class="toolbar__divider"></span>

            <div class="toolbar__presence">
                {move || {
                    all_users()
                        .into_iter()
                        .map(|(id, name, color)| {
                            let chip_color = color.clone();
                            let dot_color = color.clone();
                            let chip_name = name.clone();
                            let display_name = name.clone();
                            let user_id = id.clone();

                            let on_chip_click = move |_ev: leptos::ev::MouseEvent| {
                                #[cfg(feature = "hydrate")]
                                {
                                    use wasm_bindgen::JsCast;

                                    let mut anchor_x = 120_i32;
                                    if let Some(target) = _ev.current_target()
                                        && let Ok(el) = target.dyn_into::<web_sys::HtmlElement>()
                                    {
                                        anchor_x = el.offset_left() + (el.offset_width() / 2);
                                    }
                                    active_report.set(Some((user_id.clone(), anchor_x)));
                                }

                                #[cfg(not(feature = "hydrate"))]
                                {
                                    active_report.set(Some((user_id.clone(), 120)));
                                }
                            };

                            view! {
                                <button
                                    class="toolbar__presence-chip"
                                    title=chip_name
                                    style:border-color=chip_color
                                    on:click=on_chip_click
                                >
                                    <span class="toolbar__presence-dot" style:background=dot_color></span>
                                    {display_name}
                                </button>
                            }
                        })
                        .collect::<Vec<_>>()
                }}
            </div>

            <span class="toolbar__spacer"></span>

            <button
                class="btn toolbar__dark-toggle"
                on:click=move |_| {
                    let current = ui.get().dark_mode;
                    let next = crate::util::dark_mode::toggle(current);
                    ui.update(|u| u.dark_mode = next);
                }
                title="Toggle dark mode"
            >
                {move || if ui.get().dark_mode { "☀" } else { "☾" }}
            </button>

            <button class="btn toolbar__logout" on:click=on_logout title="Logout">
                "Logout"
            </button>

            {move || {
                active_report.get().map(|(user_id, anchor_x)| {
                    view! {
                        <UserFieldReport
                            user_id
                            anchor_x
                            direction="down"
                            on_close=close_report
                        />
                    }
                })
            }}
        </div>
    }
}
