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

    view! {
        <div class="board-stamp">
            <span class="board-stamp__name">{board_name}</span>
            <span class="board-stamp__count">{move || format!("{} objects", object_count())}</span>
        </div>
    }
}
