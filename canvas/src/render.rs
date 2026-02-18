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
