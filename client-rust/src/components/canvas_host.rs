//! Bridge component between the Leptos UI and the imperative `canvas::Engine`.
//!
//! Mounts a `<canvas>` element, creates the engine, wires pointer/keyboard
//! events, and synchronizes state between Leptos signals and the engine.
//! This component is client-only (requires WASM/hydration).
