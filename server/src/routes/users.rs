//! User profile routes.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;

use super::auth::AuthUser;
use crate::state::AppState;

#[derive(Serialize)]
pub struct UserProfile {
    pub id: Uuid,
    pub name: String,
    pub avatar_url: Option<String>,
    pub color: String,
    pub member_since: Option<String>,
    pub stats: UserStats,
}

#[derive(Serialize)]
pub struct UserStats {
    pub total_frames: i64,
    pub objects_created: i64,
    pub boards_active: i64,
    pub last_active: Option<String>,
    pub top_syscalls: Vec<SyscallCount>,
}

#[derive(Serialize)]
pub struct SyscallCount {
    pub syscall: String,
    pub count: i64,
}

/// `GET /api/users/:id/profile` — return user info with aggregate stats.
pub async fn user_profile(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(user_id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    // Fetch user row.
    let user_row = sqlx::query(
        r"SELECT id, name, avatar_url, color,
                to_char(created_at, 'YYYY-MM-DD') AS member_since
         FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    // Aggregate stats from frames table (match on "from" = user_id).
    let stats_row = sqlx::query(
        r#"SELECT
               COALESCE(COUNT(*), 0)                                        AS total_frames,
               COALESCE(COUNT(DISTINCT board_id), 0)                        AS boards_active,
               to_char(
                   MAX(to_timestamp(ts / 1000.0) AT TIME ZONE 'UTC'),
                   'YYYY-MM-DD HH24:MI'
               ) AS last_active
           FROM frames
           WHERE "from" = $1::text"#,
    )
    .bind(user_id.to_string())
    .fetch_one(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Objects created (from board_objects table, more reliable).
    let obj_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM board_objects WHERE created_by = $1")
        .bind(user_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Top syscalls breakdown.
    let syscall_rows = sqlx::query(
        r#"SELECT syscall, COUNT(*) AS cnt
           FROM frames
           WHERE "from" = $1::text
           GROUP BY syscall
           ORDER BY cnt DESC
           LIMIT 5"#,
    )
    .bind(user_id.to_string())
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let top_syscalls: Vec<SyscallCount> = syscall_rows
        .iter()
        .map(|r| SyscallCount { syscall: r.get("syscall"), count: r.get("cnt") })
        .collect();

    let profile = UserProfile {
        id: user_row.get("id"),
        name: user_row.get("name"),
        avatar_url: user_row.get("avatar_url"),
        color: user_row.get("color"),
        member_since: user_row.get("member_since"),
        stats: UserStats {
            total_frames: stats_row.get("total_frames"),
            objects_created: obj_count,
            boards_active: stats_row.get("boards_active"),
            last_active: stats_row.get("last_active"),
            top_syscalls,
        },
    };

    Ok(Json(profile))
}
