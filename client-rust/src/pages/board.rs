//! Board page — the main workspace layout.
//!
//! Composes the toolbar, left panel, canvas host, right panel, and status bar
//! in a CSS grid layout. Reads the board ID from the route parameter and
//! triggers `board:join` via the `FrameClient` on mount.
