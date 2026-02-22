//! Shared wire-protocol DTOs for the client/server boundary.
//!
//! DESIGN
//! ======
//! These types intentionally mirror server frame payloads so serde round-trips
//! stay lossless and websocket dispatch code can remain schema-driven.

#[cfg(test)]
#[path = "types_test.rs"]
mod types_test;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

pub use frames::Frame;
pub use frames::Status as FrameStatus;

/// A board object as represented in the wire protocol.
///
/// The `canvas` crate has its own `BoardObject` type; the `CanvasHost` bridge
/// converts between the two.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoardObject {
    /// Unique object identifier (UUID string).
    pub id: String,
    /// Board this object belongs to (UUID string).
    pub board_id: String,
    /// Shape or edge type (e.g. `"rectangle"`, `"arrow"`).
    pub kind: String,
    /// Left edge in world coordinates.
    pub x: f64,
    /// Top edge in world coordinates.
    pub y: f64,
    /// Bounding-box width in world coordinates.
    pub width: Option<f64>,
    /// Bounding-box height in world coordinates.
    pub height: Option<f64>,
    /// Clockwise rotation in degrees.
    pub rotation: f64,
    /// Stacking order; lower values are drawn beneath higher values.
    #[serde(deserialize_with = "deserialize_i32_from_number")]
    pub z_index: i32,
    /// Open-ended per-kind properties (fill, stroke, text, endpoints, etc.).
    pub props: serde_json::Value,
    /// User who created the object (UUID string), if known.
    pub created_by: Option<String>,
    /// Monotonically increasing edit counter for conflict detection.
    #[serde(deserialize_with = "deserialize_i64_from_number")]
    pub version: i64,
    /// Optional group membership ID (UUID string).
    pub group_id: Option<String>,
}

/// Persisted board savepoint with full snapshot for preview/rewind.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Savepoint {
    /// Unique savepoint identifier (UUID string).
    pub id: String,
    /// Board this savepoint belongs to (UUID string).
    pub board_id: String,
    /// Global frame sequence number at the time this savepoint was created.
    #[serde(deserialize_with = "deserialize_i64_from_number")]
    pub seq: i64,
    /// Creation timestamp in milliseconds since the Unix epoch.
    #[serde(deserialize_with = "deserialize_i64_from_number")]
    pub ts: i64,
    /// User who triggered the savepoint (UUID string), if user-initiated.
    pub created_by: Option<String>,
    /// Whether this savepoint was created automatically vs. manually.
    pub is_auto: bool,
    /// Short machine-readable reason (e.g. `"auto"`, `"manual"`).
    pub reason: String,
    /// Optional human-readable label shown in the rewind UI.
    pub label: Option<String>,
    /// Full board snapshot as a list of objects.
    #[serde(default)]
    pub snapshot: Vec<BoardObject>,
}

/// Presence information for a connected user on a board.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Presence {
    /// WebSocket client identifier (UUID string).
    pub client_id: String,
    /// Authenticated user identifier (UUID string).
    pub user_id: String,
    /// Display name.
    pub name: String,
    /// Assigned presence color (hex).
    pub color: String,
    /// Last known cursor position in world coordinates, if available.
    pub cursor: Option<Point>,
    /// Center of the user's visible viewport in world coordinates, if shared.
    pub camera_center: Option<Point>,
    /// Current zoom level of the user's camera, if shared.
    pub camera_zoom: Option<f64>,
    /// Current view rotation of the user's camera in degrees, if shared.
    pub camera_rotation: Option<f64>,
}

/// A 2D point in world or screen space.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Point {
    /// Horizontal coordinate.
    pub x: f64,
    /// Vertical coordinate.
    pub y: f64,
}

/// An authenticated user as returned by the `/api/auth/me` endpoint.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct User {
    /// Unique user identifier (UUID string).
    pub id: String,
    /// Display name.
    pub name: String,
    /// Avatar image URL, if available.
    pub avatar_url: Option<String>,
    /// Assigned presence color (hex).
    pub color: String,
    /// Authentication method used to create the session (e.g. `"github"`, `"email"`).
    #[serde(default = "default_auth_method")]
    pub auth_method: String,
}

fn default_auth_method() -> String {
    "session".to_owned()
}

/// Extended user profile with statistics.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserProfile {
    /// Unique user identifier (UUID string).
    pub id: String,
    /// Display name.
    pub name: String,
    /// Avatar image URL, if available.
    pub avatar_url: Option<String>,
    /// Assigned presence color (hex).
    pub color: String,
    /// ISO 8601 date string of the user's first session, if available.
    pub member_since: Option<String>,
    /// Aggregated usage statistics.
    pub stats: ProfileStats,
}

/// Aggregate statistics for a user profile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProfileStats {
    /// Total number of frames sent by this user across all sessions.
    #[serde(deserialize_with = "deserialize_i64_from_number")]
    pub total_frames: i64,
    /// Total number of board objects created by this user.
    #[serde(deserialize_with = "deserialize_i64_from_number")]
    pub objects_created: i64,
    /// Number of distinct boards this user has been active on.
    #[serde(deserialize_with = "deserialize_i64_from_number")]
    pub boards_active: i64,
    /// ISO 8601 timestamp of the most recent frame sent, if available.
    pub last_active: Option<String>,
    /// Most frequently used syscalls, sorted by count descending.
    pub top_syscalls: Vec<SyscallCount>,
}

/// A syscall name and its invocation count.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyscallCount {
    /// Namespaced syscall name (e.g. `"object:create"`).
    pub syscall: String,
    /// Number of times this syscall was invoked.
    #[serde(deserialize_with = "deserialize_i64_from_number")]
    pub count: i64,
}

fn deserialize_i32_from_number<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    let value = deserialize_i64_from_number(deserializer)?;
    i32::try_from(value).map_err(|_| D::Error::custom(format!("value {value} out of range for i32")))
}

fn deserialize_i64_from_number<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Number(number) => {
            if let Some(int) = number.as_i64() {
                return Ok(int);
            }
            #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
            if let Some(float) = number.as_f64()
                && float.is_finite()
                && float.fract() == 0.0
                && float >= i64::MIN as f64
                && float <= i64::MAX as f64
            {
                return Ok(float as i64);
            }
            Err(D::Error::custom("expected integer-compatible number"))
        }
        _ => Err(D::Error::custom("expected number")),
    }
}
