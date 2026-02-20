//! Parsing helpers for `frame_client` payload handling.

#[cfg(test)]
#[path = "frame_client_parse_test.rs"]
mod frame_client_parse_test;

#[cfg(any(test, feature = "hydrate"))]
use crate::net::types::Frame;
#[cfg(any(test, feature = "hydrate"))]
use crate::state::ai::AiMessage;
#[cfg(any(test, feature = "hydrate"))]
use crate::state::boards::{BoardListItem, BoardListPreviewObject};
#[cfg(any(test, feature = "hydrate"))]
use crate::state::chat::ChatMessage;

#[cfg(any(test, feature = "hydrate"))]
pub(super) fn parse_board_list_items(data: &serde_json::Value) -> Vec<BoardListItem> {
    let Some(rows) = data.get("boards").and_then(|v| v.as_array()) else {
        return Vec::new();
    };

    rows.iter()
        .filter_map(|row| {
            let id = row.get("id")?.as_str()?.to_owned();
            let name = row.get("name")?.as_str()?.to_owned();
            let owner_id = row
                .get("owner_id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            let snapshot = row
                .get("snapshot")
                .and_then(serde_json::Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(parse_board_list_preview_object)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            Some(BoardListItem { id, name, owner_id, snapshot })
        })
        .collect()
}

#[cfg(any(test, feature = "hydrate"))]
fn parse_board_list_preview_object(row: &serde_json::Value) -> Option<BoardListPreviewObject> {
    let kind = row
        .get("kind")
        .and_then(serde_json::Value::as_str)?
        .to_owned();
    let x = row.get("x").and_then(serde_json::Value::as_f64)?;
    let y = row.get("y").and_then(serde_json::Value::as_f64)?;
    let width = row.get("width").and_then(serde_json::Value::as_f64);
    let height = row.get("height").and_then(serde_json::Value::as_f64);
    let rotation = row
        .get("rotation")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let z_index = row
        .get("z_index")
        .and_then(|value| {
            value
                .as_i64()
                .or_else(|| value.as_f64().map(|n| n.round() as i64))
        })
        .and_then(|n| i32::try_from(n).ok())
        .unwrap_or(0);
    Some(BoardListPreviewObject { kind, x, y, width, height, rotation, z_index })
}

#[cfg(any(test, feature = "hydrate"))]
pub(super) fn deleted_board_id(frame: &Frame) -> Option<String> {
    frame
        .data
        .get("board_id")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .or_else(|| frame.board_id.clone())
}

#[cfg(any(test, feature = "hydrate"))]
pub(super) fn parse_board_objects(data: &serde_json::Value) -> Option<Vec<crate::net::types::BoardObject>> {
    data.get("objects")
        .cloned()
        .and_then(|v| serde_json::from_value::<Vec<crate::net::types::BoardObject>>(v).ok())
}

#[cfg(any(test, feature = "hydrate"))]
pub(super) fn parse_board_object_item(data: &serde_json::Value) -> Option<crate::net::types::BoardObject> {
    serde_json::from_value::<crate::net::types::BoardObject>(data.clone()).ok()
}

#[cfg(any(test, feature = "hydrate"))]
pub(super) fn parse_chat_message(frame: &Frame, data: &serde_json::Value) -> Option<ChatMessage> {
    let content = pick_str(data, &["content", "message"])?.to_owned();

    let id = data
        .get("id")
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| frame.id.clone());

    let user_id = pick_str(data, &["user_id", "from"])
        .or(frame.from.as_deref())
        .unwrap_or("unknown")
        .to_owned();

    let user_name = data
        .get("user_name")
        .and_then(|v| v.as_str())
        .unwrap_or("Agent")
        .to_owned();

    let user_color = data
        .get("user_color")
        .and_then(|v| v.as_str())
        .unwrap_or("#8a8178")
        .to_owned();

    let timestamp = pick_number(data, &["timestamp", "ts"]).unwrap_or(frame.ts as f64);

    Some(ChatMessage { id, user_id, user_name, user_color, content, timestamp })
}

#[cfg(any(test, feature = "hydrate"))]
pub(super) fn parse_ai_message_value(data: &serde_json::Value) -> Option<AiMessage> {
    let id = data
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("ai-msg")
        .to_owned();
    let role = data
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or("assistant")
        .to_owned();
    let content = pick_str(data, &["content", "text"])
        .unwrap_or_default()
        .to_owned();
    if content.trim().is_empty() {
        return None;
    }
    let timestamp = pick_number(data, &["timestamp", "ts"]).unwrap_or(0.0);
    let mutations = data.get("mutations").and_then(number_as_i64);

    Some(AiMessage { id, role, content, timestamp, mutations })
}

#[cfg(any(test, feature = "hydrate"))]
pub(super) fn parse_ai_prompt_message(frame: &Frame) -> Option<AiMessage> {
    if let Some(mut msg) = parse_ai_message_value(&frame.data) {
        if frame.status == crate::net::types::FrameStatus::Error && msg.role == "assistant" {
            msg.role = "error".to_owned();
        }
        if msg.timestamp == 0.0 {
            msg.timestamp = frame.ts as f64;
        }
        return Some(msg);
    }

    let content = pick_str(&frame.data, &["text", "content"])?;

    Some(AiMessage {
        id: frame.id.clone(),
        role: if frame.status == crate::net::types::FrameStatus::Error {
            "error".to_owned()
        } else {
            "assistant".to_owned()
        },
        content: content.to_owned(),
        timestamp: frame.ts as f64,
        mutations: frame.data.get("mutations").and_then(number_as_i64),
    })
}

#[cfg(any(test, feature = "hydrate"))]
pub(super) fn parse_ai_prompt_user_message(frame: &Frame) -> Option<AiMessage> {
    let prompt = frame
        .data
        .get("prompt")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)?;
    if prompt.is_empty() {
        return None;
    }

    Some(AiMessage {
        // ai:prompt done/error replies carry a new id and set parent_id to the original request id.
        // Use parent_id so optimistic user rows reconcile instead of duplicating.
        id: frame.parent_id.clone().unwrap_or_else(|| frame.id.clone()),
        role: "user".to_owned(),
        content: prompt.to_owned(),
        timestamp: frame.ts as f64,
        mutations: None,
    })
}

#[cfg(any(test, feature = "hydrate"))]
pub(super) fn frame_error_message(frame: &Frame) -> Option<&str> {
    pick_str(&frame.data, &["message", "error"])
}

#[cfg(any(test, feature = "hydrate"))]
pub(super) fn pick_str<'a>(data: &'a serde_json::Value, keys: &[&str]) -> Option<&'a str> {
    for key in keys {
        if let Some(value) = data.get(key).and_then(serde_json::Value::as_str) {
            return Some(value);
        }
    }
    None
}

#[cfg(any(test, feature = "hydrate"))]
pub(super) fn pick_number(data: &serde_json::Value, keys: &[&str]) -> Option<f64> {
    for key in keys {
        if let Some(value) = data.get(key) {
            if let Some(n) = value.as_f64() {
                return Some(n);
            }
            if let Some(n) = value.as_i64() {
                #[allow(clippy::cast_precision_loss)]
                {
                    return Some(n as f64);
                }
            }
        }
    }
    None
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
