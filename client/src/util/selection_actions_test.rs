use super::*;

#[test]
fn selection_scale_multiplier_clamps_target_and_handles_zero_start() {
    assert_eq!(selection_scale_multiplier(2.0, 1.0), 2.0);
    assert_eq!(selection_scale_multiplier(999.0, 2.0), 5.0);
    assert_eq!(selection_scale_multiplier(0.01, 1.0), 0.1);
    assert_eq!(selection_scale_multiplier(2.0, 0.0), 1.0);
}

#[test]
fn selection_geometry_changed_uses_deadband_thresholds() {
    assert!(!selection_geometry_changed(10.0, 20.0, 30.0, 40.0, 10.0, 20.0, 30.0, 40.0));
    assert!(!selection_geometry_changed(10.009, 20.0, 30.0, 40.0, 10.0, 20.0, 30.0, 40.0));
    assert!(selection_geometry_changed(10.02, 20.0, 30.0, 40.0, 10.0, 20.0, 30.0, 40.0));
    assert!(selection_geometry_changed(10.0, 20.0, 30.0, 40.02, 10.0, 20.0, 30.0, 40.0));
}

#[test]
fn selection_color_changed_detects_fill_base_and_shift_changes() {
    assert!(!selection_color_changed("#111111", "#222222", 0.1, "#111111", "#222222", 0.1));
    assert!(selection_color_changed("#111112", "#222222", 0.1, "#111111", "#222222", 0.1));
    assert!(selection_color_changed("#111111", "#222223", 0.1, "#111111", "#222222", 0.1));
    assert!(selection_color_changed("#111111", "#222222", 0.102, "#111111", "#222222", 0.1));
}

#[test]
fn selection_border_changed_detects_color_or_width_deltas() {
    assert!(!selection_border_changed("#111111", 2.0, "#111111", 2.0));
    assert!(selection_border_changed("#222222", 2.0, "#111111", 2.0));
    assert!(selection_border_changed("#111111", 2.01, "#111111", 2.0));
}

#[test]
fn selection_text_style_changed_detects_color_or_size_deltas() {
    assert!(!selection_text_style_changed("#111111", 18.0, "#111111", 18.0));
    assert!(selection_text_style_changed("#222222", 18.0, "#111111", 18.0));
    assert!(selection_text_style_changed("#111111", 18.01, "#111111", 18.0));
}

#[test]
fn representative_scale_from_values_returns_mean_or_default() {
    assert_eq!(representative_scale_from_values(&[]), 1.0);
    assert_eq!(representative_scale_from_values(&[1.0, 2.0, 3.0]), 2.0);
}
