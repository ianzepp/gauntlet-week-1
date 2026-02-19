//! Savepoint list for the right-side "Field Records" panel.

use leptos::prelude::*;

use crate::app::FrameSender;
use crate::net::types::{Frame, FrameStatus};
use crate::state::board::BoardState;

#[component]
pub fn RewindShelf() -> impl IntoView {
    let board = expect_context::<RwSignal<BoardState>>();
    let sender = expect_context::<RwSignal<FrameSender>>();
    let ordered = move || board.get().savepoints.clone();

    let on_create = move |_| {
        let Some(board_id) = board.get_untracked().board_id else {
            return;
        };
        let frame = Frame {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            ts: 0,
            board_id: Some(board_id),
            from: None,
            syscall: "board:savepoint:create".to_owned(),
            status: FrameStatus::Request,
            data: serde_json::json!({
                "label": "Manual savepoint"
            }),
        };
        let _ = sender.get_untracked().send(&frame);
    };

    view! {
        <div class="rewind-shelf">
            <div class="rewind-shelf__toolbar">
                <button class="rewind-shelf__create" on:click=on_create>
                    "Drop Savepoint"
                </button>
            </div>
            <div class="rewind-shelf__records">
                {move || {
                    let rows = ordered();
                    if rows.is_empty() {
                        return view! {
                            <div class="rewind-shelf__empty">
                                "No records yet. Savepoints are auto-captured on create/delete."
                            </div>
                        }
                        .into_any();
                    }
                    rows.into_iter()
                        .map(|sp| {
                            let title = sp.label.clone().unwrap_or_else(|| {
                                if sp.is_auto {
                                    "Auto savepoint".to_owned()
                                } else {
                                    "Savepoint".to_owned()
                                }
                            });
                            let meta = format!("{} · seq {}", sp.reason, sp.seq);
                            view! {
                                <div class="rewind-record rewind-record--stack">
                                    <span class="rewind-record__title">{title}</span>
                                    <span class="rewind-record__meta">{meta}</span>
                                </div>
                            }
                        })
                        .collect_view()
                        .into_any()
                }}
            </div>
        </div>
    }
}
