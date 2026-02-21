//! Selection drag/apply/commit helpers used by canvas host.
//!
//! Every interactive dial or control that modifies the selected objects follows a three-phase
//! lifecycle:
//!
//! 1. **Seed** — snapshot the pre-drag values of all selected objects into a drag-state struct.
//! 2. **Apply** — on each pointer-move, compute the new target value and write it directly into
//!    the reactive `BoardState`, keeping the canvas in sync without a server round-trip.
//! 3. **Commit** — on pointer-up, compare the current values to the snapshotted seed values and
//!    emit `object:update` frames only for objects that actually changed.
//!
//! Separating apply from commit prevents flooding the server with update frames during a drag
//! while still delivering a single authoritative update at the end.

#[cfg(test)]
#[path = "selection_actions_test.rs"]
mod selection_actions_test;

use leptos::prelude::*;

use crate::app::FrameSender;
use crate::state::board::BoardState;
#[cfg(feature = "hydrate")]
use crate::util::color::normalize_hex_color;
#[cfg(feature = "hydrate")]
use crate::util::dial_math::{BORDER_WIDTH_MAX, BORDER_WIDTH_MIN, snap_font_size_to_px};
#[cfg(feature = "hydrate")]
use crate::util::frame_emit::{send_object_update_geometry, send_object_update_props, send_object_update_rotation};
#[cfg(feature = "hydrate")]
use crate::util::object_props::{
    object_base_fill_hex, object_border_color_hex, object_border_width, object_fill_hex, object_font_size,
    object_lightness_shift, object_scale_components, object_text_color_hex, upsert_object_border_props,
    upsert_object_color_props, upsert_object_scale_props, upsert_object_text_style_props,
};
#[cfg(feature = "hydrate")]
use crate::util::selection_metrics::representative_rotation_deg;

/// Compute the per-object scale multiplier needed to move the group from `start_group_scale`
/// to `target_scale`.
///
/// Returns 1.0 (no change) when `start_group_scale` is zero to avoid division by zero.
/// The result is the ratio `target / start` and must be applied individually to each object's
/// own `start_scale` to produce that object's new absolute scale.
#[cfg(any(test, feature = "hydrate"))]
fn selection_scale_multiplier(target_scale: f64, start_group_scale: f64) -> f64 {
    let target_scale = target_scale.clamp(0.1, 10.0);
    if start_group_scale.abs() < f64::EPSILON {
        1.0
    } else {
        target_scale / start_group_scale
    }
}

/// Return whether an object's geometry changed by more than the commit threshold (0.01 units).
///
/// Used by commit functions to skip emitting frames for objects whose position or size did not
/// meaningfully change during a drag, avoiding unnecessary server traffic.
#[cfg(any(test, feature = "hydrate"))]
fn selection_geometry_changed(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    start_x: f64,
    start_y: f64,
    start_width: f64,
    start_height: f64,
) -> bool {
    (x - start_x).abs() > 0.01
        || (y - start_y).abs() > 0.01
        || (width - start_width).abs() > 0.01
        || (height - start_height).abs() > 0.01
}

/// Return whether any of the three color parameters changed beyond noise thresholds.
///
/// String equality is used for the hex colors; `shift` uses a 0.001 tolerance to absorb
/// floating-point rounding from the dial-to-shift conversion.
#[cfg(any(test, feature = "hydrate"))]
fn selection_color_changed(
    fill: &str,
    base_fill: &str,
    shift: f64,
    start_fill: &str,
    start_base_fill: &str,
    start_shift: f64,
) -> bool {
    fill != start_fill || base_fill != start_base_fill || (shift - start_shift).abs() > 0.001
}

/// Return whether either border parameter changed beyond noise thresholds.
///
/// Width uses a 0.001 tolerance to absorb dial-to-pixel rounding.
#[cfg(any(test, feature = "hydrate"))]
fn selection_border_changed(color: &str, width: f64, start_color: &str, start_width: f64) -> bool {
    color != start_color || (width - start_width).abs() > 0.001
}

/// Return whether either text-style parameter changed beyond noise thresholds.
///
/// Size uses a 0.001 tolerance to absorb dial-to-pixel rounding.
#[cfg(any(test, feature = "hydrate"))]
fn selection_text_style_changed(color: &str, size: f64, start_color: &str, start_size: f64) -> bool {
    color != start_color || (size - start_size).abs() > 0.001
}

