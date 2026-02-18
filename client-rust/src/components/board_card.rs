//! Reusable card component for board list items on the dashboard.

use leptos::prelude::*;

/// A clickable card representing a board.
#[component]
pub fn BoardCard(
    id: String,
    name: String,
    #[prop(optional)] active: bool,
    #[prop(optional)] mini: bool,
) -> impl IntoView {
    let href = format!("/board/{id}");

    view! {
        <a
            class="board-card"
            class:board-card--active=active
            class:board-card--mini=mini
            href=href
        >
            <span class="board-card__name">{name}</span>
            <span class="board-card__id">{id}</span>
            <span class="board-card__preview"></span>
        </a>
    }
}
