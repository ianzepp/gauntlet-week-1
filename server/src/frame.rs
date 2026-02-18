//! Frame — the universal message type for `CollabBoard`.
//!
//! ARCHITECTURE
//! ============
//! Every communication in `CollabBoard` is a Frame. Clients send request frames
//! over WebSocket, the server dispatches by syscall prefix, and responses flow
//! back as done/error frames. Ported from Prior's kernel/src/frame.rs with
//! `board_id` replacing room.
//!
//! DESIGN
//! ======
//! - Flat data: payload is always `Map<String, Value>`, never nested.
//! - Responses correlate to requests via `parent_id`.
//! - The WS handler routes on `syscall` prefix ("board:", "object:", etc.)
//!   and never inspects `data`.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// =============================================================================
// FIELD CONSTANTS
// =============================================================================

/// Frame data key for error messages.
pub const FRAME_MESSAGE: &str = "message";

/// Frame data key for grepable error codes.
pub const FRAME_CODE: &str = "code";

/// Frame data key for the retryable flag on error frames.
pub const FRAME_RETRYABLE: &str = "retryable";

/// Frame data key for text content (used by `with_content`).
pub const FRAME_CONTENT: &str = "content";

// =============================================================================
// TYPES
// =============================================================================

/// Flat key-value payload. Alias to reduce noise in signatures.
pub type Data = HashMap<String, serde_json::Value>;

/// Lifecycle position of a frame in a request/response stream.
///
/// Every exchange is `request → done` or `request → error`.
/// No special cases, no "ok" shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Request,
    Done,
    Error,
    Cancel,
}

impl Status {
    /// Terminal statuses end a response stream.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Status::Done | Status::Error | Status::Cancel)
    }
}

/// The universal message type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub id: Uuid,
    #[serde(default)]
    pub parent_id: Option<Uuid>,
    /// Milliseconds since Unix epoch. Set automatically at construction.
    #[serde(default)]
    pub ts: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub board_id: Option<Uuid>,
    #[serde(default)]
    pub from: Option<String>,
    pub syscall: String,
    #[serde(default = "default_status")]
    pub status: Status,
    #[serde(default)]
    pub data: Data,
}

fn default_status() -> Status {
    Status::Request
}

// =============================================================================
// ERROR CODES
// =============================================================================

/// Grepable error code and retryable flag for structured error frames.
pub trait ErrorCode: std::fmt::Display {
    fn error_code(&self) -> &'static str;

    fn retryable(&self) -> bool {
        false
    }
}

// =============================================================================
// CONSTRUCTORS
// =============================================================================

/// Current time as milliseconds since Unix epoch.
fn now_ms() -> i64 {
    let Ok(dur) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return 0;
    };
    i64::try_from(dur.as_millis()).unwrap_or(0)
}

impl Frame {
    /// Create a request frame. Entry point for every syscall.
    pub fn request(syscall: impl Into<String>, data: Data) -> Self {
        Self {
            id: Uuid::new_v4(),
            parent_id: None,
            ts: now_ms(),
            board_id: None,
            from: None,
            syscall: syscall.into(),
            status: Status::Request,
            data,
        }
    }

    /// Create a cancel frame targeting a previously submitted request.
    #[must_use]
    pub fn cancel(target_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            parent_id: Some(target_id),
            ts: now_ms(),
            board_id: None,
            from: None,
            syscall: String::new(),
            status: Status::Cancel,
            data: Data::new(),
        }
    }

    /// Create a done response. Terminal, carries no data.
    #[must_use]
    pub fn done(&self) -> Self {
        self.reply(Status::Done, Data::new())
    }

    /// Create a done response carrying payload data. Terminal.
    #[must_use]
    pub fn done_with(&self, data: Data) -> Self {
        self.reply(Status::Done, data)
    }

    /// Create an error response from a plain string. Terminal.
    #[must_use]
    pub fn error(&self, message: impl Into<String>) -> Self {
        let mut data = Data::new();
        data.insert(FRAME_MESSAGE.into(), serde_json::Value::String(message.into()));
        self.reply(Status::Error, data)
    }

    /// Create a structured error response from a typed error. Terminal.
    #[must_use]
    pub fn error_from(&self, err: &(impl ErrorCode + ?Sized)) -> Self {
        let mut data = Data::new();
        data.insert(FRAME_CODE.into(), serde_json::Value::String(err.error_code().to_string()));
        data.insert(FRAME_MESSAGE.into(), serde_json::Value::String(err.to_string()));
        data.insert(FRAME_RETRYABLE.into(), serde_json::Value::Bool(err.retryable()));
        self.reply(Status::Error, data)
    }

    /// Build a reply frame. Inherits `parent_id`, `board_id`, `from`, and `syscall`.
    fn reply(&self, status: Status, data: Data) -> Self {
        Self {
            id: Uuid::new_v4(),
            parent_id: Some(self.id),
            ts: now_ms(),
            board_id: self.board_id,
            from: self.from.clone(),
            syscall: self.syscall.clone(),
            status,
            data,
        }
    }
}

// =============================================================================
// BUILDERS
// =============================================================================

impl Frame {
    #[must_use]
    pub fn with_board_id(mut self, board_id: Uuid) -> Self {
        self.board_id = Some(board_id);
        self
    }

    #[must_use]
    pub fn with_from(mut self, from: impl Into<String>) -> Self {
        self.from = Some(from.into());
        self
    }

    #[must_use]
    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.data
            .insert(FRAME_CONTENT.into(), serde_json::Value::String(content.into()));
        self
    }

    #[must_use]
    pub fn with_data(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.data.insert(key.into(), value.into());
        self
    }
}

// =============================================================================
// ROUTING
// =============================================================================

impl Frame {
    /// Extract the syscall prefix (everything before the first ':').
    #[must_use]
    pub fn prefix(&self) -> &str {
        let Some((prefix, _)) = self.syscall.split_once(':') else {
            return &self.syscall;
        };
        prefix
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
#[path = "frame_test.rs"]
mod tests;
