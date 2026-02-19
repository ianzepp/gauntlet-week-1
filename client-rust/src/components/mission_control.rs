//! In-board board switcher listing available boards with navigation.
//!
//! SYSTEM CONTEXT
//! ==============
//! Supports board-to-board navigation without leaving the board route, which
//! relies on route-transition membership handling in `pages::board`.

use leptos::prelude::*;

use crate::app::FrameSender;
use crate::components::board_card::BoardCard;
use crate::net::types::{Frame, FrameStatus};
use crate::state::board::BoardState;
use crate::state::boards::BoardsState;

/// Compact board list for switching boards without leaving the workspace.
#[component]
pub fn MissionControl() -> impl IntoView {
    let board = expect_context::<RwSignal<BoardState>>();
    let boards = expect_context::<RwSignal<BoardsState>>();
    let sender = expect_context::<RwSignal<FrameSender>>();

    let requested_list = RwSignal::new(false);
    Effect::new(move || {
        if requested_list.get() {
            return;
        }
        if !matches!(board.get().connection_status, crate::state::board::ConnectionStatus::Connected) {
            return;
        }
        boards.update(|s| s.loading = true);
        let frame = Frame {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            ts: 0,
            board_id: None,
            from: None,
            syscall: "board:list".to_owned(),
            status: FrameStatus::Request,
            data: serde_json::json!({}),
        };
        let _ = sender.get_untracked().send(&frame);
        requested_list.set(true);
    });

    view! {
        <div class="mission-control">
            <div class="mission-control__inner">
                <Show
                    when=move || !boards.get().loading
                    fallback=move || view! { <p class="mission-control__loading">"Loading..."</p> }
                >
                    {move || {
                        let active_board = board.get().board_id;
                        boards
                            .get()
                            .items
                            .into_iter()
                            .map(|item| {
                                let is_active = active_board.as_deref() == Some(item.id.as_str());
                                view! {
                                    <BoardCard id=item.id name=item.name active=is_active mini=true/>
                                }
                            })
                            .collect::<Vec<_>>()
                    }}
                </Show>
            </div>
        </div>
    }
}