/// Compute the arithmetic mean of a slice of scale values.
///
/// Returns 1.0 for an empty slice so that no-selection callers receive a neutral default.
#[cfg(any(test, feature = "hydrate"))]
fn representative_scale_from_values(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 1.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

/// Per-object snapshot captured at the start of a rotation drag.
///
/// `start_pointer_angle_deg` is the compass angle of the pointer at drag start, used as the
/// reference from which all subsequent pointer angles are measured. `start_rotations` maps each
/// object ID to its rotation at drag start, so that incremental deltas are applied from a
/// stable baseline rather than accumulated frame-to-frame.
#[cfg(feature = "hydrate")]
#[derive(Clone)]
pub struct SelectionRotationDragState {
    /// Compass angle of the pointer (degrees) at the moment the drag began.
    pub start_pointer_angle_deg: f64,
    /// `(object_id, rotation_deg)` for each selected object at drag start.
    pub start_rotations: Vec<(String, f64)>,
}

/// Per-object snapshot captured at the start of a scale drag.
///
/// Stores enough geometry to reconstruct both the new size (via `base_width * new_scale`)
/// and the new position (via proportional repositioning around the group center).
#[cfg(feature = "hydrate")]
#[derive(Clone)]
pub struct SelectionScaleSeed {
    /// Object identifier.
    pub id: String,
    /// Board this object belongs to, needed for the update frame.
    pub board_id: String,
    /// Object version at seed time, used for optimistic concurrency on update frames.
    pub version: i64,
    /// Reference width this object was designed at (from `"baseWidth"` prop).
    pub base_width: f64,
    /// Reference height this object was designed at (from `"baseHeight"` prop).
    pub base_height: f64,
    /// Scale factor at drag start.
    pub start_scale: f64,
    /// Object x position at drag start.
    pub x: f64,
    /// Object y position at drag start.
    pub y: f64,
    /// Object width at drag start.
    pub width: f64,
    /// Object height at drag start.
    pub height: f64,
}

/// Group-level drag state for a scale operation across the full selection.
///
/// `group_center_x` / `group_center_y` is the bounding-box centre of all selected objects at
/// drag start. Repositioning each object proportionally relative to this centre is what keeps
/// the selection visually stable while scaling — objects move outward from the centre as they
/// grow and inward as they shrink.
#[cfg(feature = "hydrate")]
#[derive(Clone)]
pub struct SelectionScaleDragState {
    /// Per-object snapshots for all selected objects.
    pub start_items: Vec<SelectionScaleSeed>,
    /// Bounding-box centre X of the selection at drag start (world units).
    pub group_center_x: f64,
    /// Bounding-box centre Y of the selection at drag start (world units).
    pub group_center_y: f64,
    /// Representative scale of the selection at drag start, used to compute the multiplier.
    pub start_group_scale: f64,
}

/// Per-object snapshot captured at the start of a color drag.
///
/// Preserves the base fill and lightness shift independently so that the drag applies the new
/// shift against the *original* base rather than accumulating shifts.
#[cfg(feature = "hydrate")]
#[derive(Clone)]
pub struct SelectionColorSeed {
    /// Object identifier.
    pub id: String,
    /// Board this object belongs to.
    pub board_id: String,
    /// Object version at seed time.
    pub version: i64,
    /// Displayed fill color (after lightness shift) at drag start.
    pub start_fill: String,
    /// Base fill color (before lightness shift) at drag start.
    pub start_base_fill: String,
    /// Lightness shift value at drag start, in [-1, 1].
    pub start_lightness_shift: f64,
}

/// Group-level drag state for a color-shift operation across the full selection.
#[cfg(feature = "hydrate")]
#[derive(Clone)]
pub struct SelectionColorDragState {
    /// Per-object snapshots for all selected objects.
    pub start_items: Vec<SelectionColorSeed>,
}

/// Per-object snapshot captured at the start of a border drag.
#[cfg(feature = "hydrate")]
#[derive(Clone)]
pub struct SelectionBorderSeed {
    /// Object identifier.
    pub id: String,
    /// Board this object belongs to.
    pub board_id: String,
    /// Object version at seed time.
    pub version: i64,
    /// Border color at drag start.
    pub start_border_color: String,
    /// Border width in pixels at drag start.
    pub start_border_width: f64,
}

/// Group-level drag state for a border width/color operation across the full selection.
#[cfg(feature = "hydrate")]
#[derive(Clone)]
pub struct SelectionBorderDragState {
    /// Per-object snapshots for all selected objects.
    pub start_items: Vec<SelectionBorderSeed>,
}

/// Per-object snapshot captured at the start of a text-style drag.
#[cfg(feature = "hydrate")]
#[derive(Clone)]
pub struct SelectionTextStyleSeed {
    /// Object identifier.
    pub id: String,
    /// Board this object belongs to.
    pub board_id: String,
    /// Object version at seed time.
    pub version: i64,
    /// Text color at drag start.
    pub start_text_color: String,
    /// Font size in pixels at drag start.
    pub start_font_size: f64,
}

/// Group-level drag state for a text-style operation across the full selection.
#[cfg(feature = "hydrate")]
#[derive(Clone)]
pub struct SelectionTextStyleDragState {
    /// Per-object snapshots for all selected objects.
    pub start_items: Vec<SelectionTextStyleSeed>,
}

/// Return `true` if the board has at least one selected object.
#[cfg(feature = "hydrate")]
pub fn has_selection(board: RwSignal<BoardState>) -> bool {
    !board.get_untracked().selection.is_empty()
}

/// Collect `(object_id, rotation_deg)` pairs for all currently selected objects.
///
/// Used to seed a [`SelectionRotationDragState`] before a rotation drag begins.
#[cfg(feature = "hydrate")]
pub fn selected_object_rotations(board: RwSignal<BoardState>) -> Vec<(String, f64)> {
    let state = board.get_untracked();
    state
        .selection
        .iter()
        .filter_map(|id| state.objects.get(id).map(|obj| (id.clone(), obj.rotation)))
        .collect()
}

/// Snapshot all selected objects into a [`SelectionScaleDragState`] ready for scale dragging.
///
/// Computes the bounding box of the whole selection to obtain the group centre, which is used
/// during [`apply_selection_scale_drag`] to reposition each object proportionally as the group
/// scales. Returns `None` when nothing is selected.
#[cfg(feature = "hydrate")]
pub fn selection_scale_seed(board: RwSignal<BoardState>) -> Option<SelectionScaleDragState> {
    let state = board.get_untracked();
    let mut items: Vec<SelectionScaleSeed> = Vec::new();
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for id in &state.selection {
        let Some(obj) = state.objects.get(id) else {
            continue;
        };
        let width = obj.width.unwrap_or(120.0).max(1.0);
        let height = obj.height.unwrap_or(80.0).max(1.0);
        let (base_width, base_height, start_scale) = object_scale_components(obj, width, height);
        let x = obj.x;
        let y = obj.y;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x + width);
        max_y = max_y.max(y + height);
        items.push(SelectionScaleSeed {
            id: obj.id.clone(),
            board_id: obj.board_id.clone(),
            version: obj.version,
            base_width,
            base_height,
            start_scale,
            x,
            y,
            width,
            height,
        });
    }
    if items.is_empty() {
        return None;
    }
    let start_group_scale = selection_representative_scale_from_items(&items);
    Some(SelectionScaleDragState {
        start_items: items,
        group_center_x: (min_x + max_x) * 0.5,
        group_center_y: (min_y + max_y) * 0.5,
        start_group_scale,
    })
}

