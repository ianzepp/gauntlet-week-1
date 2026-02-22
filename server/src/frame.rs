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
/// Exchanges are typically `request → done` or `request → error`.
/// Streaming operations may emit `request → item* → done`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Initial request frame from a client.
    Request,
    /// Intermediate streaming item (non-terminal).
    Item,
    /// Intermediate streaming batch (non-terminal).
    Bulk,
    /// Successful terminal response.
    Done,
    /// Error terminal response.
    Error,
    /// Cancellation signal.
    Cancel,
}

impl From<Status> for frames::Status {
    fn from(value: Status) -> Self {
        match value {
            Status::Request => Self::Request,
            Status::Item => Self::Item,
            Status::Bulk => Self::Bulk,
            Status::Done => Self::Done,
            Status::Error => Self::Error,
            Status::Cancel => Self::Cancel,
        }
    }
}

impl From<frames::Status> for Status {
    fn from(value: frames::Status) -> Self {
        match value {
            frames::Status::Request => Self::Request,
            frames::Status::Item => Self::Item,
            frames::Status::Bulk => Self::Bulk,
            frames::Status::Done => Self::Done,
            frames::Status::Error => Self::Error,
            frames::Status::Cancel => Self::Cancel,
        }
    }
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
    /// Unique identifier for this frame.
    pub id: Uuid,
    /// ID of the request frame this is replying to, if any.
    #[serde(default)]
    pub parent_id: Option<Uuid>,
    /// Milliseconds since Unix epoch. Set automatically at construction.
    #[serde(default)]
    pub ts: i64,
    /// Board this frame belongs to, if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub board_id: Option<Uuid>,
    /// Sender identifier (user ID string or server label).
    #[serde(default)]
    pub from: Option<String>,
    /// Namespaced operation name, e.g. `"object:create"`.
    pub syscall: String,
    /// Lifecycle position of this frame in its request/response stream.
    #[serde(default = "default_status")]
    pub status: Status,
    /// Optional trace metadata attached to this frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace: Option<serde_json::Value>,
    /// Flat key-value payload specific to the syscall.
    #[serde(default)]
    pub data: Data,
}

/// Error returned when converting a [`frames::Frame`] into a server [`Frame`].
#[derive(Debug, thiserror::Error)]
pub enum FrameConvertError {
    /// A UUID field could not be parsed.
    #[error("invalid uuid in field `{field}`: {value}")]
    InvalidUuid { field: &'static str, value: String },
}

fn default_status() -> Status {
    Status::Request
}

// =============================================================================
// ERROR CODES
// =============================================================================

/// Grepable error code and retryable flag for structured error frames.
pub trait ErrorCode: std::fmt::Display {
    /// Short uppercase error code included in the `code` field of error frames.
    fn error_code(&self) -> &'static str;

    /// Whether the client should automatically retry after receiving this error.
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
            trace: None,
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
            trace: None,
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

    /// Create an item response carrying payload data. Non-terminal.
    #[must_use]
    pub fn item_with(&self, data: Data) -> Self {
        self.reply(Status::Item, data)
    }

    /// Create a bulk response carrying payload data. Non-terminal.
    #[must_use]
    pub fn bulk_with(&self, data: Data) -> Self {
        self.reply(Status::Bulk, data)
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
            trace: None,
            data,
        }
    }
}

impl From<&Frame> for frames::Frame {
    fn from(value: &Frame) -> Self {
        let data = serde_json::Value::Object(
            value
                .data
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        );

        Self {
            id: value.id.to_string(),
            parent_id: value.parent_id.map(|v| v.to_string()),
            ts: value.ts,
            board_id: value.board_id.map(|v| v.to_string()),
            from: value.from.clone(),
            syscall: value.syscall.clone(),
            status: value.status.into(),
            trace: value.trace.clone(),
            data,
        }
    }
}

impl TryFrom<frames::Frame> for Frame {
    type Error = FrameConvertError;

    fn try_from(value: frames::Frame) -> Result<Self, Self::Error> {
        let id = value
            .id
            .parse::<Uuid>()
            .map_err(|_| FrameConvertError::InvalidUuid { field: "id", value: value.id.clone() })?;

        let parent_id = match value.parent_id {
            Some(parent) => Some(
                parent
                    .parse::<Uuid>()
                    .map_err(|_| FrameConvertError::InvalidUuid { field: "parent_id", value: parent })?,
            ),
            None => None,
        };

        let board_id = match value.board_id {
            Some(board) => Some(
                board
                    .parse::<Uuid>()
                    .map_err(|_| FrameConvertError::InvalidUuid { field: "board_id", value: board })?,
            ),
            None => None,
        };

        let data = match value.data {
            serde_json::Value::Object(map) => map.into_iter().collect(),
            _ => Data::new(),
        };

        Ok(Self {
            id,
            parent_id,
            ts: value.ts,
            board_id,
            from: value.from,
            syscall: value.syscall,
            status: value.status.into(),
            trace: value.trace,
            data,
        })
    }
}

// =============================================================================
// BUILDERS
// =============================================================================

impl Frame {
    /// Set the `board_id` field on this frame.
    #[must_use]
    pub fn with_board_id(mut self, board_id: Uuid) -> Self {
        self.board_id = Some(board_id);
        self
    }

    /// Set the `from` sender identifier field.
    #[must_use]
    pub fn with_from(mut self, from: impl Into<String>) -> Self {
        self.from = Some(from.into());
        self
    }

    /// Insert a `content` string into the frame payload under [`FRAME_CONTENT`].
    #[must_use]
    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.data
            .insert(FRAME_CONTENT.into(), serde_json::Value::String(content.into()));
        self
    }

    /// Insert an arbitrary key-value pair into the frame payload.
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
