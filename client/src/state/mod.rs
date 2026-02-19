//! Shared client-side state modules.
//!
//! DESIGN
//! ======
//! State is split by domain (`auth`, `board`, `chat`, etc.) so individual
//! components can depend on small focused models.

pub mod ai;
pub mod auth;
pub mod board;
pub mod boards;
pub mod canvas_view;
pub mod chat;
pub mod ui;