/// Apply an in-progress scale drag to all selected objects.
///
/// For each object the new scale is `seed.start_scale * multiplier`, where `multiplier` is the
/// ratio of `target_scale` to the group's starting scale. Each object's centre is repositioned
/// proportionally relative to the group centre so the group expands/contracts uniformly.
/// Writes changes directly into `board` without emitting any frames.
#[cfg(feature = "hydrate")]
pub fn apply_selection_scale_drag(
    board: RwSignal<BoardState>,
    drag_state_signal: RwSignal<Option<SelectionScaleDragState>>,
    target_scale: f64,
) {
    let Some(drag_state) = drag_state_signal.get_untracked() else {
        return;
    };
    let multiplier = selection_scale_multiplier(target_scale, drag_state.start_group_scale);
    board.update(|b| {
        for seed in &drag_state.start_items {
            let Some(obj) = b.objects.get_mut(&seed.id) else {
                continue;
            };
            let start_cx = seed.x + (seed.width * 0.5);
            let start_cy = seed.y + (seed.height * 0.5);
            let next_scale = (seed.start_scale * multiplier).clamp(0.1, 10.0);
            let new_w = (seed.base_width * next_scale).max(1.0);
            let new_h = (seed.base_height * next_scale).max(1.0);
            let new_cx = drag_state.group_center_x + ((start_cx - drag_state.group_center_x) * multiplier);
            let new_cy = drag_state.group_center_y + ((start_cy - drag_state.group_center_y) * multiplier);
            obj.width = Some(new_w);
            obj.height = Some(new_h);
            obj.x = new_cx - (new_w * 0.5);
            obj.y = new_cy - (new_h * 0.5);
            upsert_object_scale_props(obj, next_scale, seed.base_width, seed.base_height);
        }
    });
}

