//! Shared frame emission helpers.
//!
//! Every mutation the client sends to the server is a `"object:update"` frame, but the payload
//! differs depending on what changed. Three narrow variants keep payload size small and make it
//! easy for the server to partially apply changes without touching unrelated fields:
//!
//! - **props-only** — for visual property changes (color, border, text style).
//! - **rotation-only** — for rotation changes; omits position/size to avoid stomping concurrent moves.
//! - **geometry** — for position/size changes; includes props because scale metadata must travel together.

#[cfg(test)]
#[path = "frame_emit_test.rs"]
mod frame_emit_test;

use leptos::prelude::{GetUntracked, RwSignal};

use crate::app::FrameSender;
use crate::net::types::{Frame, FrameStatus};

/// Build a props-only update frame, carrying the full props blob but no geometry fields.
fn object_update_props_frame(board_id: &str, object_id: &str, version: i64, props: &serde_json::Value) -> Frame {
    Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: Some(board_id.to_owned()),
        from: None,
        syscall: "object:update".to_owned(),
        status: FrameStatus::Request,
        trace: None,
        data: serde_json::json!({
            "id": object_id,
            "version": version,
            "props": props,
        }),
    }
}

/// Build a rotation-only update frame, carrying only the `rotation` field.
///
/// Deliberately excludes x/y/width/height to avoid overwriting concurrent geometry changes
/// from other clients or the AI during a rotate-then-commit sequence.
fn object_update_rotation_frame(board_id: &str, object_id: &str, version: i64, rotation: f64) -> Frame {
    Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: Some(board_id.to_owned()),
        from: None,
        syscall: "object:update".to_owned(),
        status: FrameStatus::Request,
        trace: None,
        data: serde_json::json!({
            "id": object_id,
            "version": version,
            "rotation": rotation,
        }),
    }
}

/// Build a geometry update frame carrying position, size, and props.
///
/// Props are included alongside geometry because scale metadata (`"baseWidth"`, `"baseHeight"`,
/// `"scale"`) must always stay consistent with the actual `width`/`height` values. Sending them
/// together prevents a race where geometry and scale props diverge.
fn object_update_geometry_frame(
    board_id: &str,
    object_id: &str,
    version: i64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    props: &serde_json::Value,
) -> Frame {
    Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: Some(board_id.to_owned()),
        from: None,
        syscall: "object:update".to_owned(),
        status: FrameStatus::Request,
        trace: None,
        data: serde_json::json!({
            "id": object_id,
            "version": version,
            "x": x,
            "y": y,
            "width": width,
            "height": height,
            "props": props,
        }),
    }
}

/// Emit a props-only update for a single object.
///
/// Use this after color, border, or text-style changes — anything that mutates `props` without
/// changing position or size.
pub fn send_object_update_props(
    sender: RwSignal<FrameSender>,
    board_id: &str,
    object_id: &str,
    version: i64,
    props: &serde_json::Value,
) {
    let frame = object_update_props_frame(board_id, object_id, version, props);
    let _ = sender.get_untracked().send(&frame);
}

/// Emit a rotation-only update for a single object.
///
/// Use this after the rotation dial is released — keeps the frame payload minimal and avoids
/// conflicting with concurrent position updates.
pub fn send_object_update_rotation(
    sender: RwSignal<FrameSender>,
    board_id: &str,
    object_id: &str,
    version: i64,
    rotation: f64,
) {
    let frame = object_update_rotation_frame(board_id, object_id, version, rotation);
    let _ = sender.get_untracked().send(&frame);
}

/// Emit a geometry update (position, size, and props) for a single object.
///
/// Use this after a scale drag or any move/resize operation that must keep scale props in sync
/// with the new dimensions.
pub fn send_object_update_geometry(
    sender: RwSignal<FrameSender>,
    board_id: &str,
    object_id: &str,
    version: i64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    props: &serde_json::Value,
) {
    let frame = object_update_geometry_frame(board_id, object_id, version, x, y, width, height, props);
    let _ = sender.get_untracked().send(&frame);
}
