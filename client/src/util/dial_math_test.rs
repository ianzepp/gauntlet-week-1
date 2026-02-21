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

#[test]
fn signed_angle_delta_deg_non_finite_returns_zero() {
    assert_eq!(signed_angle_delta_deg(f64::INFINITY, 0.0), 0.0);
    assert_eq!(signed_angle_delta_deg(f64::NEG_INFINITY, 0.0), 0.0);
    assert_eq!(signed_angle_delta_deg(f64::NAN, 0.0), 0.0);
    assert_eq!(signed_angle_delta_deg(0.0, f64::NAN), 0.0);
    assert_eq!(signed_angle_delta_deg(f64::INFINITY, f64::INFINITY), 0.0);
}

#[test]
fn border_width_angle_roundtrip() {
    for width in [0, 1, 5, 12, 24] {
        let w = width as f64;
        let angle = dial_angle_from_border_width(w);
        let back = border_width_from_dial_angle(angle);
        assert!((w - back).abs() < 1.0, "border width {w} roundtripped to {back}");
    }
}

#[test]
fn font_size_angle_roundtrip() {
    for size in [8, 12, 24, 48, 96] {
        let s = size as f64;
        let angle = dial_angle_from_font_size(s);
        let back = font_size_from_dial_angle(angle);
        assert!((s - back).abs() < 1.0, "font size {s} roundtripped to {back}");
    }
}

#[test]
fn compass_snap_at_exact_epsilon_boundary() {
    // 6.0 degrees from cardinal 0 should snap to 0
    let snapped = apply_compass_drag_snapping(6.0, false);
    assert_eq!(snapped, 0.0);

    // 6.0 degrees from cardinal 90 should snap to 90
    let snapped = apply_compass_drag_snapping(84.0, false);
    assert_eq!(snapped, 90.0);

    // 6.0 degrees from 270 should snap to 270
    let snapped = apply_compass_drag_snapping(276.0, false);
    assert_eq!(snapped, 270.0);
}