/// Commit scale changes to the server for objects that actually moved or resized.
///
/// Compares each object's current geometry to its seeded start values, skipping objects that
/// did not meaningfully change. Clears the drag state signal after committing.
#[cfg(feature = "hydrate")]
pub fn commit_selection_scale_updates(
    board: RwSignal<BoardState>,
    sender: RwSignal<FrameSender>,
    drag_state_signal: RwSignal<Option<SelectionScaleDragState>>,
) {
    let Some(drag_state) = drag_state_signal.get_untracked() else {
        return;
    };
    let state = board.get_untracked();
    for seed in &drag_state.start_items {
        let Some(obj) = state.objects.get(&seed.id) else {
            continue;
        };
        let changed = selection_geometry_changed(
            obj.x,
            obj.y,
            obj.width.unwrap_or(seed.width),
            obj.height.unwrap_or(seed.height),
            seed.x,
            seed.y,
            seed.width,
            seed.height,
        );
        if !changed {
            continue;
        }
        send_object_update_geometry(
            sender,
            &seed.board_id,
            &seed.id,
            seed.version,
            obj.x,
            obj.y,
            obj.width.unwrap_or(seed.width),
            obj.height.unwrap_or(seed.height),
            &obj.props,
        );
    }
    drag_state_signal.set(None);
}

/// Set all selected objects to an absolute `target_scale` and immediately commit to the server.
///
/// Unlike the drag path, this bypasses the seed/apply/commit cycle and emits update frames
/// synchronously. Used by numeric input controls where the user types an exact value.
#[cfg(feature = "hydrate")]
pub fn apply_group_scale_target(board: RwSignal<BoardState>, sender: RwSignal<FrameSender>, target_scale: f64) {
    let target_scale = target_scale.clamp(0.1, 10.0);
    let state = board.get_untracked();
    let selected: Vec<String> = state
        .selection
        .iter()
        .filter(|id| state.objects.contains_key(*id))
        .cloned()
        .collect();
    if selected.is_empty() {
        return;
    }
    board.update(|b| {
        for id in &selected {
            let Some(obj) = b.objects.get_mut(id) else {
                continue;
            };
            let width = obj.width.unwrap_or(120.0).max(1.0);
            let height = obj.height.unwrap_or(80.0).max(1.0);
            let (base_width, base_height, _current_scale) = object_scale_components(obj, width, height);
            let cx = obj.x + (width * 0.5);
            let cy = obj.y + (height * 0.5);
            let new_w = (base_width * target_scale).max(1.0);
            let new_h = (base_height * target_scale).max(1.0);
            obj.width = Some(new_w);
            obj.height = Some(new_h);
            obj.x = cx - (new_w * 0.5);
            obj.y = cy - (new_h * 0.5);
            upsert_object_scale_props(obj, target_scale, base_width, base_height);
        }
    });

    let post = board.get_untracked();
    for id in selected {
        let Some(obj) = post.objects.get(&id) else {
            continue;
        };
        send_object_update_geometry(
            sender,
            &obj.board_id,
            &obj.id,
            obj.version,
            obj.x,
            obj.y,
            obj.width.unwrap_or(120.0),
            obj.height.unwrap_or(80.0),
            &obj.props,
        );
    }
}

/// No-op stub used on SSR builds where the hydrate feature is absent.
#[cfg(not(feature = "hydrate"))]
pub fn apply_group_scale_target(_board: RwSignal<BoardState>, _sender: RwSignal<FrameSender>, _target_scale: f64) {}

/// Snapshot all selected objects into a [`SelectionColorDragState`] ready for color dragging.
///
/// Records the current fill, base fill, and lightness shift for each object so that the drag
/// applies shifts against a stable baseline. Returns `None` when nothing is selected.
#[cfg(feature = "hydrate")]
pub fn selection_color_seed(board: RwSignal<BoardState>) -> Option<SelectionColorDragState> {
    let state = board.get_untracked();
    let mut items = Vec::new();
    for id in &state.selection {
        let Some(obj) = state.objects.get(id) else {
            continue;
        };
        items.push(SelectionColorSeed {
            id: obj.id.clone(),
            board_id: obj.board_id.clone(),
            version: obj.version,
            start_fill: object_fill_hex(obj),
            start_base_fill: object_base_fill_hex(obj),
            start_lightness_shift: object_lightness_shift(obj),
        });
    }
    if items.is_empty() {
        return None;
    }
    Some(SelectionColorDragState { start_items: items })
}

