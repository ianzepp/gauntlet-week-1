//! Utility helpers shared across client UI modules.
//!
//! SYSTEM CONTEXT
//! ==============
//! Utility modules isolate browser/environment concerns from page and component
//! logic to improve reuse and testability.

pub mod auth;
pub mod canvas_viewport;
pub mod color;
pub mod dark_mode;
pub mod dial_math;
pub mod frame;
pub mod frame_emit;
pub mod object_props;
pub mod selection_metrics;
pub mod shape_palette;
