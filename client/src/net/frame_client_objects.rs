//! Object and presence frame handlers extracted from `frame_client`.

#[cfg(test)]
#[path = "frame_client_objects_test.rs"]
mod frame_client_objects_test;

#[cfg(any(test, feature = "hydrate"))]
use crate::net::types::Frame;
#[cfg(any(test, feature = "hydrate"))]
use crate::state::board::BoardState;
#[cfg(feature = "hydrate")]
use std::{cell::RefCell, collections::HashMap};

#[cfg(feature = "hydrate")]
thread_local! {
    static LAST_CURSOR_APPLY_MS: RefCell<HashMap<String, f64>> = RefCell::new(HashMap::new());
}

#[cfg(any(test, feature = "hydrate"))]
pub(super) fn is_object_related_syscall(syscall: &str) -> bool {
    matches!(
        syscall,
        "object:create"
            | "object:update"
            | "object:delete"
            | "object:drag"
            | "object:drag:end"
            | "cursor:moved"
            | "cursor:clear"
    )
}

#[cfg(feature = "hydrate")]
pub(super) fn handle_object_frame(frame: &Frame, board: leptos::prelude::RwSignal<BoardState>) -> bool {
    use leptos::prelude::Update;

    if frame.syscall == "cursor:moved" && !should_apply_cursor_frame(frame) {
        return true;
    }
    if frame.syscall == "cursor:clear" {
        clear_cursor_gate(frame);
    }

    board.update(|b| {
        apply_object_frame(frame, b);
    });
    is_object_related_syscall(&frame.syscall)
}

#[cfg(feature = "hydrate")]
fn should_apply_cursor_frame(frame: &Frame) -> bool {
    // Strong coalescing to prevent seconds-long replay under browser-specific load.
    const CURSOR_MIN_APPLY_INTERVAL_MS: f64 = 33.0;
    let Some(client_id) = frame
        .data
        .get("client_id")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
    else {
        return true;
    };

    let now = js_sys::Date::now();
    LAST_CURSOR_APPLY_MS.with(|state| {
        let mut state = state.borrow_mut();
        if let Some(last) = state.get(&client_id)
            && now - *last < CURSOR_MIN_APPLY_INTERVAL_MS
        {
            return false;
        }
        state.insert(client_id, now);
        true
    })
}

#[cfg(feature = "hydrate")]
fn clear_cursor_gate(frame: &Frame) {
    let Some(client_id) = frame
        .data
        .get("client_id")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
    else {
        return;
    };
    LAST_CURSOR_APPLY_MS.with(|state| {
        state.borrow_mut().remove(&client_id);
    });
}

#[cfg(any(test, feature = "hydrate"))]
pub(super) fn apply_object_frame(frame: &Frame, board: &mut BoardState) {
    use crate::net::types::{BoardObject, FrameStatus};
    cleanup_stale_drags(board, frame.ts);
    cleanup_stale_cursors(board, frame.ts);

    match frame.syscall.as_str() {
        "object:create" if frame.status == FrameStatus::Done => {
            if let Ok(obj) = serde_json::from_value::<BoardObject>(frame.data.clone()) {
                board.objects.insert(obj.id.clone(), obj);
            }
        }
        "object:update" if frame.status == FrameStatus::Done => {
            if let Some(id) = frame.data.get("id").and_then(|v| v.as_str()) {
                if let Some(existing) = board.objects.get_mut(id) {
                    merge_object_update(existing, &frame.data);
                    board.drag_objects.remove(id);
                    board.drag_updated_at.remove(id);
                } else {
                    // Defensive: don't keep stale selection for unknown objects.
                    board.selection.remove(id);
                }
            }
        }
        "object:delete" if frame.status == FrameStatus::Done => {
            if let Some(id) = frame.data.get("id").and_then(|v| v.as_str()) {
                board.objects.remove(id);
                board.selection.remove(id);
                board.drag_objects.remove(id);
                board.drag_updated_at.remove(id);
            }
        }
        "object:drag" => {
            if let Some(id) = frame.data.get("id").and_then(|v| v.as_str())
                && let Some(existing) = board.objects.get(id as &str)
            {
                // Conflict guard: don't apply peer drag jitter onto local selected object.
                if board.selection.contains(id) {
                    return;
                }
                let mut dragged = existing.clone();
                merge_object_update(&mut dragged, &frame.data);
                if let Some(prev) = board.drag_objects.get(id) {
                    let prev_ts = board.drag_updated_at.get(id).copied().unwrap_or(frame.ts);
                    if should_smooth_drag(prev_ts, frame.ts) {
                        smooth_drag_object(prev, &mut dragged, &frame.data, smoothing_alpha(prev_ts, frame.ts));
                    }
                }
                board.drag_objects.insert(id.to_owned(), dragged);
                board.drag_updated_at.insert(id.to_owned(), frame.ts);
            }
        }
        "object:drag:end" => {
            if let Some(id) = frame.data.get("id").and_then(|v| v.as_str()) {
                board.drag_objects.remove(id);
                board.drag_updated_at.remove(id);
            }
        }
        "cursor:moved" => apply_cursor_moved(board, &frame.data, frame.ts),
        "cursor:clear" => apply_cursor_clear(board, &frame.data),
        _ => {}
    }
}

