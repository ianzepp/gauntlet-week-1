//! Connected users panel for the right rail.
//!
//! SYSTEM CONTEXT
//! ==============
//! Renders board presence state populated by websocket `board:users:list`,
//! `board:join`, `board:part`, and `cursor:*` frames.

use leptos::prelude::*;

use crate::state::board::BoardState;

/// Tab content showing currently connected users on this board.
#[component]
pub fn UserListPanel() -> impl IntoView {
    let board = expect_context::<RwSignal<BoardState>>();

    let rows = move || {
        let state = board.get();
        let self_client_id = state.self_client_id.clone();
        let followed_client_id = state.follow_client_id.clone();
        let mut items = state
            .presence
            .values()
            .cloned()
            .map(|p| {
                let is_self = self_client_id.as_deref() == Some(p.client_id.as_str());
                let is_followed = followed_client_id.as_deref() == Some(p.client_id.as_str());
                (p, is_self, is_followed)
            })
            .collect::<Vec<_>>();
        items.sort_by(|(a, _, _), (b, _, _)| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        items
    };

    view! {
        <div class="user-list-panel">
            <div class="user-list-panel__summary">
                {move || format!("{} connected", rows().len())}
            </div>

            <Show
                when=move || !rows().is_empty()
                fallback=move || view! { <div class="user-list-panel__empty">"No active users."</div> }
            >
                <div class="user-list-panel__table-wrap">
                    <table class="user-list-panel__table">
                        <thead>
                            <tr>
                                <th>"User"</th>
                                <th>"Status"</th>
                                <th>"Client"</th>
                                <th>"Action"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {move || {
                                rows()
                                    .into_iter()
                                    .map(|(presence, is_self, is_followed)| {
                                        let dot_style = format!("background:{};", presence.color);
                                        let status = if is_followed {
                                            "Following"
                                        } else if is_self {
                                            "You"
                                        } else {
                                            "Connected"
                                        };
                                        let short_client = shorten_client_id(&presence.client_id);
                                        let follow_client_id = presence.client_id.clone();
                                        let on_follow = Callback::new({
                                            let board = board;
                                            move |_| {
                                                board.update(|b| {
                                                    let next_follow = match b.follow_client_id.as_deref() {
                                                        Some(current) if current == follow_client_id.as_str() => None,
                                                        _ => Some(follow_client_id.clone()),
                                                    };
                                                    b.follow_client_id = next_follow;
                                                    b.jump_to_client_id = None;
                                                });
                                            }
                                        });
                                        let action_title = if is_followed { "Stop following camera" } else { "Follow camera" };
                                        view! {
                                            <tr>
                                                <td>
                                                    <div class="user-list-panel__user">
                                                        <span class="user-list-panel__dot" style=dot_style></span>
                                                        <span>{presence.name}</span>
                                                    </div>
                                                </td>
                                                <td>{status}</td>
                                                <td class="user-list-panel__mono" title=presence.client_id.clone()>{short_client}</td>
                                                <td>
                                                    <Show
                                                        when=move || !is_self
                                                        fallback=move || {
                                                            view! { <span class="user-list-panel__na">"—"</span> }
                                                        }
                                                    >
                                                        <button
                                                            class="user-list-panel__action-btn"
                                                            class:user-list-panel__action-btn--active=is_followed
                                                            on:click=move |_| on_follow.run(())
                                                            title=action_title
                                                        >
                                                            {if is_followed {
                                                                view! {
                                                                    <svg viewBox="0 0 20 20" aria-hidden="true">
                                                                        <rect x="4" y="3" width="12" height="14" rx="2" ry="2"></rect>
                                                                        <path d="M7 9 V7.5 C7 5.57 8.57 4 10.5 4 C12.43 4 14 5.57 14 7.5 V9"></path>
                                                                        <circle cx="10" cy="12" r="1.2"></circle>
                                                                    </svg>
                                                                }
                                                                    .into_any()
                                                            } else {
                                                                view! {
                                                                    <svg viewBox="0 0 20 20" aria-hidden="true">
                                                                        <rect x="4" y="9" width="12" height="8" rx="2" ry="2"></rect>
                                                                        <path d="M7 9 V7.5 C7 5.57 8.57 4 10.5 4 C12.43 4 14 5.57 14 7.5"></path>
                                                                        <circle cx="10" cy="13" r="1.2"></circle>
                                                                    </svg>
                                                                }
                                                                    .into_any()
                                                            }}
                                                        </button>
                                                    </Show>
                                                </td>
                                            </tr>
                                        }
                                    })
                                    .collect_view()
                            }}
                        </tbody>
                    </table>
                </div>
            </Show>
        </div>
    }
}

fn shorten_client_id(id: &str) -> String {
    if id.len() <= 8 {
        return id.to_owned();
    }
    format!("{}...", &id[..8])
}
