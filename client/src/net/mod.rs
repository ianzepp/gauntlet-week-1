//! Networking modules for HTTP + websocket frame protocol.
//!
//! SYSTEM CONTEXT
//! ==============
//! `api` handles REST calls, `frame_client` manages the websocket lifecycle,
//! and `types` defines the shared wire schema.

pub mod api;
pub mod frame_client;
pub mod types;