#[cfg(any(test, feature = "hydrate"))]
fn smooth_drag_object(
    previous: &crate::net::types::BoardObject,
    next: &mut crate::net::types::BoardObject,
    patch: &serde_json::Value,
    alpha: f64,
) {
    if patch.get("x").is_some() {
        next.x = lerp(previous.x, next.x, alpha);
    }
    if patch.get("y").is_some() {
        next.y = lerp(previous.y, next.y, alpha);
    }
    if patch.get("width").is_some()
        && let (Some(prev), Some(curr)) = (previous.width, next.width)
    {
        next.width = Some(lerp(prev, curr, alpha));
    }
    if patch.get("height").is_some()
        && let (Some(prev), Some(curr)) = (previous.height, next.height)
    {
        next.height = Some(lerp(prev, curr, alpha));
    }
    if patch.get("rotation").is_some() {
        next.rotation = lerp(previous.rotation, next.rotation, alpha);
    }
}

#[cfg(any(test, feature = "hydrate"))]
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

#[cfg(any(test, feature = "hydrate"))]
pub(super) fn should_smooth_drag(prev_ts: i64, next_ts: i64) -> bool {
    // Keep fast streams crisp; smooth only slower arrivals.
    next_ts.saturating_sub(prev_ts) >= 80
}

#[cfg(any(test, feature = "hydrate"))]
pub(super) fn smoothing_alpha(prev_ts: i64, next_ts: i64) -> f64 {
    let dt = next_ts.saturating_sub(prev_ts);
    if dt >= 200 {
        0.65
    } else if dt >= 120 {
        0.55
    } else {
        0.45
    }
}

#[cfg(any(test, feature = "hydrate"))]
pub(super) fn cleanup_stale_drags(board: &mut BoardState, now_ts: i64) {
    const DRAG_STALE_MS: i64 = 1500;
    if now_ts <= 0 {
        return;
    }
    let stale = board
        .drag_updated_at
        .iter()
        .filter_map(|(id, ts)| (now_ts - *ts > DRAG_STALE_MS).then_some(id.clone()))
        .collect::<Vec<_>>();
    for id in stale {
        board.drag_updated_at.remove(&id);
        board.drag_objects.remove(&id);
    }
}

#[cfg(any(test, feature = "hydrate"))]
pub(super) fn cleanup_stale_cursors(board: &mut BoardState, now_ts: i64) {
    const CURSOR_STALE_MS: i64 = 3000;
    if now_ts <= 0 {
        return;
    }
    let stale = board
        .cursor_updated_at
        .iter()
        .filter_map(|(id, ts)| (now_ts - *ts > CURSOR_STALE_MS).then_some(id.clone()))
        .collect::<Vec<_>>();
    for id in stale {
        board.cursor_updated_at.remove(&id);
        if let Some(p) = board.presence.get_mut(&id) {
            p.cursor = None;
        }
    }
}

#[cfg(any(test, feature = "hydrate"))]
pub(super) fn apply_cursor_moved(board: &mut BoardState, data: &serde_json::Value, ts: i64) {
    use crate::net::types::Point;

    let Some(client_id) = data.get("client_id").and_then(|v| v.as_str()) else {
        return;
    };
    let x = data.get("x").and_then(|v| v.as_f64());
    let y = data.get("y").and_then(|v| v.as_f64());
    let camera_center_x = data.get("camera_center_x").and_then(|v| v.as_f64());
    let camera_center_y = data.get("camera_center_y").and_then(|v| v.as_f64());
    let camera_zoom = data.get("camera_zoom").and_then(|v| v.as_f64());
    let camera_rotation = data.get("camera_rotation").and_then(|v| v.as_f64());

    if !board.presence.contains_key(client_id) {
        upsert_presence_from_payload(board, data);
    }
    if let Some(p) = board.presence.get_mut(client_id) {
        if let (Some(x), Some(y)) = (x, y) {
            board.cursor_updated_at.insert(client_id.to_owned(), ts);
            p.cursor = Some(Point { x, y });
        }
        if let (Some(cx), Some(cy)) = (camera_center_x, camera_center_y) {
            p.camera_center = Some(Point { x: cx, y: cy });
        }
        if let Some(zoom) = camera_zoom {
            p.camera_zoom = Some(zoom);
        }
        if let Some(rotation) = camera_rotation {
            p.camera_rotation = Some(rotation);
        }
    }
}

