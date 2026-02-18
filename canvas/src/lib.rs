//! Canvas rendering and input engine for the collaborative whiteboard.
//!
//! This crate is compiled to WebAssembly and runs in the browser. It owns the
//! full lifecycle of the canvas: translating raw DOM input events into board
//! mutations, maintaining camera state for pan/zoom, hit-testing objects, and
//! (eventually) rendering the scene. The host JavaScript layer is responsible
//! only for wiring DOM events to the engine and persisting the resulting
//! [`engine::Action`]s to the server.
//!
//! ## Module layout
//!
//! | Module | Role |
//! |--------|------|
//! | [`engine`] | Top-level engine and testable [`engine::EngineCore`] |
//! | [`doc`] | In-memory document store and board object types |
//! | [`camera`] | Pan/zoom camera and coordinate conversions |
//! | [`input`] | Input event types and the gesture state machine |
//! | [`hit`] | Hit-testing against board objects |
//! | [`render`] | Scene rendering (stub — not yet implemented) |
//! | [`consts`] | Shared numeric constants (zoom limits, minimum sizes, etc.) |

pub mod camera;
pub mod consts;
pub mod doc;
pub mod engine;
pub mod hit;
pub mod input;
pub mod render;
