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

fn board_error_to_status(err: board::BoardError) -> StatusCode {
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
