//! Shared frame emission helpers.

#[cfg(test)]
#[path = "frame_emit_test.rs"]
mod frame_emit_test;

use leptos::prelude::{GetUntracked, RwSignal};

use crate::app::FrameSender;
use crate::net::types::{Frame, FrameStatus};

fn object_update_props_frame(board_id: &str, object_id: &str, version: i64, props: &serde_json::Value) -> Frame {
    Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: Some(board_id.to_owned()),
        from: None,
        syscall: "object:update".to_owned(),
        status: FrameStatus::Request,
        data: serde_json::json!({
            "id": object_id,
            "version": version,
            "props": props,
        }),
    }
}

fn object_update_rotation_frame(board_id: &str, object_id: &str, version: i64, rotation: f64) -> Frame {
    Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id: Some(board_id.to_owned()),
        from: None,
        syscall: "object:update".to_owned(),
        status: FrameStatus::Request,
        data: serde_json::json!({
            "id": object_id,
            "version": version,
            "rotation": rotation,
        }),
    }
}

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
