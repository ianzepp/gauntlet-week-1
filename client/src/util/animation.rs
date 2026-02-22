//! Animation clip parsing and playback projection helpers.
//!
//! Clips are stored in `BoardObject.props.animation` and replayed as a
//! transient scene projection so canonical board state stays untouched.

#[cfg(test)]
#[path = "animation_test.rs"]
mod animation_test;

use std::collections::{HashMap, HashSet};

use crate::net::types::BoardObject;
use crate::state::board::BoardState;
use crate::state::ui::UiState;

#[derive(Clone, Debug, PartialEq)]
pub struct AnimationClip {
    pub duration_ms: f64,
    pub looped: bool,
    pub scope_object_ids: Option<Vec<String>>,
    pub events: Vec<AnimationEvent>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnimationEvent {
    pub t_ms: f64,
    pub op: AnimationOp,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AnimationOp {
    Create {
        object: BoardObject,
    },
    Update {
        target_id: String,
        patch: serde_json::Value,
    },
    Delete {
        target_id: String,
    },
}

#[must_use]
pub fn resolve_active_clip(board: &BoardState, ui: &UiState) -> Option<(String, AnimationClip)> {
    if let Some(id) = ui.animation_clip_object_id.as_deref()
        && let Some(obj) = board.objects.get(id)
        && let Some(clip) = extract_clip_from_object(obj)
    {
        return Some((id.to_owned(), with_fallback_scope(board, id, clip)));
    }

    let mut selected = board.selection.iter().cloned().collect::<Vec<_>>();
    selected.sort();
    for id in selected {
        if let Some(obj) = board.objects.get(&id)
            && let Some(clip) = extract_clip_from_object(obj)
        {
            return Some((id.clone(), with_fallback_scope(board, &id, clip)));
        }
    }

    let mut ids = board.objects.keys().cloned().collect::<Vec<_>>();
    ids.sort();
    for id in ids {
        if let Some(obj) = board.objects.get(&id)
            && let Some(clip) = extract_clip_from_object(obj)
        {
            return Some((id.clone(), with_fallback_scope(board, &id, clip)));
        }
    }
    None
}

#[must_use]
pub fn extract_clip_from_object(obj: &BoardObject) -> Option<AnimationClip> {
    let root = obj.props.get("animation")?.as_object()?;
    let duration_ms = root
        .get("durationMs")
        .or_else(|| root.get("duration_ms"))
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0)
        .max(0.0);
    if duration_ms <= 0.0 {
        return None;
    }

    let looped = root
        .get("loop")
        .or_else(|| root.get("looped"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);

    let scope_object_ids = root
        .get("scopeObjectIds")
        .or_else(|| root.get("scope_object_ids"))
        .and_then(serde_json::Value::as_array)
        .map(|ids| {
            ids.iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect::<Vec<_>>()
        })
        .filter(|ids| !ids.is_empty());

    let mut events = root
        .get("events")
        .and_then(serde_json::Value::as_array)
        .map_or_else(Vec::new, |items| {
            items
                .iter()
                .filter_map(parse_event)
                .filter(|event| event.t_ms >= 0.0)
                .collect::<Vec<_>>()
        });
    events.sort_by(|a, b| a.t_ms.total_cmp(&b.t_ms));

    Some(AnimationClip { duration_ms, looped, scope_object_ids, events })
}

#[must_use]
pub fn project_clip_scene(
    base_objects: &HashMap<String, BoardObject>,
    active_board_id: Option<&str>,
    clip: &AnimationClip,
    playhead_ms: f64,
) -> HashMap<String, BoardObject> {
    let mut projected = base_objects.clone();
    let capped_playhead = playhead_ms.clamp(0.0, clip.duration_ms);
    let scope = clip
        .scope_object_ids
        .as_ref()
        .map(|ids| ids.iter().cloned().collect::<HashSet<_>>());

    if let Some(scope) = &scope {
        for id in scope {
            projected.remove(id);
        }
    }

    for event in &clip.events {
        if event.t_ms > capped_playhead {
            break;
        }
        apply_event(&mut projected, active_board_id, scope.as_ref(), event);
    }

    projected
}

fn with_fallback_scope(board: &BoardState, clip_object_id: &str, mut clip: AnimationClip) -> AnimationClip {
    if clip.scope_object_ids.is_some() {
        return clip;
    }
    let mut inferred = board
        .objects
        .values()
        .filter_map(|obj| {
            if obj.group_id.as_deref() == Some(clip_object_id) {
                Some(obj.id.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    inferred.sort();
    inferred.dedup();
    if !inferred.is_empty() {
        clip.scope_object_ids = Some(inferred);
    }
    clip
}

fn parse_event(value: &serde_json::Value) -> Option<AnimationEvent> {
    let event = value.as_object()?;
    let t_ms = event
        .get("tMs")
        .or_else(|| event.get("t_ms"))
        .and_then(serde_json::Value::as_f64)?;
    let op_name = event
        .get("op")
        .and_then(serde_json::Value::as_str)?
        .trim()
        .to_ascii_lowercase();

    let op = match op_name.as_str() {
        "create" => {
            let object = event.get("object")?;
            let object = serde_json::from_value::<BoardObject>(object.clone()).ok()?;
            AnimationOp::Create { object }
        }
        "update" => {
            let target_id = event
                .get("targetId")
                .or_else(|| event.get("target_id"))
                .and_then(serde_json::Value::as_str)?
                .to_owned();
            let patch = event
                .get("patch")
                .cloned()
                .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));
            AnimationOp::Update { target_id, patch }
        }
        "delete" => {
            let target_id = event
                .get("targetId")
                .or_else(|| event.get("target_id"))
                .and_then(serde_json::Value::as_str)?
                .to_owned();
            AnimationOp::Delete { target_id }
        }
        _ => return None,
    };

    Some(AnimationEvent { t_ms, op })
}

fn apply_event(
    projected: &mut HashMap<String, BoardObject>,
    active_board_id: Option<&str>,
    scope: Option<&HashSet<String>>,
    event: &AnimationEvent,
) {
    match &event.op {
        AnimationOp::Create { object } => {
            if !in_scope(scope, &object.id) {
                return;
            }
            let mut obj = object.clone();
            if obj.board_id.is_empty()
                && let Some(board_id) = active_board_id
            {
                obj.board_id = board_id.to_owned();
            }
            projected.insert(obj.id.clone(), obj);
        }
        AnimationOp::Update { target_id, patch } => {
            if !in_scope(scope, target_id) {
                return;
            }
            let Some(existing) = projected.get_mut(target_id) else {
                return;
            };
            apply_patch(existing, patch);
        }
        AnimationOp::Delete { target_id } => {
            if !in_scope(scope, target_id) {
                return;
            }
            projected.remove(target_id);
        }
    }
}

fn in_scope(scope: Option<&HashSet<String>>, object_id: &str) -> bool {
    scope.is_none_or(|ids| ids.contains(object_id))
}

fn apply_patch(obj: &mut BoardObject, patch: &serde_json::Value) {
    let Some(data) = patch.as_object() else {
        return;
    };

    if let Some(x) = data.get("x").and_then(serde_json::Value::as_f64) {
        obj.x = x;
    }
    if let Some(y) = data.get("y").and_then(serde_json::Value::as_f64) {
        obj.y = y;
    }
    if let Some(width) = data.get("width").and_then(serde_json::Value::as_f64) {
        obj.width = Some(width);
    }
    if let Some(height) = data.get("height").and_then(serde_json::Value::as_f64) {
        obj.height = Some(height);
    }
    if let Some(rotation) = data.get("rotation").and_then(serde_json::Value::as_f64) {
        obj.rotation = rotation;
    }
    if let Some(z_index) = data.get("z_index").and_then(serde_json::Value::as_i64)
        && let Ok(z_index) = i32::try_from(z_index)
    {
        obj.z_index = z_index;
    }
    if let Some(next_kind) = data.get("kind").and_then(serde_json::Value::as_str) {
        obj.kind = next_kind.to_owned();
    }
    if let Some(group_id) = data.get("group_id") {
        obj.group_id = group_id.as_str().map(str::to_owned);
    }

    if let Some(next_props) = data.get("props").and_then(serde_json::Value::as_object) {
        if !obj.props.is_object() {
            obj.props = serde_json::json!({});
        }
        if let Some(existing) = obj.props.as_object_mut() {
            for (k, v) in next_props {
                if v.is_null() {
                    existing.remove(k);
                } else {
                    existing.insert(k.clone(), v.clone());
                }
            }
        }
    }
}
