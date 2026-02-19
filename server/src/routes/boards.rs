//! Board member management routes.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
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
