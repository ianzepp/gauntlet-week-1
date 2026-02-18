//! Rendering: draws the full canvas scene to a 2D context.
//!
//! This module is the only place that touches [`web_sys::CanvasRenderingContext2d`].
//! It receives read-only views of document state and camera state and produces
//! pixels — it does not mutate any application state.

use web_sys::CanvasRenderingContext2d;

use crate::camera::Camera;
use crate::doc::DocStore;
use crate::input::UiState;

/// Draw the full scene: grid, objects, selection UI.
///
/// Not yet implemented — this is the rendering entry point.
pub fn draw(_ctx: &CanvasRenderingContext2d, _doc: &DocStore, _camera: &Camera, _ui: &UiState) {
    todo!()
}
