//! Shared helpers for constructing outbound request frames.
//!
//! SYSTEM CONTEXT
//! ==============
//! Multiple UI surfaces emit syscall frames over websocket. Centralizing the
//! base request envelope prevents drift across call sites.

use crate::net::types::{Frame, FrameStatus};

/// Build a request frame with standard client metadata.
pub fn request_frame(syscall: &str, board_id: Option<String>, data: serde_json::Value) -> Frame {
    Frame {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        ts: 0,
        board_id,
        from: None,
        syscall: syscall.to_owned(),
        status: FrameStatus::Request,
        data,
    }
}