/// Apply an in-progress lightness-shift drag to all selected objects.
///
/// The shift is applied to each object's *original* base fill color from the seed, ensuring
/// that dragging back to the start position fully restores the original color.
#[cfg(feature = "hydrate")]
pub fn apply_selection_color_shift(
    board: RwSignal<BoardState>,
    drag_state_signal: RwSignal<Option<SelectionColorDragState>>,
    target_shift: f64,
) {
    let Some(drag_state) = drag_state_signal.get_untracked() else {
        return;
    };
    let shift = target_shift.clamp(-1.0, 1.0);
    board.update(|b| {
        for seed in &drag_state.start_items {
            let Some(obj) = b.objects.get_mut(&seed.id) else {
                continue;
            };
            upsert_object_color_props(obj, &seed.start_base_fill, shift);
        }
    });
}

/// Commit color changes to the server for objects whose color actually changed.
///
/// Compares fill, base fill, and lightness shift to seeded values. Clears the drag state.
#[cfg(feature = "hydrate")]
pub fn commit_selection_color_updates(
    board: RwSignal<BoardState>,
    sender: RwSignal<FrameSender>,
    drag_state_signal: RwSignal<Option<SelectionColorDragState>>,
) {
    let Some(drag_state) = drag_state_signal.get_untracked() else {
        return;
    };
    let state = board.get_untracked();
    for seed in &drag_state.start_items {
        let Some(obj) = state.objects.get(&seed.id) else {
            continue;
        };
        let fill = object_fill_hex(obj);
        let base_fill = object_base_fill_hex(obj);
        let shift = object_lightness_shift(obj);
        let changed = selection_color_changed(
            &fill,
            &base_fill,
            shift,
            &seed.start_fill,
            &seed.start_base_fill,
            seed.start_lightness_shift,
        );
        if !changed {
            continue;
        }
        send_object_update_props(sender, &seed.board_id, &seed.id, seed.version, &obj.props);
    }
    drag_state_signal.set(None);
}

/// Set all selected objects to an absolute base fill color and immediately commit.
///
/// Preserves each object's existing lightness shift, only replacing the base hue.
#[cfg(feature = "hydrate")]
pub fn apply_group_base_color_target(board: RwSignal<BoardState>, sender: RwSignal<FrameSender>, raw_color: String) {
    let base_fill = normalize_hex_color(&raw_color, "#D94B4B");
    let state = board.get_untracked();
    let selected: Vec<String> = state
        .selection
        .iter()
        .filter(|id| state.objects.contains_key(*id))
        .cloned()
        .collect();
    if selected.is_empty() {
        return;
    }

    board.update(|b| {
        for id in &selected {
            let Some(obj) = b.objects.get_mut(id) else {
                continue;
            };
            let shift = object_lightness_shift(obj);
            upsert_object_color_props(obj, &base_fill, shift);
        }
    });

    let post = board.get_untracked();
    for id in selected {
        let Some(obj) = post.objects.get(&id) else {
            continue;
        };
        send_object_update_props(sender, &obj.board_id, &obj.id, obj.version, &obj.props);
    }
}

/// Reset all selected objects to the application default fill color (`#D94B4B`, no shift).
#[cfg(feature = "hydrate")]
pub fn apply_group_background_defaults_target(board: RwSignal<BoardState>, sender: RwSignal<FrameSender>) {
    let base_fill = "#D94B4B".to_owned();
    let state = board.get_untracked();
    let selected: Vec<String> = state
        .selection
        .iter()
        .filter(|id| state.objects.contains_key(*id))
        .cloned()
        .collect();
    if selected.is_empty() {
        return;
    }

    board.update(|b| {
        for id in &selected {
            let Some(obj) = b.objects.get_mut(id) else {
                continue;
            };
            upsert_object_color_props(obj, &base_fill, 0.0);
        }
    });

    let post = board.get_untracked();
    for id in selected {
        let Some(obj) = post.objects.get(&id) else {
            continue;
        };
        send_object_update_props(sender, &obj.board_id, &obj.id, obj.version, &obj.props);
    }
}

/// No-op stub used on SSR builds where the hydrate feature is absent.
#[cfg(not(feature = "hydrate"))]
pub fn apply_group_base_color_target(_board: RwSignal<BoardState>, _sender: RwSignal<FrameSender>, _raw_color: String) {
}

/// No-op stub used on SSR builds where the hydrate feature is absent.
#[cfg(not(feature = "hydrate"))]
pub fn apply_group_background_defaults_target(_board: RwSignal<BoardState>, _sender: RwSignal<FrameSender>) {}

/// Snapshot all selected objects into a [`SelectionBorderDragState`] ready for border dragging.
///
/// Returns `None` when nothing is selected.
#[cfg(feature = "hydrate")]
pub fn selection_border_seed(board: RwSignal<BoardState>) -> Option<SelectionBorderDragState> {
    let state = board.get_untracked();
    let mut items = Vec::new();
    for id in &state.selection {
        let Some(obj) = state.objects.get(id) else {
            continue;
        };
        items.push(SelectionBorderSeed {
            id: obj.id.clone(),
            board_id: obj.board_id.clone(),
            version: obj.version,
            start_border_color: object_border_color_hex(obj),
            start_border_width: object_border_width(obj),
        });
    }
    if items.is_empty() {
        return None;
    }
    Some(SelectionBorderDragState { start_items: items })
}

