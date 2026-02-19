//! Shared wire-protocol DTOs for the client/server boundary.
//!
//! DESIGN
//! ======
//! These types intentionally mirror server frame payloads so serde round-trips
//! stay lossless and websocket dispatch code can remain schema-driven.

#[cfg(test)]
#[path = "types_test.rs"]
mod types_test;

use serde::{Deserialize, Serialize};

pub use frames::Frame;
pub use frames::Status as FrameStatus;

/// A board object as represented in the wire protocol.
/// The `canvas` crate has its own `BoardObject` type; the `CanvasHost` bridge
/// converts between the two.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoardObject {
    pub id: String,
    pub board_id: String,
    pub kind: String,
    pub x: f64,
    pub y: f64,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub rotation: f64,
    pub z_index: i32,
    pub props: serde_json::Value,
    pub created_by: Option<String>,
    pub version: i64,
}

/// Persisted board savepoint with full snapshot for preview/rewind.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Savepoint {
    pub id: String,
    pub board_id: String,
    pub seq: i64,
    pub ts: i64,
    pub created_by: Option<String>,
    pub is_auto: bool,
    pub reason: String,
    pub label: Option<String>,
    #[serde(default)]
    pub snapshot: Vec<BoardObject>,
}

/// Presence information for a connected user on a board.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Presence {
    pub client_id: String,
    pub user_id: String,
    pub name: String,
    pub color: String,
    pub cursor: Option<Point>,
    pub camera_center: Option<Point>,
    pub camera_zoom: Option<f64>,
}

/// A 2D point.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// An authenticated user.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub color: String,
}

/// Extended user profile with statistics.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub color: String,
    pub member_since: Option<String>,
    pub stats: ProfileStats,
}

/// Aggregate statistics for a user profile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProfileStats {
    pub total_frames: i64,
    pub objects_created: i64,
    pub boards_active: i64,
    pub last_active: Option<String>,
    pub top_syscalls: Vec<SyscallCount>,
}

/// A syscall name and its invocation count.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyscallCount {
    pub syscall: String,
    pub count: i64,
}
