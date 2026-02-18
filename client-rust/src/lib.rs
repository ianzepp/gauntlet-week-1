//! # client-rust
//!
//! Leptos + WASM frontend for the collaborative whiteboard application.
//! Replaces the React + Konva.js `client/` with a Rust-native UI layer.
//!
//! This crate contains pages, components, application state, network types,
//! and the WebSocket frame client. It integrates with the `canvas` crate
//! for imperative canvas rendering via the `CanvasHost` bridge component.

pub mod app;
pub mod components;
pub mod net;
pub mod pages;
pub mod state;
pub mod util;

/// WASM hydration entrypoint called by the generated JS glue code.
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(app::App);
}
