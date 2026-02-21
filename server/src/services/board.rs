//! Board service — CRUD, join/part, and state hydration.
//!
//! DESIGN
//! ======
//! Boards are created and listed via REST-like operations (dispatched from
//! WS frames). Board state is hydrated from Postgres on first join and kept
//! in memory while any client is connected.
//!
//! ERROR HANDLING
//! ==============
//! On last-client part, dirty objects are flushed before eviction. If that
//! flush fails, the board is intentionally kept in memory with dirty flags
//! intact so the persistence worker can retry instead of losing edits.

use std::collections::HashMap;

use sqlx::PgPool;
use sqlx::QueryBuilder;
use tokio::sync::mpsc;
use tracing::info;
use uuid::Uuid;

use crate::frame::Frame;
use crate::state::{AppState, BoardObject, BoardState, ConnectedClient};

// =============================================================================
// TYPES
// =============================================================================

#[derive(Debug, thiserror::Error)]
pub enum BoardError {
    #[error("board not found: {0}")]
    NotFound(Uuid),
    #[error("board access forbidden: {0}")]
    Forbidden(Uuid),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl crate::frame::ErrorCode for BoardError {
    fn error_code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "E_BOARD_NOT_FOUND",
            Self::Forbidden(_) => "E_BOARD_FORBIDDEN",
            Self::Database(_) => "E_DATABASE",
        }
    }
}