#[cfg(any(test, feature = "hydrate"))]
pub(super) fn apply_cursor_clear(board: &mut BoardState, data: &serde_json::Value) {
    let Some(client_id) = data.get("client_id").and_then(|v| v.as_str()) else {
        return;
    };
    board.cursor_updated_at.remove(client_id);
    if let Some(p) = board.presence.get_mut(client_id) {
        p.cursor = None;
    }
}

#[cfg(any(test, feature = "hydrate"))]
pub(super) fn upsert_presence_from_payload(board: &mut BoardState, data: &serde_json::Value) {
    use crate::net::types::Presence;

    let Some(client_id) = data.get("client_id").and_then(|v| v.as_str()) else {
        return;
    };
    let user_id = data
        .get("user_id")
        .and_then(|v| v.as_str())
        .unwrap_or(client_id);
    let user_name = data
        .get("user_name")
        .or_else(|| data.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("Agent");
    let user_color = data
        .get("user_color")
        .or_else(|| data.get("color"))
        .and_then(|v| v.as_str())
        .unwrap_or("#8a8178");

    let existing_cursor = board.presence.get(client_id).and_then(|p| p.cursor.clone());
    let existing_camera_center = board
        .presence
        .get(client_id)
        .and_then(|p| p.camera_center.clone());
    let existing_camera_zoom = board.presence.get(client_id).and_then(|p| p.camera_zoom);
    let existing_camera_rotation = board
        .presence
        .get(client_id)
        .and_then(|p| p.camera_rotation);
    let payload_camera_center = data
        .get("camera_center")
        .and_then(|v| serde_json::from_value::<crate::net::types::Point>(v.clone()).ok())
        .or_else(|| {
            Some(crate::net::types::Point {
                x: data.get("camera_center_x")?.as_f64()?,
                y: data.get("camera_center_y")?.as_f64()?,
            })
        });
    let payload_camera_zoom = data.get("camera_zoom").and_then(|v| v.as_f64());
    let payload_camera_rotation = data.get("camera_rotation").and_then(|v| v.as_f64());
    board.presence.insert(
        client_id.to_owned(),
        Presence {
            client_id: client_id.to_owned(),
            user_id: user_id.to_owned(),
            name: user_name.to_owned(),
            color: user_color.to_owned(),
            cursor: existing_cursor,
            camera_center: payload_camera_center.or(existing_camera_center),
            camera_zoom: payload_camera_zoom.or(existing_camera_zoom),
            camera_rotation: payload_camera_rotation.or(existing_camera_rotation),
        },
    );
}

/// Merge partial object updates into an existing `BoardObject`.
#[cfg(any(test, feature = "hydrate"))]
pub(super) fn merge_object_update(obj: &mut crate::net::types::BoardObject, data: &serde_json::Value) {
    if let Some(x) = data.get("x").and_then(|v| v.as_f64()) {
        obj.x = x;
    }
    if let Some(y) = data.get("y").and_then(|v| v.as_f64()) {
        obj.y = y;
    }
    if let Some(w) = data.get("width").and_then(|v| v.as_f64()) {
        obj.width = Some(w);
    }
    if let Some(h) = data.get("height").and_then(|v| v.as_f64()) {
        obj.height = Some(h);
    }
    if let Some(r) = data.get("rotation").and_then(|v| v.as_f64()) {
        obj.rotation = r;
    }
    if let Some(z) = data.get("z_index").and_then(number_as_i64) {
        #[allow(clippy::cast_possible_truncation)]
        {
            obj.z_index = z as i32;
        }
    }
    if let Some(props) = data.get("props") {
        obj.props = props.clone();
    }
    if let Some(v) = data.get("version").and_then(number_as_i64) {
        obj.version = v;
    }
    if data.get("group_id").is_some() {
        obj.group_id = data
            .get("group_id")
            .and_then(|v| v.as_str())
            .map(str::to_owned);
    }
}

#[cfg(any(test, feature = "hydrate"))]
fn number_as_i64(value: &serde_json::Value) -> Option<i64> {
    value.as_i64().or_else(|| {
        value
            .as_f64()
            .filter(|v| v.is_finite() && v.fract() == 0.0)
            .and_then(|v| {
                if (i64::MIN as f64..=i64::MAX as f64).contains(&v) {
                    Some(v as i64)
                } else {
                    None
                }
            })
    })
}
