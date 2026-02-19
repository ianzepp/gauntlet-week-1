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
        let mut rows = board.get().presence.values().cloned().collect::<Vec<_>>();
        rows.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then_with(|| a.client_id.cmp(&b.client_id))
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
                            view! {
                                <div class="board-stamp__user-row">
                                    <span class="board-stamp__user-color" style:background=p.color.clone()></span>
                                    <span class="board-stamp__user-name" title=p.client_id.clone()>{p.name}</span>
                                    <span class="board-stamp__user-actions"></span>
                                </div>
                            }
                        })
                        .collect::<Vec<_>>()
                }}
            </div>
        </div>
    }
}