/// Apply an in-progress border-width drag to all selected objects.
///
/// Preserves each object's seed border color, only updating the width.
#[cfg(feature = "hydrate")]
pub fn apply_selection_border_width(
    board: RwSignal<BoardState>,
    drag_state_signal: RwSignal<Option<SelectionBorderDragState>>,
    target_width: f64,
) {
    let Some(drag_state) = drag_state_signal.get_untracked() else {
        return;
    };
    let width = target_width.clamp(BORDER_WIDTH_MIN, BORDER_WIDTH_MAX);
    board.update(|b| {
        for seed in &drag_state.start_items {
            let Some(obj) = b.objects.get_mut(&seed.id) else {
                continue;
            };
            upsert_object_border_props(obj, &seed.start_border_color, width);
        }
    });
}

/// Commit border changes to the server for objects whose border actually changed.
///
/// Compares color and width to seeded values; clears the drag state after committing.
#[cfg(feature = "hydrate")]
pub fn commit_selection_border_updates(
    board: RwSignal<BoardState>,
    sender: RwSignal<FrameSender>,
    drag_state_signal: RwSignal<Option<SelectionBorderDragState>>,
) {
    let Some(drag_state) = drag_state_signal.get_untracked() else {
        return;
    };
    let state = board.get_untracked();
    for seed in &drag_state.start_items {
        let Some(obj) = state.objects.get(&seed.id) else {
            continue;
        };
        let color = object_border_color_hex(obj);
        let width = object_border_width(obj);
        let changed = selection_border_changed(&color, width, &seed.start_border_color, seed.start_border_width);
        if !changed {
            continue;
        }
        send_object_update_props(sender, &seed.board_id, &seed.id, seed.version, &obj.props);
    }
    drag_state_signal.set(None);
}

/// Set all selected objects to an absolute border color and immediately commit.
///
/// Preserves each object's existing border width.
#[cfg(feature = "hydrate")]
pub fn apply_group_border_color_target(board: RwSignal<BoardState>, sender: RwSignal<FrameSender>, raw_color: String) {
    let border = normalize_hex_color(&raw_color, "#1F1A17");
    let state = board.get_untracked();
    let selected: Vec<String> = state
        .selection
        .iter()
        .filter(|id| state.objects.contains_key(*id))
        .cloned()
        .collect();
    if selected.is_empty() {
        return;
    }

    board.update(|b| {
        for id in &selected {
            let Some(obj) = b.objects.get_mut(id) else {
                continue;
            };
            let width = object_border_width(obj);
            upsert_object_border_props(obj, &border, width);
        }
    });

    let post = board.get_untracked();
    for id in selected {
        let Some(obj) = post.objects.get(&id) else {
            continue;
        };
        send_object_update_props(sender, &obj.board_id, &obj.id, obj.version, &obj.props);
    }
}

/// Reset all selected objects to the application default border (`#1F1A17`, 0px width).
#[cfg(feature = "hydrate")]
pub fn apply_group_border_defaults_target(board: RwSignal<BoardState>, sender: RwSignal<FrameSender>) {
    let border = "#1F1A17".to_owned();
    let width = 0.0;
    let state = board.get_untracked();
    let selected: Vec<String> = state
        .selection
        .iter()
        .filter(|id| state.objects.contains_key(*id))
        .cloned()
        .collect();
    if selected.is_empty() {
        return;
    }

    board.update(|b| {
        for id in &selected {
            let Some(obj) = b.objects.get_mut(id) else {
                continue;
            };
            upsert_object_border_props(obj, &border, width);
        }
    });

    let post = board.get_untracked();
    for id in selected {
        let Some(obj) = post.objects.get(&id) else {
            continue;
        };
        send_object_update_props(sender, &obj.board_id, &obj.id, obj.version, &obj.props);
    }
}

/// No-op stub used on SSR builds where the hydrate feature is absent.
#[cfg(not(feature = "hydrate"))]
pub fn apply_group_border_color_target(
    _board: RwSignal<BoardState>,
    _sender: RwSignal<FrameSender>,
    _raw_color: String,
) {
}

/// No-op stub used on SSR builds where the hydrate feature is absent.
#[cfg(not(feature = "hydrate"))]
pub fn apply_group_border_defaults_target(_board: RwSignal<BoardState>, _sender: RwSignal<FrameSender>) {}

