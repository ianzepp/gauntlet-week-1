//! Semi-transparent overlay label displayed on the canvas area.

use leptos::prelude::*;

use crate::state::board::BoardState;

/// Board stamp — semi-transparent overlay showing board name and stats.
#[component]
pub fn BoardStamp() -> impl IntoView {
    let board = expect_context::<RwSignal<BoardState>>();

    let board_name = move || {
        board
            .get()
            .board_name
            .unwrap_or_else(|| "Untitled".to_owned())
    };
    let object_count = move || board.get().objects.len();
    let users = move || {
        let state = board.get();
        let self_client_id = state.self_client_id.unwrap_or_default();
        let mut rows = state.presence.values().cloned().collect::<Vec<_>>();
        rows.sort_by(|a, b| match (a.client_id == self_client_id, b.client_id == self_client_id) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a
                .name
                .cmp(&b.name)
                .then_with(|| a.client_id.cmp(&b.client_id)),
        });
        rows
    };

    view! {
        <div class="board-stamp">
            <span class="board-stamp__name">{board_name}</span>
            <span class="board-stamp__count">{move || format!("{} objects", object_count())}</span>
            <div class="board-stamp__users">
                {move || {
                    users()
                        .into_iter()
                        .map(|p| {
                            let client_id = p.client_id.clone();
                            let is_self = board.get().self_client_id.as_deref() == Some(client_id.as_str());
                            let follow_client_for_class = client_id.clone();
                            let follow_client_for_title = client_id.clone();
                            view! {
                                <div class="board-stamp__user-row">
                                    <span class="board-stamp__user-color" style:background=p.color.clone()></span>
                                    <button
                                        class="board-stamp__user-name"
                                        title=p.client_id.clone()
                                        on:click={
                                            let client_id = client_id.clone();
                                            move |_| {
                                                board.update(|b| b.jump_to_client_id = Some(client_id.clone()));
                                            }
                                        }
                                    >
                                        {p.name}
                                    </button>
                                    <span class="board-stamp__user-actions">
                                        <button
                                            class="board-stamp__follow-btn"
                                            class:board-stamp__follow-btn--active=move || board.get().follow_client_id.as_deref() == Some(follow_client_for_class.as_str())
                                            title=move || if is_self {
                                                "Cannot follow your own camera"
                                            } else if board.get().follow_client_id.as_deref() == Some(follow_client_for_title.as_str()) {
                                                "Disable follow camera"
                                            } else {
                                                "Follow camera"
                                            }
                                            disabled=is_self
                                            on:click={
                                                let client_id = client_id.clone();
                                                move |_| {
                                                    board.update(|b| {
                                                        if b.follow_client_id.as_deref() == Some(client_id.as_str()) {
                                                            b.follow_client_id = None;
                                                            if b.jump_to_client_id.as_deref() == Some(client_id.as_str()) {
                                                                b.jump_to_client_id = None;
                                                            }
                                                        } else {
                                                            b.follow_client_id = Some(client_id.clone());
                                                            b.jump_to_client_id = Some(client_id.clone());
                                                        }
                                                    });
                                                }
                                            }
                                        >
                                            <svg viewBox="0 0 20 20" aria-hidden="true">
                                                <circle cx="10" cy="10" r="5.5"></circle>
                                                <circle cx="10" cy="10" r="1.5"></circle>
                                            </svg>
                                        </button>
                                    </span>
                                </div>
                            }
                        })
                        .collect::<Vec<_>>()
                }}
            </div>
        </div>
    }
}
