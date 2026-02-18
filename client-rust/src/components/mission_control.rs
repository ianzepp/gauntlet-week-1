//! In-board board switcher listing available boards with navigation.

use leptos::prelude::*;

use crate::net::api::BoardListItem;

/// Compact board list for switching between boards without leaving the workspace.
#[component]
pub fn MissionControl() -> impl IntoView {
    let boards = Resource::new(|| (), |()| async move { crate::net::api::fetch_boards().await });

    view! {
        <div class="mission-control">
            <h3 class="mission-control__title">"Boards"</h3>
            <Suspense fallback=move || view! { <p>"Loading..."</p> }>
                {move || {
                    boards
                        .get()
                        .map(|list| {
                            view! {
                                <ul class="mission-control__list">
                                    {list
                                        .into_iter()
                                        .map(board_item)
                                        .collect::<Vec<_>>()}
                                </ul>
                            }
                        })
                }}
            </Suspense>
        </div>
    }
}

fn board_item(b: BoardListItem) -> impl IntoView {
    let href = format!("/board/{}", b.id);
    view! {
        <li class="mission-control__item">
            <a href=href class="mission-control__link">
                {b.name}
            </a>
        </li>
    }
}