/// Snapshot all selected objects into a [`SelectionTextStyleDragState`] ready for text-style dragging.
///
/// Returns `None` when nothing is selected.
#[cfg(feature = "hydrate")]
pub fn selection_text_style_seed(board: RwSignal<BoardState>) -> Option<SelectionTextStyleDragState> {
    let state = board.get_untracked();
    let mut items = Vec::new();
    for id in &state.selection {
        let Some(obj) = state.objects.get(id) else {
            continue;
        };
        items.push(SelectionTextStyleSeed {
            id: obj.id.clone(),
            board_id: obj.board_id.clone(),
            version: obj.version,
            start_text_color: object_text_color_hex(obj),
            start_font_size: object_font_size(obj),
        });
    }
    if items.is_empty() {
        return None;
    }
    Some(SelectionTextStyleDragState { start_items: items })
}

/// Apply an in-progress font-size drag to all selected objects.
///
/// Preserves each object's seed text color, only updating the font size.
/// The size is snapped to the nearest integer pixel before writing.
#[cfg(feature = "hydrate")]
pub fn apply_selection_font_size(
    board: RwSignal<BoardState>,
    drag_state_signal: RwSignal<Option<SelectionTextStyleDragState>>,
    target_size: f64,
) {
    let Some(drag_state) = drag_state_signal.get_untracked() else {
        return;
    };
    let size = snap_font_size_to_px(target_size);
    board.update(|b| {
        for seed in &drag_state.start_items {
            let Some(obj) = b.objects.get_mut(&seed.id) else {
                continue;
            };
            upsert_object_text_style_props(obj, &seed.start_text_color, size);
        }
    });
}

/// Commit text-style changes to the server for objects whose style actually changed.
///
/// Compares text color and font size to seeded values; clears the drag state after committing.
#[cfg(feature = "hydrate")]
pub fn commit_selection_text_style_updates(
    board: RwSignal<BoardState>,
    sender: RwSignal<FrameSender>,
    drag_state_signal: RwSignal<Option<SelectionTextStyleDragState>>,
) {
    let Some(drag_state) = drag_state_signal.get_untracked() else {
        return;
    };
    let state = board.get_untracked();
    for seed in &drag_state.start_items {
        let Some(obj) = state.objects.get(&seed.id) else {
            continue;
        };
        let color = object_text_color_hex(obj);
        let size = object_font_size(obj);
        let changed = selection_text_style_changed(&color, size, &seed.start_text_color, seed.start_font_size);
        if !changed {
            continue;
        }
        send_object_update_props(sender, &seed.board_id, &seed.id, seed.version, &obj.props);
    }
    drag_state_signal.set(None);
}

/// Set all selected objects to an absolute text color and immediately commit.
///
/// Preserves each object's existing font size.
#[cfg(feature = "hydrate")]
pub fn apply_group_text_color_target(board: RwSignal<BoardState>, sender: RwSignal<FrameSender>, raw_color: String) {
    let color = normalize_hex_color(&raw_color, "#1F1A17");
    let state = board.get_untracked();
    let selected: Vec<String> = state
        .selection
        .iter()
        .filter(|id| state.objects.contains_key(*id))
        .cloned()
        .collect();
    if selected.is_empty() {
        return;
    }

    board.update(|b| {
        for id in &selected {
            let Some(obj) = b.objects.get_mut(id) else {
                continue;
            };
            let size = object_font_size(obj);
            upsert_object_text_style_props(obj, &color, size);
        }
    });

    let post = board.get_untracked();
    for id in selected {
        let Some(obj) = post.objects.get(&id) else {
            continue;
        };
        send_object_update_props(sender, &obj.board_id, &obj.id, obj.version, &obj.props);
    }
}

/// Reset all selected objects to the default text style (`#1F1A17`, 24px).
#[cfg(feature = "hydrate")]
pub fn apply_group_text_style_defaults_target(board: RwSignal<BoardState>, sender: RwSignal<FrameSender>) {
    let color = "#1F1A17".to_owned();
    let size = 24.0;
    let state = board.get_untracked();
    let selected: Vec<String> = state
        .selection
        .iter()
        .filter(|id| state.objects.contains_key(*id))
        .cloned()
        .collect();
    if selected.is_empty() {
        return;
    }

    board.update(|b| {
        for id in &selected {
            let Some(obj) = b.objects.get_mut(id) else {
                continue;
            };
            upsert_object_text_style_props(obj, &color, size);
        }
    });

    let post = board.get_untracked();
    for id in selected {
        let Some(obj) = post.objects.get(&id) else {
            continue;
        };
        send_object_update_props(sender, &obj.board_id, &obj.id, obj.version, &obj.props);
    }
}

