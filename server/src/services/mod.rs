//! Domain services used by websocket and HTTP routes.
//!
//! ARCHITECTURE
//! ============
//! Service modules own business logic and persistence concerns so route
//! handlers can stay focused on protocol translation and auth plumbing.

pub mod ai;
pub mod auth;
pub mod board;
pub mod email_auth;
pub mod object;
pub mod persistence;
pub mod savepoint;
pub mod session;
pub mod tool_syscall;
