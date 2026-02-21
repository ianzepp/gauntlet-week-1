//! Board member management routes.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::response::{IntoResponse, Json, Response};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::routes::auth::AuthUser;
use crate::services::board::{self, BoardMemberRow, BoardRole};
use crate::state::AppState;

#[derive(Serialize)]
pub struct BoardMemberResponse {
    pub user_id: Uuid,
    pub name: String,
    pub avatar_url: Option<String>,
    pub color: String,
    pub role: String,
    pub is_owner: bool,
}

fn to_response(row: BoardMemberRow) -> BoardMemberResponse {
    BoardMemberResponse {
        user_id: row.user_id,
        name: row.name,
        avatar_url: row.avatar_url,
        color: row.color,
        role: row.role.as_str().to_owned(),
        is_owner: row.is_owner,
    }
}

#[derive(Deserialize)]
pub struct UpsertBoardMemberBody {
    pub user_id: Option<Uuid>,
    pub email: Option<String>,
    pub role: String,
}

#[derive(Deserialize)]
pub struct UpdateBoardMemberBody {
    pub role: String,
}

/// `GET /api/boards/:id/members` — list board members.
pub async fn list_members(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(board_id): Path<Uuid>,
) -> Result<Json<Vec<BoardMemberResponse>>, StatusCode> {
    let rows = board::list_board_members(&state.pool, board_id, auth.user.id)
        .await
        .map_err(board_error_to_status)?;

    Ok(Json(rows.into_iter().map(to_response).collect()))
}

/// `POST /api/boards/:id/members` — add or update a board member.
pub async fn upsert_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(board_id): Path<Uuid>,
    Json(body): Json<UpsertBoardMemberBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let Some(role) = BoardRole::from_str(&body.role) else {
        return Err(StatusCode::BAD_REQUEST);
    };

    let target_user_id = match (body.user_id, body.email.as_deref()) {
        (Some(user_id), _) => user_id,
        (None, Some(email)) => board::resolve_user_id_by_email(&state.pool, email)
            .await
            .map_err(board_error_to_status)?
            .ok_or(StatusCode::NOT_FOUND)?,
        (None, None) => return Err(StatusCode::BAD_REQUEST),
    };

    board::add_or_update_board_member(&state.pool, board_id, auth.user.id, target_user_id, role)
        .await
        .map_err(board_error_to_status)?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// `PATCH /api/boards/:id/members/:user_id` — update member role.