/// No-op stub used on SSR builds where the hydrate feature is absent.
#[cfg(not(feature = "hydrate"))]
pub fn apply_group_text_color_target(_board: RwSignal<BoardState>, _sender: RwSignal<FrameSender>, _raw_color: String) {
}

/// No-op stub used on SSR builds where the hydrate feature is absent.
#[cfg(not(feature = "hydrate"))]
pub fn apply_group_text_style_defaults_target(_board: RwSignal<BoardState>, _sender: RwSignal<FrameSender>) {}

/// Compute the representative scale for a set of scale seeds by averaging their start scales.
#[cfg(feature = "hydrate")]
fn selection_representative_scale_from_items(items: &[SelectionScaleSeed]) -> f64 {
    representative_scale_from_values(&items.iter().map(|s| s.start_scale).collect::<Vec<_>>())
}

/// Apply an in-progress rotation drag to all selected objects.
///
/// Each frame, the pointer's current compass angle is snapped via `snap_fn`, and the delta
/// from the drag's starting pointer angle is computed via `delta_fn`. That delta is added to
/// each object's per-object start rotation and normalised with `normalize_fn`. Injecting the
/// math functions as closures keeps this function testable without browser or signal state.
#[cfg(feature = "hydrate")]
pub fn apply_selection_rotation_drag(
    board: RwSignal<BoardState>,
    drag_state_signal: RwSignal<Option<SelectionRotationDragState>>,
    pointer_angle_deg: f64,
    shift_snap: bool,
    snap_fn: impl Fn(f64, bool) -> f64,
    delta_fn: impl Fn(f64, f64) -> f64,
    normalize_fn: impl Fn(f64) -> f64,
) {
    let Some(drag_state) = drag_state_signal.get_untracked() else {
        return;
    };
    let snapped_pointer = snap_fn(pointer_angle_deg, shift_snap);
    let delta = delta_fn(snapped_pointer, drag_state.start_pointer_angle_deg);
    board.update(|b| {
        for (id, start_rotation) in &drag_state.start_rotations {
            if let Some(obj) = b.objects.get_mut(id) {
                obj.rotation = normalize_fn(*start_rotation + delta);
            }
        }
    });
}

/// Commit rotation changes to the server for objects that rotated by more than 0.01°.
///
/// Uses `angular_delta_fn` to measure the absolute change so that wrapping around 0°/360° is
/// handled correctly. Clears the drag state signal after committing.
#[cfg(feature = "hydrate")]
pub fn commit_selection_rotation_updates(
    board: RwSignal<BoardState>,
    sender: RwSignal<FrameSender>,
    drag_state_signal: RwSignal<Option<SelectionRotationDragState>>,
    angular_delta_fn: impl Fn(f64, f64) -> f64,
) {
    let Some(drag_state) = drag_state_signal.get_untracked() else {
        return;
    };
    let state = board.get_untracked();
    for (id, start_rotation) in &drag_state.start_rotations {
        let Some(obj) = state.objects.get(id) else {
            continue;
        };
        if angular_delta_fn(obj.rotation, *start_rotation) < 0.01 {
            continue;
        }
        send_object_update_rotation(sender, &obj.board_id, &obj.id, obj.version, obj.rotation);
    }
    drag_state_signal.set(None);
}

/// Set all selected objects to an absolute rotation and immediately commit.
///
/// Computes the delta from the current representative rotation to `target_deg`, then applies
/// it individually to each object's current rotation so that multi-object selections rotate
/// consistently rather than jumping to the same absolute angle.
#[cfg(feature = "hydrate")]
pub fn apply_group_rotation_target(
    board: RwSignal<BoardState>,
    sender: RwSignal<FrameSender>,
    target_deg: f64,
    delta_fn: impl Fn(f64, f64) -> f64,
    normalize_fn: impl Fn(f64) -> f64,
) {
    let state = board.get_untracked();
    let selected: Vec<String> = state
        .selection
        .iter()
        .filter(|id| state.objects.contains_key(*id))
        .cloned()
        .collect();
    if selected.is_empty() {
        return;
    }

    let current = representative_rotation_deg(&state);
    let delta = delta_fn(target_deg, current);

    board.update(|b| {
        for id in &selected {
            if let Some(obj) = b.objects.get_mut(id) {
                obj.rotation = normalize_fn(obj.rotation + delta);
            }
        }
    });

    let post = board.get_untracked();
    for id in selected {
        let Some(obj) = post.objects.get(&id) else {
            continue;
        };
        send_object_update_rotation(sender, &obj.board_id, &obj.id, obj.version, obj.rotation);
    }
}
