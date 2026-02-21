use super::*;

#[test]
fn canvas_view_state_defaults_are_neutral() {
    let state = CanvasViewState::default();
    assert!(state.cursor_world.is_none());
    assert_eq!(state.camera_center_world.x, 0.0);
    assert_eq!(state.camera_center_world.y, 0.0);
    assert_eq!(state.zoom, 1.0);
    assert!(state.fps.is_none());
    assert!(state.fps_last_sample_ms.is_none());
    assert!(state.last_render_ms.is_none());
    assert_eq!(state.pan_x, 0.0);
    assert_eq!(state.pan_y, 0.0);
    assert_eq!(state.view_rotation_deg, 0.0);
    assert_eq!(state.viewport_width, 0.0);
    assert_eq!(state.viewport_height, 0.0);
}
