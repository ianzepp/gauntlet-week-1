//! WebSocket frame client for real-time communication with the server.
//!
//! The `FrameClient` manages the WebSocket lifecycle: connection, reconnection
//! with exponential backoff, frame dispatch, and signal updates. It is the
//! primary bridge between the server's frame protocol and the Leptos UI state.
//!
//! Implementation requires the WASM target and will use `gloo-net` WebSocket
//! wrappers. The client runs as a `spawn_local` async task.
