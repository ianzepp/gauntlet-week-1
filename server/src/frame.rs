//! Frame — the universal message type for `CollabBoard`.
//!
//! ARCHITECTURE
//! ============
//! Every communication in `CollabBoard` is a Frame. Clients send request frames
//! over WebSocket, the server dispatches by syscall prefix, and responses flow
//! back as item/done/error frames. Ported from Prior's kernel/src/frame.rs with
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
/// Every exchange is `request → item* → done` or `request → error`.
/// No special cases, no "ok" shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Request,
    Item,
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
    pub parent_id: Option<Uuid>,
    /// Milliseconds since Unix epoch. Set automatically at construction.
    pub ts: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub board_id: Option<Uuid>,
    pub from: Option<String>,
    pub syscall: String,
    pub status: Status,
    pub data: Data,
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

    /// Create an item response carrying one result.
    #[must_use]
    pub fn item(&self, data: Data) -> Self {
        self.reply(Status::Item, data)
    }

    /// Create a done response. Terminal, carries no data.
    #[must_use]
    pub fn done(&self) -> Self {
        self.reply(Status::Done, Data::new())
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

    /// Build a reply frame. Inherits `parent_id`, `board_id`, and `syscall`.
    fn reply(&self, status: Status, data: Data) -> Self {
        Self {
            id: Uuid::new_v4(),
            parent_id: Some(self.id),
            ts: now_ms(),
            board_id: self.board_id,
            from: None,
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
mod tests {
    use super::*;

    #[test]
    fn request_sets_fields() {
        let frame = Frame::request("board:create", Data::new());
        assert_eq!(frame.syscall, "board:create");
        assert_eq!(frame.status, Status::Request);
        assert!(frame.parent_id.is_none());
        assert!(frame.board_id.is_none());
        assert!(frame.ts > 0);
    }

    #[test]
    fn reply_inherits_context() {
        let board_id = Uuid::new_v4();
        let req = Frame::request("object:create", Data::new()).with_board_id(board_id);
        let item = req.item(Data::new());

        assert_eq!(item.parent_id, Some(req.id));
        assert_eq!(item.board_id, Some(board_id));
        assert_eq!(item.syscall, "object:create");
        assert_eq!(item.status, Status::Item);
    }

    #[test]
    fn done_is_terminal() {
        assert!(Status::Done.is_terminal());
        assert!(Status::Error.is_terminal());
        assert!(Status::Cancel.is_terminal());
        assert!(!Status::Request.is_terminal());
        assert!(!Status::Item.is_terminal());
    }

    #[test]
    fn prefix_extraction() {
        let frame = Frame::request("object:create", Data::new());
        assert_eq!(frame.prefix(), "object");

        let frame = Frame::request("noseparator", Data::new());
        assert_eq!(frame.prefix(), "noseparator");
    }

    #[test]
    fn json_round_trip() {
        let board_id = Uuid::new_v4();
        let original = Frame::request("board:join", Data::new())
            .with_board_id(board_id)
            .with_from("test-user")
            .with_data("key", "value");

        let json = serde_json::to_string(&original).expect("serialize");
        let restored: Frame = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(restored.id, original.id);
        assert_eq!(restored.board_id, Some(board_id));
        assert_eq!(restored.syscall, "board:join");
        assert_eq!(restored.from.as_deref(), Some("test-user"));
        assert_eq!(restored.data.get("key").and_then(|v| v.as_str()), Some("value"));
    }

    #[test]
    fn error_from_typed() {
        #[derive(Debug, thiserror::Error)]
        #[error("not found")]
        struct NotFound;

        impl ErrorCode for NotFound {
            fn error_code(&self) -> &'static str {
                "E_NOT_FOUND"
            }
        }

        let req = Frame::request("object:get", Data::new());
        let err = req.error_from(&NotFound);

        assert_eq!(err.status, Status::Error);
        assert_eq!(err.data.get("code").and_then(|v| v.as_str()), Some("E_NOT_FOUND"));
        assert_eq!(err.data.get("message").and_then(|v| v.as_str()), Some("not found"));
        assert_eq!(
            err.data
                .get("retryable")
                .and_then(serde_json::Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn cancel_references_target() {
        let req = Frame::request("ai:prompt", Data::new());
        let cancel = Frame::cancel(req.id);

        assert_eq!(cancel.parent_id, Some(req.id));
        assert_eq!(cancel.status, Status::Cancel);
        assert!(cancel.status.is_terminal());
    }
}