/// Row returned from board queries.
#[derive(Debug, Clone)]
pub struct BoardRow {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Option<Uuid>,
    pub is_public: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardRole {
    Viewer,
    Editor,
    Admin,
}

impl BoardRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Viewer => "viewer",
            Self::Editor => "editor",
            Self::Admin => "admin",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "viewer" => Some(Self::Viewer),
            "editor" => Some(Self::Editor),
            "admin" => Some(Self::Admin),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardPermission {
    View,
    Edit,
    Admin,
}

#[derive(Debug, Clone)]
pub struct BoardMemberRow {
    pub user_id: Uuid,
    pub name: String,
    pub avatar_url: Option<String>,
    pub color: String,
    pub role: BoardRole,
    pub is_owner: bool,
}

#[derive(Debug, Clone)]
pub struct BoardUser {
    pub client_id: Uuid,
    pub user_id: Uuid,
    pub user_name: String,
    pub user_color: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BoardPreviewObject {
    pub kind: String,
    pub x: f64,
    pub y: f64,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub rotation: f64,
    pub z_index: i32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BoardExportObject {
    pub id: Uuid,
    pub board_id: Uuid,
    pub kind: String,
    pub x: f64,
    pub y: f64,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub rotation: f64,
    pub z_index: i32,
    pub props: serde_json::Value,
    pub created_by: Option<Uuid>,
    pub version: i32,
}

// =============================================================================
// CRUD
// =============================================================================

/// Create a new board.
///
/// # Errors
///
/// Returns a database error if the insert fails.
pub async fn create_board(pool: &PgPool, name: &str, owner_id: Uuid) -> Result<BoardRow, BoardError> {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO boards (id, name, owner_id, is_public) VALUES ($1, $2, $3, false)")
        .bind(id)
        .bind(name)
        .bind(owner_id)
        .execute(pool)
        .await?;

    Ok(BoardRow { id, name: name.to_string(), owner_id: Some(owner_id), is_public: false })
}

/// List all boards.
///
/// # Errors
///
/// Returns a database error if the query fails.
pub async fn list_boards(pool: &PgPool, user_id: Uuid) -> Result<Vec<BoardRow>, BoardError> {
    let rows = sqlx::query_as::<_, (Uuid, String, Option<Uuid>, bool)>(
        "SELECT id, name, owner_id, is_public
         FROM boards
         WHERE owner_id = $1
            OR owner_id IS NULL
            OR is_public = true
            OR EXISTS (
                SELECT 1 FROM board_members bm
                WHERE bm.board_id = boards.id AND bm.user_id = $1
            )
         ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, name, owner_id, is_public)| BoardRow { id, name, owner_id, is_public })
        .collect())
}

fn role_satisfies(role: BoardRole, permission: BoardPermission) -> bool {
    match permission {
        BoardPermission::View => true,
        BoardPermission::Edit => matches!(role, BoardRole::Editor | BoardRole::Admin),
        BoardPermission::Admin => matches!(role, BoardRole::Admin),
    }
}

/// Ensure a user has permission on a board.
///
/// Legacy `owner_id IS NULL` boards remain globally accessible for compatibility.
pub async fn ensure_board_permission(
    pool: &PgPool,
    board_id: Uuid,
    user_id: Uuid,
    permission: BoardPermission,
) -> Result<(), BoardError> {
    let board_row = sqlx::query_as::<_, (Option<Uuid>, bool)>("SELECT owner_id, is_public FROM boards WHERE id = $1")
        .bind(board_id)
        .fetch_optional(pool)
        .await?;

    let Some((owner_id, is_public)) = board_row else {
        return Err(BoardError::NotFound(board_id));
    };

    if owner_id == Some(user_id) {
        return Ok(());
    }

    if owner_id.is_none() {
        return Ok(());
    }

    if permission == BoardPermission::View && is_public {
        return Ok(());
    }

    let member_role =
        sqlx::query_scalar::<_, String>("SELECT role FROM board_members WHERE board_id = $1 AND user_id = $2")
            .bind(board_id)
            .bind(user_id)
            .fetch_optional(pool)
            .await?;

    if let Some(raw_role) = member_role
        && let Some(role) = BoardRole::from_str(&raw_role)
        && role_satisfies(role, permission)
    {
        return Ok(());
    }

    Err(BoardError::Forbidden(board_id))
}

#[must_use]
pub async fn client_has_permission(
    state: &AppState,
    board_id: Uuid,
    client_id: Uuid,
    permission: BoardPermission,
) -> bool {
    let boards = state.boards.read().await;
    let Some(board_state) = boards.get(&board_id) else {
        return false;
    };
    let Some(client) = board_state.users.get(&client_id) else {
        return false;
    };
    match permission {
        BoardPermission::View => true,
        BoardPermission::Edit => client.can_edit || client.can_admin,
        BoardPermission::Admin => client.can_admin,
    }
}

pub async fn list_board_members(
    pool: &PgPool,
    board_id: Uuid,
    user_id: Uuid,
) -> Result<Vec<BoardMemberRow>, BoardError> {
    ensure_board_permission(pool, board_id, user_id, BoardPermission::View).await?;

    let owner_row = sqlx::query_as::<_, (Option<Uuid>,)>("SELECT owner_id FROM boards WHERE id = $1")
        .bind(board_id)
        .fetch_optional(pool)
        .await?;
    let Some((owner_id,)) = owner_row else {
        return Err(BoardError::NotFound(board_id));
    };

    let mut rows = sqlx::query_as::<_, (Uuid, String, Option<String>, String, String)>(
        r"SELECT u.id, u.name, u.avatar_url, u.color, bm.role
          FROM board_members bm
          JOIN users u ON u.id = bm.user_id
          WHERE bm.board_id = $1
          ORDER BY u.name ASC",
    )
    .bind(board_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .filter_map(|(member_id, name, avatar_url, color, role_raw)| {
        Some(BoardMemberRow {
            user_id: member_id,
            name,
            avatar_url,
            color,
            role: BoardRole::from_str(&role_raw)?,
            is_owner: false,
        })
    })
    .collect::<Vec<_>>();

    if let Some(owner_id) = owner_id {
        let owner_exists = rows.iter().any(|row| row.user_id == owner_id);
        if !owner_exists
            && let Some((name, avatar_url, color)) = sqlx::query_as::<_, (String, Option<String>, String)>(
                "SELECT name, avatar_url, color FROM users WHERE id = $1",
            )
            .bind(owner_id)
            .fetch_optional(pool)
            .await?
        {
            rows.insert(
                0,
                BoardMemberRow { user_id: owner_id, name, avatar_url, color, role: BoardRole::Admin, is_owner: true },
            );
        }
    }

    Ok(rows)
}

pub async fn add_or_update_board_member(
    pool: &PgPool,
    board_id: Uuid,
    acting_user_id: Uuid,
    target_user_id: Uuid,
    role: BoardRole,
) -> Result<(), BoardError> {
    ensure_board_permission(pool, board_id, acting_user_id, BoardPermission::Admin).await?;

    let owner_id = sqlx::query_scalar::<_, Option<Uuid>>("SELECT owner_id FROM boards WHERE id = $1")
        .bind(board_id)
        .fetch_optional(pool)
        .await?;

    if let Some(Some(owner_id)) = owner_id
        && owner_id == target_user_id
    {
        return Ok(());
    }

    sqlx::query(
        "INSERT INTO board_members (board_id, user_id, role)
         VALUES ($1, $2, $3)
         ON CONFLICT (board_id, user_id) DO UPDATE SET role = EXCLUDED.role",
    )
    .bind(board_id)
    .bind(target_user_id)
    .bind(role.as_str())
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn remove_board_member(
    pool: &PgPool,
    board_id: Uuid,
    acting_user_id: Uuid,
    target_user_id: Uuid,
) -> Result<(), BoardError> {
    ensure_board_permission(pool, board_id, acting_user_id, BoardPermission::Admin).await?;

    let owner_id = sqlx::query_scalar::<_, Option<Uuid>>("SELECT owner_id FROM boards WHERE id = $1")
        .bind(board_id)
        .fetch_optional(pool)
        .await?;

    if let Some(Some(owner_id)) = owner_id
        && owner_id == target_user_id
    {
        return Err(BoardError::Forbidden(board_id));
    }

    sqlx::query("DELETE FROM board_members WHERE board_id = $1 AND user_id = $2")
        .bind(board_id)
        .bind(target_user_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn resolve_user_id_by_email(pool: &PgPool, email: &str) -> Result<Option<Uuid>, BoardError> {
    let normalized = email.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Ok(None);
    }
    let row = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(normalized)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

// =============================================================================
// ACCESS CODES
// =============================================================================

/// Characters used for access codes (no ambiguous chars: 0/O/I/L/1).
const ACCESS_CODE_CHARSET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
const ACCESS_CODE_LENGTH: usize = 6;

/// Generate a 6-char access code for a board.
///
/// # Errors
///
/// Returns `Forbidden` if the user is not an admin of the board,
/// or a database error if the insert fails.
pub async fn generate_access_code(pool: &PgPool, board_id: Uuid, user_id: Uuid) -> Result<String, BoardError> {
    ensure_board_permission(pool, board_id, user_id, BoardPermission::Admin).await?;

    let code = generate_random_code();

    sqlx::query("INSERT INTO board_access_codes (code, board_id, created_by) VALUES ($1, $2, $3)")
        .bind(&code)
        .bind(board_id)
        .bind(user_id)
        .execute(pool)
        .await?;

    Ok(code)
}

/// Redeem an access code and add the user as an editor on the board.
///
/// # Errors
///
/// Returns `NotFound` if the code is invalid or expired,
/// or a database error if the upsert fails.
pub async fn redeem_access_code(pool: &PgPool, code: &str, user_id: Uuid) -> Result<Uuid, BoardError> {
    let normalized = code.trim().to_ascii_uppercase();

    let row =
        sqlx::query_as::<_, (Uuid,)>("SELECT board_id FROM board_access_codes WHERE code = $1 AND expires_at > now()")
            .bind(&normalized)
            .fetch_optional(pool)
            .await?;

    let Some((board_id,)) = row else {
        // Use a sentinel UUID for "code not found" — the user doesn't need to know which board.
        return Err(BoardError::NotFound(Uuid::nil()));
    };

    // Add user as editor (upsert — no-op if already a member with same or higher role).
    sqlx::query(
        "INSERT INTO board_members (board_id, user_id, role) VALUES ($1, $2, 'editor') \
         ON CONFLICT (board_id, user_id) DO NOTHING",
    )
    .bind(board_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(board_id)
}

fn generate_random_code() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    (0..ACCESS_CODE_LENGTH)
        .map(|_| {
            let idx = rng.random_range(0..ACCESS_CODE_CHARSET.len());
            ACCESS_CODE_CHARSET[idx] as char
        })
        .collect()
}

/// Load lightweight preview objects for a set of boards.
///
/// # Errors
///
/// Returns a database error if the query fails.
pub async fn list_board_preview_objects(
    pool: &PgPool,
    board_ids: &[Uuid],
    per_board_limit: i64,
) -> Result<HashMap<Uuid, Vec<BoardPreviewObject>>, BoardError> {
    if board_ids.is_empty() || per_board_limit <= 0 {
        return Ok(HashMap::new());
    }

    let mut builder = QueryBuilder::new(
        "SELECT board_id, kind, x, y, width, height, rotation, z_index
         FROM (
            SELECT board_id, kind, x, y, width, height, rotation, z_index, id,
                   row_number() OVER (PARTITION BY board_id ORDER BY z_index ASC, id ASC) AS row_num
            FROM board_objects
            WHERE board_id IN (",
    );
    {
        let mut separated = builder.separated(", ");
        for board_id in board_ids {
            separated.push_bind(board_id);
        }
    }
    builder.push(
        ")
         ) ranked
         WHERE row_num <= ",
    );
    builder.push_bind(per_board_limit);
    builder.push(" ORDER BY board_id ASC, z_index ASC, row_num ASC");

    let rows = builder
        .build_query_as::<(Uuid, String, f64, f64, Option<f64>, Option<f64>, f64, i32)>()
        .fetch_all(pool)
        .await?;

    let mut out: HashMap<Uuid, Vec<BoardPreviewObject>> = HashMap::new();
    for (board_id, kind, x, y, width, height, rotation, z_index) in rows {
        out.entry(board_id)
            .or_default()
            .push(BoardPreviewObject { kind, x, y, width, height, rotation, z_index });
    }
    Ok(out)
}

/// Load full board objects for JSONL export.
///
/// # Errors
///
/// Returns `Forbidden`/`NotFound` when access is invalid or a database error
/// if the query fails.
pub async fn list_board_export_objects(
    pool: &PgPool,
    board_id: Uuid,
    user_id: Uuid,
) -> Result<Vec<BoardExportObject>, BoardError> {
    ensure_board_permission(pool, board_id, user_id, BoardPermission::View).await?;

    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            String,
            f64,
            f64,
            Option<f64>,
            Option<f64>,
            f64,
            i32,
            serde_json::Value,
            Option<Uuid>,
            i32,
        ),
    >(
        "SELECT id, board_id, kind, x, y, width, height, rotation, z_index, props, created_by, version
         FROM board_objects
         WHERE board_id = $1
         ORDER BY z_index ASC, id ASC",
    )
    .bind(board_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(id, board_id, kind, x, y, width, height, rotation, z_index, props, created_by, version)| {
                BoardExportObject {
                    id,
                    board_id,
                    kind,
                    x,
                    y,
                    width,
                    height,
                    rotation,
                    z_index,
                    props,
                    created_by,
                    version,
                }
            },
        )
        .collect())
}

/// Delete a board by ID.
///
/// # Errors
///
/// Returns a database error if the delete fails.
pub async fn delete_board(pool: &PgPool, board_id: Uuid, user_id: Uuid) -> Result<(), BoardError> {
    ensure_board_permission(pool, board_id, user_id, BoardPermission::Admin).await?;

    let result = sqlx::query("DELETE FROM boards WHERE id = $1")
        .bind(board_id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(BoardError::NotFound(board_id));
    }
    Ok(())
}

pub async fn set_board_visibility(
    pool: &PgPool,
    board_id: Uuid,
    user_id: Uuid,
    is_public: bool,
) -> Result<(), BoardError> {
    ensure_board_permission(pool, board_id, user_id, BoardPermission::Admin).await?;

    let result = sqlx::query("UPDATE boards SET is_public = $2 WHERE id = $1")
        .bind(board_id)
        .bind(is_public)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(BoardError::NotFound(board_id));
    }
    Ok(())
}

// =============================================================================
// JOIN / PART
// =============================================================================

/// Join a board. Hydrates from Postgres if not already in memory.
/// Returns the current list of board objects.
///
/// # Errors
///
/// Returns a database error if hydration fails.
pub async fn join_board(
    state: &AppState,
    board_id: Uuid,
    user_id: Uuid,
    user_name: &str,
    user_color: &str,
    client_id: Uuid,
    tx: mpsc::Sender<Frame>,
) -> Result<Vec<BoardObject>, BoardError> {
    ensure_board_permission(&state.pool, board_id, user_id, BoardPermission::View).await?;
    let can_edit = ensure_board_permission(&state.pool, board_id, user_id, BoardPermission::Edit)
        .await
        .is_ok();
    let can_admin = ensure_board_permission(&state.pool, board_id, user_id, BoardPermission::Admin)
        .await
        .is_ok();

    // Fetch object snapshot outside locks; we'll apply it only if needed.
    let hydration_snapshot = hydrate_objects(&state.pool, board_id).await?;

    let mut boards = state.boards.write().await;
    let board_state = boards.entry(board_id).or_insert_with(BoardState::new);

    // Hydrate from Postgres if this is the first live client for this board.
    if board_state.clients.is_empty() {
        board_state.objects = hydration_snapshot;
        info!(%board_id, count = board_state.objects.len(), "hydrated board from database");
    }

    board_state.clients.insert(client_id, tx);
    board_state.users.insert(
        client_id,
        ConnectedClient {
            user_id,
            user_name: user_name.to_owned(),
            user_color: user_color.to_owned(),
            can_edit,
            can_admin,
        },
    );
    let objects: Vec<BoardObject> = board_state.objects.values().cloned().collect();

    info!(%board_id, %client_id, clients = board_state.clients.len(), "client joined board");
    Ok(objects)
}

/// Leave a board. Removes the client sender. If last client, flushes
/// dirty objects and evicts the board state from memory.
pub async fn part_board(state: &AppState, board_id: Uuid, client_id: Uuid) {
    let mut boards = state.boards.write().await;
    let Some(board_state) = boards.get_mut(&board_id) else {
        return;
    };

    board_state.clients.remove(&client_id);
    board_state.users.remove(&client_id);
    info!(%board_id, %client_id, remaining = board_state.clients.len(), "client left board");

    if board_state.clients.is_empty() {
        // PHASE: HANDLE CLEAN EVICTION FAST PATH
        // WHY: avoid unnecessary I/O when the board has no pending mutations.
        if board_state.dirty.is_empty() {
            boards.remove(&board_id);
            info!(%board_id, "evicted board from memory");
        } else {
            // PHASE: SNAPSHOT DIRTY OBJECTS FOR FINAL FLUSH
            // WHY: perform DB I/O outside the lock and keep dirty flags until
            // the write has actually succeeded.
            let dirty_objects = board_state
                .dirty
                .iter()
                .filter_map(|id| board_state.objects.get(id).cloned())
                .collect::<Vec<_>>();
            let dirty_versions = dirty_objects
                .iter()
                .map(|obj| (obj.id, obj.version))
                .collect::<Vec<_>>();

            // Release lock before writing to Postgres.
            drop(boards);
            let flush_result = flush_objects(&state.pool, &dirty_objects).await;

            // PHASE: ACK OR RETAIN DIRTY FLAGS
            // WHY: clear dirties only when persisted. On error, retain state.
            let mut boards = state.boards.write().await;
            let Some(bs) = boards.get_mut(&board_id) else {
                return;
            };
            if !bs.clients.is_empty() {
                return;
            }

            match flush_result {
                Ok(()) => {
                    clear_flushed_dirty_ids(bs, &dirty_versions);
                    if bs.dirty.is_empty() {
                        boards.remove(&board_id);
                        info!(%board_id, "evicted board from memory");
                    } else {
                        tracing::warn!(
                            %board_id,
                            remaining_dirty = bs.dirty.len(),
                            "retaining board after final flush because newer dirty objects exist"
                        );
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, %board_id, "final flush failed; board retained for retry");
                }
            }
        }
    }
}

fn clear_flushed_dirty_ids(board_state: &mut BoardState, flushed_versions: &[(Uuid, i32)]) {
    for (object_id, flushed_version) in flushed_versions {
        let can_clear = match board_state.objects.get(object_id) {
            Some(current) => current.version == *flushed_version,
            None => true,
        };
        if can_clear {
            board_state.dirty.remove(object_id);
        }
    }
}

/// List currently connected users for a board keyed by connection.
pub async fn list_board_users(state: &AppState, board_id: Uuid) -> Vec<BoardUser> {
    let boards = state.boards.read().await;
    let Some(board_state) = boards.get(&board_id) else {
        return Vec::new();
    };
    board_state
        .users
        .iter()
        .map(|(client_id, user)| BoardUser {
            client_id: *client_id,
            user_id: user.user_id,
            user_name: user.user_name.clone(),
            user_color: user.user_color.clone(),
        })
        .collect()
}

// =============================================================================
// BROADCAST
// =============================================================================

/// Broadcast a frame to all clients in a board, optionally excluding one.
pub async fn broadcast(state: &AppState, board_id: Uuid, frame: &Frame, exclude: Option<Uuid>) {
    let boards = state.boards.read().await;
    let Some(board_state) = boards.get(&board_id) else {
        return;
    };

    for (client_id, tx) in &board_state.clients {
        if exclude == Some(*client_id) {
            continue;
        }
        // Best-effort: if a client's channel is full, skip it.
        let _ = tx.try_send(frame.clone());
    }
}

// =============================================================================
// HELPERS
// =============================================================================

async fn hydrate_objects(pool: &PgPool, board_id: Uuid) -> Result<HashMap<Uuid, BoardObject>, sqlx::Error> {
    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            String,
            f64,
            f64,
            Option<f64>,
            Option<f64>,
            f64,
            i32,
            serde_json::Value,
            Option<Uuid>,
            i32,
        ),
    >(
        "SELECT id, board_id, kind, x, y, width, height, rotation, z_index, props, created_by, version \
         FROM board_objects WHERE board_id = $1",
    )
    .bind(board_id)
    .fetch_all(pool)
    .await?;

    let mut objects = HashMap::new();
    for (id, board_id, kind, x, y, width, height, rotation, z_index, props, created_by, version) in rows {
        objects.insert(
            id,
            BoardObject { id, board_id, kind, x, y, width, height, rotation, z_index, props, created_by, version },
        );
    }
    Ok(objects)
}

/// Batch upsert objects to Postgres.
pub async fn flush_objects(pool: &PgPool, objects: &[BoardObject]) -> Result<(), sqlx::Error> {
    for obj in objects {
        sqlx::query(
            "INSERT INTO board_objects (id, board_id, kind, x, y, width, height, rotation, z_index, props, created_by, version, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, now()) \
             ON CONFLICT (id) DO UPDATE SET \
                 x = EXCLUDED.x, y = EXCLUDED.y, width = EXCLUDED.width, height = EXCLUDED.height, \
                 rotation = EXCLUDED.rotation, z_index = EXCLUDED.z_index, props = EXCLUDED.props, \
                 version = EXCLUDED.version, updated_at = now()",
        )
        .bind(obj.id)
        .bind(obj.board_id)
        .bind(&obj.kind)
        .bind(obj.x)
        .bind(obj.y)
        .bind(obj.width)
        .bind(obj.height)
        .bind(obj.rotation)
        .bind(obj.z_index)
        .bind(&obj.props)
        .bind(obj.created_by)
        .bind(obj.version)
        .execute(pool)
        .await?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "board_test.rs"]
mod tests;
