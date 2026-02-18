//! In-board board switcher listing available boards with navigation.

use leptos::prelude::*;

use crate::components::board_card::BoardCard;
use crate::state::board::BoardState;

/// Compact board list for switching boards without leaving the workspace.
#[component]
pub fn MissionControl() -> impl IntoView {
    let board = expect_context::<RwSignal<BoardState>>();
    let boards = LocalResource::new(|| crate::net::api::fetch_boards());

    view! {
        <div class="mission-control">
            <div class="mission-control__inner">
                <Suspense fallback=move || view! { <p class="mission-control__loading">"Loading..."</p> }>
                    {move || {
                        let active_board = board.get().board_id;
                        boards
                            .get()
                            .map(|list| {
                                list.into_iter()
                                    .map(|item| {
                                        let is_active = active_board.as_deref() == Some(item.id.as_str());
                                        view! {
                                            <BoardCard id=item.id name=item.name active=is_active mini=true/>
                                        }
                                    })
                                    .collect::<Vec<_>>()
                            })
                    }}
                </Suspense>
            </div>
        </div>
    }
}
