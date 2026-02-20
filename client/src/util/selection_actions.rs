//! Selection drag/apply/commit helpers used by canvas host.

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

#[cfg(feature = "hydrate")]
#[derive(Clone)]
pub struct SelectionRotationDragState {
    pub start_pointer_angle_deg: f64,
    pub start_rotations: Vec<(String, f64)>,
}

#[cfg(feature = "hydrate")]
#[derive(Clone)]
pub struct SelectionScaleSeed {
    pub id: String,
    pub board_id: String,
    pub version: i64,
    pub base_width: f64,
    pub base_height: f64,
    pub start_scale: f64,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[cfg(feature = "hydrate")]
#[derive(Clone)]
pub struct SelectionScaleDragState {
    pub start_items: Vec<SelectionScaleSeed>,
    pub group_center_x: f64,
    pub group_center_y: f64,
    pub start_group_scale: f64,
}

#[cfg(feature = "hydrate")]
#[derive(Clone)]
pub struct SelectionColorSeed {
    pub id: String,
    pub board_id: String,
    pub version: i64,
    pub start_fill: String,
    pub start_base_fill: String,
    pub start_lightness_shift: f64,
}

#[cfg(feature = "hydrate")]
#[derive(Clone)]
pub struct SelectionColorDragState {
    pub start_items: Vec<SelectionColorSeed>,
}

#[cfg(feature = "hydrate")]
#[derive(Clone)]
pub struct SelectionBorderSeed {
    pub id: String,
    pub board_id: String,
    pub version: i64,
    pub start_border_color: String,
    pub start_border_width: f64,
}

#[cfg(feature = "hydrate")]
#[derive(Clone)]
pub struct SelectionBorderDragState {
    pub start_items: Vec<SelectionBorderSeed>,
}

#[cfg(feature = "hydrate")]
#[derive(Clone)]
pub struct SelectionTextStyleSeed {
    pub id: String,
    pub board_id: String,
    pub version: i64,
    pub start_text_color: String,
    pub start_font_size: f64,
}

#[cfg(feature = "hydrate")]
#[derive(Clone)]
pub struct SelectionTextStyleDragState {
    pub start_items: Vec<SelectionTextStyleSeed>,
}

#[cfg(feature = "hydrate")]
pub fn has_selection(board: RwSignal<BoardState>) -> bool {
    !board.get_untracked().selection.is_empty()
}

#[cfg(feature = "hydrate")]
pub fn selected_object_rotations(board: RwSignal<BoardState>) -> Vec<(String, f64)> {
    let state = board.get_untracked();
    state
        .selection
        .iter()
        .filter_map(|id| state.objects.get(id).map(|obj| (id.clone(), obj.rotation)))
        .collect()
}

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

#[cfg(feature = "hydrate")]
pub fn apply_selection_scale_drag(
    board: RwSignal<BoardState>,
    drag_state_signal: RwSignal<Option<SelectionScaleDragState>>,
    target_scale: f64,
) {
    let Some(drag_state) = drag_state_signal.get_untracked() else {
        return;
    };
    let target_scale = target_scale.clamp(0.1, 10.0);
    let multiplier = if drag_state.start_group_scale.abs() < f64::EPSILON {
        1.0
    } else {
        target_scale / drag_state.start_group_scale
    };
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
        let changed = (obj.x - seed.x).abs() > 0.01
            || (obj.y - seed.y).abs() > 0.01
            || (obj.width.unwrap_or(seed.width) - seed.width).abs() > 0.01
            || (obj.height.unwrap_or(seed.height) - seed.height).abs() > 0.01;
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

#[cfg(not(feature = "hydrate"))]
pub fn apply_group_scale_target(_board: RwSignal<BoardState>, _sender: RwSignal<FrameSender>, _target_scale: f64) {}

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
        let changed = fill != seed.start_fill
            || base_fill != seed.start_base_fill
            || (shift - seed.start_lightness_shift).abs() > 0.001;
        if !changed {
            continue;
        }
        send_object_update_props(sender, &seed.board_id, &seed.id, seed.version, &obj.props);
    }
    drag_state_signal.set(None);
}

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

#[cfg(not(feature = "hydrate"))]
pub fn apply_group_base_color_target(_board: RwSignal<BoardState>, _sender: RwSignal<FrameSender>, _raw_color: String) {
}

#[cfg(not(feature = "hydrate"))]
pub fn apply_group_background_defaults_target(_board: RwSignal<BoardState>, _sender: RwSignal<FrameSender>) {}

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
        let changed = color != seed.start_border_color || (width - seed.start_border_width).abs() > 0.001;
        if !changed {
            continue;
        }
        send_object_update_props(sender, &seed.board_id, &seed.id, seed.version, &obj.props);
    }
    drag_state_signal.set(None);
}

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

#[cfg(not(feature = "hydrate"))]
pub fn apply_group_border_color_target(
    _board: RwSignal<BoardState>,
    _sender: RwSignal<FrameSender>,
    _raw_color: String,
) {
}

#[cfg(not(feature = "hydrate"))]
pub fn apply_group_border_defaults_target(_board: RwSignal<BoardState>, _sender: RwSignal<FrameSender>) {}

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
        let changed = color != seed.start_text_color || (size - seed.start_font_size).abs() > 0.001;
        if !changed {
            continue;
        }
        send_object_update_props(sender, &seed.board_id, &seed.id, seed.version, &obj.props);
    }
    drag_state_signal.set(None);
}

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

#[cfg(not(feature = "hydrate"))]
pub fn apply_group_text_color_target(_board: RwSignal<BoardState>, _sender: RwSignal<FrameSender>, _raw_color: String) {
}

#[cfg(not(feature = "hydrate"))]
pub fn apply_group_text_style_defaults_target(_board: RwSignal<BoardState>, _sender: RwSignal<FrameSender>) {}

#[cfg(feature = "hydrate")]
fn selection_representative_scale_from_items(items: &[SelectionScaleSeed]) -> f64 {
    if items.is_empty() {
        return 1.0;
    }
    items.iter().map(|s| s.start_scale).sum::<f64>() / items.len() as f64
}

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
