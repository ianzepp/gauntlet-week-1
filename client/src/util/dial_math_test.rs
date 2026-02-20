use super::*;

#[test]
fn normalize_degrees_360_wraps_values() {
    assert_eq!(normalize_degrees_360(0.0), 0.0);
    assert_eq!(normalize_degrees_360(370.0), 10.0);
    assert_eq!(normalize_degrees_360(-10.0), 350.0);
}

#[test]
fn signed_angle_delta_picks_shortest_direction() {
    assert_eq!(signed_angle_delta_deg(10.0, 350.0), 20.0);
    assert_eq!(signed_angle_delta_deg(350.0, 10.0), -20.0);
}

#[test]
fn angular_delta_is_symmetric_and_bounded() {
    assert_eq!(angular_delta_deg(10.0, 350.0), 20.0);
    assert_eq!(angular_delta_deg(350.0, 10.0), 20.0);
    assert_eq!(angular_delta_deg(45.0, 45.0), 0.0);
}

#[test]
fn compass_drag_snapping_handles_cardinal_and_shift_grid() {
    assert_eq!(apply_compass_drag_snapping(2.0, false), 0.0);
    assert_eq!(apply_compass_drag_snapping(88.0, false), 90.0);
    assert_eq!(apply_compass_drag_snapping(359.0, false), 0.0);
    assert_eq!(apply_compass_drag_snapping(47.0, true), 45.0);
}

#[test]
fn zoom_tick_tension_pulls_toward_ticks_and_clamps() {
    let adjusted = apply_zoom_tick_tension(88.0);
    assert!(adjusted > 88.0);
    assert!(adjusted < 90.0);
    assert_eq!(apply_zoom_tick_tension(999.0), ZOOM_DIAL_MAX_ANGLE_DEG);
    assert_eq!(apply_zoom_tick_tension(-999.0), ZOOM_DIAL_MIN_ANGLE_DEG);
}

#[test]
fn zoom_angle_mapping_roundtrips_for_common_value() {
    let zoom = 1.35;
    let angle = dial_angle_from_zoom(zoom);
    let back = zoom_from_dial_angle(angle);
    assert!((zoom - back).abs() < 0.0001);
}

#[test]
fn color_shift_angle_mapping_clamps() {
    assert_eq!(dial_angle_from_color_shift(2.0), ZOOM_DIAL_MAX_ANGLE_DEG);
    assert_eq!(dial_angle_from_color_shift(-2.0), ZOOM_DIAL_MIN_ANGLE_DEG);
    assert_eq!(color_shift_from_dial_angle(999.0), 1.0);
    assert_eq!(color_shift_from_dial_angle(-999.0), -1.0);
}

#[test]
fn border_width_conversion_and_formatting_cover_edges() {
    assert_eq!(snap_border_width_to_px(4.49), 4.0);
    assert_eq!(snap_border_width_to_px(4.5), 5.0);
    assert_eq!(snap_border_width_to_px(-5.0), BORDER_WIDTH_MIN);
    assert_eq!(snap_border_width_to_px(99.0), BORDER_WIDTH_MAX);
    assert_eq!(border_width_from_dial_angle(ZOOM_DIAL_MIN_ANGLE_DEG), BORDER_WIDTH_MIN);
    assert_eq!(border_width_from_dial_angle(ZOOM_DIAL_MAX_ANGLE_DEG), BORDER_WIDTH_MAX);
    assert_eq!(format_border_width_label(4.0), "4px");
    assert_eq!(format_border_width_label(4.25), "4.2px");
}

#[test]
fn font_size_conversion_and_formatting_cover_edges() {
    assert_eq!(snap_font_size_to_px(7.4), TEXT_SIZE_MIN);
    assert_eq!(snap_font_size_to_px(200.0), TEXT_SIZE_MAX);
    assert_eq!(font_size_from_dial_angle(ZOOM_DIAL_MIN_ANGLE_DEG), TEXT_SIZE_MIN);
    assert_eq!(font_size_from_dial_angle(ZOOM_DIAL_MAX_ANGLE_DEG), TEXT_SIZE_MAX);
    assert_eq!(format_text_size_label(24.2), "24px");
}
