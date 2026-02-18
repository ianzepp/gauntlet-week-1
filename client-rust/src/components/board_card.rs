//! Reusable card component for board list items on the dashboard.

use leptos::prelude::*;

/// A clickable card representing a board in the dashboard list.
#[component]
pub fn BoardCard(id: String, name: String) -> impl IntoView {
    let href = format!("/board/{id}");

    view! {
        <a class="board-card" href=href>
            <span class="board-card__name">{name}</span>
        </a>
    }
}
