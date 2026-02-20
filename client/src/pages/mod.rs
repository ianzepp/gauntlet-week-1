//! Page modules for route-level screens.
//!
//! ARCHITECTURE
//! ============
//! Each page owns route-scoped orchestration and delegates rendering details
//! to `components`.

pub mod board;
pub(crate) mod board_prompt;
pub mod dashboard;
pub mod login;