pub async fn update_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((board_id, member_user_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateBoardMemberBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let Some(role) = BoardRole::from_str(&body.role) else {
        return Err(StatusCode::BAD_REQUEST);
    };

    board::add_or_update_board_member(&state.pool, board_id, auth.user.id, member_user_id, role)
        .await
        .map_err(board_error_to_status)?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// `DELETE /api/boards/:id/members/:user_id` — remove member.
pub async fn delete_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((board_id, member_user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    board::remove_board_member(&state.pool, board_id, auth.user.id, member_user_id)
        .await
        .map_err(board_error_to_status)?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub(crate) fn board_error_to_status(err: board::BoardError) -> StatusCode {
    match err {
        board::BoardError::NotFound(_) => StatusCode::NOT_FOUND,
        board::BoardError::Forbidden(_) => StatusCode::FORBIDDEN,
        board::BoardError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[derive(Serialize)]
struct BoardExportMetaLine {
    #[serde(rename = "type")]
    line_type: &'static str,
    version: u8,
    board_id: Uuid,
    exported_at_ms: u128,
    object_count: usize,
}

#[derive(Serialize)]
struct BoardExportObjectLine {
    #[serde(rename = "type")]
    line_type: &'static str,
    #[serde(flatten)]
    object: board::BoardExportObject,
}

#[derive(Deserialize)]
pub struct ImportJsonlBody {
    pub jsonl: String,
}

#[derive(Serialize)]
pub struct ImportJsonlResponse {
    pub imported: usize,
    pub skipped: usize,
}

/// `GET /api/boards/:id/export.jsonl` — download board snapshot as NDJSON/JSONL.
pub async fn export_jsonl(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(board_id): Path<Uuid>,
) -> Result<Response, StatusCode> {
    let objects = board::list_board_export_objects(&state.pool, board_id, auth.user.id)
        .await
        .map_err(board_error_to_status)?;

    let exported_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());

    let mut lines = Vec::with_capacity(objects.len() + 1);
    let meta = BoardExportMetaLine {
        line_type: "board_export_meta",
        version: 1,
        board_id,
        exported_at_ms,
        object_count: objects.len(),
    };
    let meta_line = serde_json::to_string(&meta).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    lines.push(format!("{meta_line}\n"));

    for object in objects {
        let line = BoardExportObjectLine { line_type: "object", object };
        let serialized = serde_json::to_string(&line).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        lines.push(format!("{serialized}\n"));
    }

    let stream = futures::stream::iter(
        lines
            .into_iter()
            .map(|line| Ok::<axum::body::Bytes, std::convert::Infallible>(axum::body::Bytes::from(line))),
    );
    let body = axum::body::Body::from_stream(stream);
    let filename = format!("board-{board_id}.jsonl");

    Ok((
        [
            (CONTENT_TYPE, "application/x-ndjson; charset=utf-8"),
            (CONTENT_DISPOSITION, &format!("attachment; filename=\"{filename}\"")),
        ],
        body,
    )
        .into_response())
}

fn now_ms_i64() -> i64 {
    let Ok(duration) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) else {
        return 0;
    };
    i64::try_from(duration.as_millis()).unwrap_or(0)
}

#[allow(clippy::cast_possible_truncation)]
pub(crate) fn parse_import_object_line(
    line: &str,
    board_id: Uuid,
    user_id: Uuid,
) -> Result<Option<crate::state::BoardObject>, serde_json::Error> {
    let value = serde_json::from_str::<serde_json::Value>(line)?;
    let Some(map) = value.as_object() else {
        return Ok(None);
    };

    let line_type = map.get("type").and_then(serde_json::Value::as_str);
    if line_type == Some("board_export_meta") {
        return Ok(None);
    }
    if line_type != Some("object") && !map.contains_key("kind") {
        return Ok(None);
    }

    let kind = map
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("rectangle")
        .to_owned();
    let x = map
        .get("x")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let y = map
        .get("y")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let width = map.get("width").and_then(serde_json::Value::as_f64);
    let height = map.get("height").and_then(serde_json::Value::as_f64);
    let rotation = map
        .get("rotation")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let z_index = map
        .get("z_index")
        .and_then(|value| {
            value
                .as_i64()
                .or_else(|| value.as_f64().map(|float| float as i64))
        })
        .and_then(|value| i32::try_from(value).ok())
        .unwrap_or(0);
    let props = map
        .get("props")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let version = map
        .get("version")
        .and_then(|value| {
            value
                .as_i64()
                .or_else(|| value.as_f64().map(|float| float as i64))
        })
        .and_then(|value| i32::try_from(value).ok())
        .unwrap_or(1)
        .max(1);
    let group_id = map
        .get("group_id")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| Uuid::parse_str(s).ok());

    Ok(Some(crate::state::BoardObject {
        id: Uuid::new_v4(),
        board_id,
        kind,
        x,
        y,
        width,
        height,
        rotation,
        z_index,
        props,
        created_by: Some(user_id),
        version,
        group_id,
    }))
}

#[cfg(test)]
#[path = "boards_test.rs"]
mod tests;

/// `POST /api/boards/:id/import.jsonl` — import NDJSON/JSONL object lines.
pub async fn import_jsonl(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(board_id): Path<Uuid>,
    Json(body): Json<ImportJsonlBody>,
) -> Result<Json<ImportJsonlResponse>, StatusCode> {
    board::ensure_board_permission(&state.pool, board_id, auth.user.id, board::BoardPermission::Edit)
        .await
        .map_err(board_error_to_status)?;

    let mut objects = Vec::new();
    let mut skipped = 0_usize;

    for raw_line in body.jsonl.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        match parse_import_object_line(line, board_id, auth.user.id) {
            Ok(Some(object)) => objects.push(object),
            Ok(None) | Err(_) => skipped = skipped.saturating_add(1),
        }
    }

    if objects.is_empty() {
        return Ok(Json(ImportJsonlResponse { imported: 0, skipped }));
    }

    board::flush_objects(&state.pool, &objects)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    {
        let mut boards = state.boards.write().await;
        if let Some(board_state) = boards.get_mut(&board_id) {
            for object in &objects {
                board_state.objects.insert(object.id, object.clone());
                board_state.dirty.remove(&object.id);
            }
        }
    }

    for object in &objects {
        let data = match serde_json::to_value(object) {
            Ok(serde_json::Value::Object(map)) => map.into_iter().collect(),
            _ => continue,
        };
        let frame = crate::frame::Frame {
            id: Uuid::new_v4(),
            parent_id: None,
            ts: now_ms_i64(),
            board_id: Some(board_id),
            from: None,
            syscall: "object:create".to_owned(),
            status: crate::frame::Status::Done,
            data,
        };
        board::broadcast(&state, board_id, &frame, None).await;
    }

    Ok(Json(ImportJsonlResponse { imported: objects.len(), skipped }))
}
