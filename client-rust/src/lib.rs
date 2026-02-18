//! # client-rust
//!
//! Leptos + WASM frontend for the collaborative whiteboard application.
//! Replaces the React + Konva.js `client/` with a Rust-native UI layer.
//!
//! This crate contains pages, components, application state, network types,
//! and the WebSocket frame client. It integrates with the `canvas` crate
//! for imperative canvas rendering via the `CanvasHost` bridge component.

pub mod components;
pub mod net;
pub mod pages;
pub mod state;
pub mod util;
