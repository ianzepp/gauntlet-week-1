//! Property inspector for the currently selected board object.

use leptos::prelude::*;

use crate::app::FrameSender;
use crate::net::types::{BoardObject, Frame, FrameStatus};
use crate::state::board::BoardState;

/// Inspector panel showing properties of the selected object.
///
/// Reads from `BoardState.selection` and `BoardState.objects`. Editable fields
/// send `object:update` frames on change. Delete button sends `object:delete`.
#[component]
pub fn InspectorPanel() -> impl IntoView {
    let board = expect_context::<RwSignal<BoardState>>();
    let sender = expect_context::<RwSignal<FrameSender>>();

    let selected_object = move || {
        let state = board.get();
        let sel_id = state.selection.iter().next().cloned();
        sel_id.and_then(|id| state.objects.get(&id).cloned())
    };

    view! {
        <div class="inspector-panel">
            {move || {
                if let Some(obj) = selected_object() {
                    render_inspector(obj, board, sender).into_any()
                } else {
                    view! { <p class="inspector-panel__empty">"No object selected"</p> }.into_any()
                }
            }}
        </div>
    }
}

fn render_inspector(obj: BoardObject, board: RwSignal<BoardState>, sender: RwSignal<FrameSender>) -> impl IntoView {
    let obj_id = obj.id.clone();
    let board_id = obj.board_id.clone();

    let on_delete = {
        let obj_id = obj_id.clone();
        let board_id = board_id.clone();
        move |_| {
            let frame = Frame {
                id: uuid::Uuid::new_v4().to_string(),
                parent_id: None,
                ts: 0.0,
                board_id: Some(board_id.clone()),
                from: None,
                syscall: "object:delete".to_owned(),
                status: FrameStatus::Request,
                data: serde_json::json!({ "id": obj_id }),
            };
            sender.get().send(&frame);
            board.update(|b| {
                b.objects.remove(&obj_id);
                b.selection.remove(&obj_id);
            });
        }
    };

    view! {
        <div class="inspector-panel__card">
            <h3 class="inspector-panel__title">{obj.kind.clone()}</h3>

            <dl class="inspector-panel__fields">
                <dt>"ID"</dt>
                <dd class="inspector-panel__mono">{obj.id.clone()}</dd>
                <dt>"Position"</dt>
                <dd>{format!("({:.0}, {:.0})", obj.x, obj.y)}</dd>
                <dt>"Size"</dt>
                <dd>
                    {format!(
                        "{} \u{00D7} {}",
                        obj.width.map_or("—".to_owned(), |w| format!("{w:.0}")),
                        obj.height.map_or("—".to_owned(), |h| format!("{h:.0}")),
                    )}
                </dd>
                <dt>"Rotation"</dt>
                <dd>{format!("{:.0}\u{00B0}", obj.rotation)}</dd>
                <dt>"Z-index"</dt>
                <dd>{obj.z_index}</dd>
                <dt>"Version"</dt>
                <dd>{obj.version}</dd>
            </dl>

            <div class="inspector-panel__actions">
                <button class="btn btn--danger" on:click=on_delete>
                    "Delete"
                </button>
            </div>
        </div>
    }
}
