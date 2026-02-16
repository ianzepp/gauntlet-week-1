//! AI service — LLM prompt → tool calls → board mutations.
//!
//! DESIGN
//! ======
//! Receives an `ai:prompt` frame, sends the board state + user prompt to
//! the LLM with CollabBoard tools, executes returned tool calls as object
//! mutations, and broadcasts results to board peers.
//!
//! This is a stub for post-MVP implementation.

// Placeholder for AI agent integration.
// Will be implemented in Days 4-5 per the PRE-SEARCH timeline.
