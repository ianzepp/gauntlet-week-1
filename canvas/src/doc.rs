//! Document model: board objects, their properties, and the in-memory store.
//!
//! This module defines the core data types that describe what is on the canvas
//! (`BoardObject`, `ObjectKind`), a sparse-update type for incremental edits
//! (`PartialBoardObject`), a typed accessor for the open-ended `props` JSON bag
//! (`Props`), and the runtime store that owns all live objects (`DocStore`).
//!
//! Data flows into this layer from the network (JSON deserialization) and from
//! the input engine (mutations). The renderer reads from `DocStore` via
//! `sorted_objects` to determine draw order.

#[cfg(test)]
#[path = "doc_test.rs"]
mod doc_test;

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a board object.
pub type ObjectId = Uuid;

/// The kind of a board object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ObjectKind {
    /// Axis-aligned rectangle.
    Rect,
    /// Text-only object rendered without a shape fill/stroke.
    Text,
    /// Frame container shape.
    Frame,
    /// Ellipse inscribed within the bounding box.
    Ellipse,
    /// Diamond (rhombus) with vertices at bounding-box edge midpoints.
    Diamond,
    /// Five-point star inscribed within the bounding box.
    Star,
    /// Straight line segment between two endpoints stored in `props`.
    Line,
    /// Directed arrow (line with an arrowhead) between two endpoints stored in `props`.
    Arrow,
    /// Embedded SVG content rendered inside a rectangular bounding box.
    Svg,
}

/// A board object as stored in the document and on the wire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardObject {
    /// Unique identifier for this object.
    pub id: ObjectId,
    /// The board this object belongs to.
    pub board_id: ObjectId,
    /// Shape or edge type.
    pub kind: ObjectKind,
    /// Left edge of the bounding box in world coordinates.
    pub x: f64,
    /// Top edge of the bounding box in world coordinates.
    pub y: f64,
    /// Width of the bounding box in world coordinates.
    pub width: f64,
    /// Height of the bounding box in world coordinates.
    pub height: f64,
    /// Clockwise rotation in degrees around the bounding-box center.
    pub rotation: f64,
    /// Stacking order; lower values are drawn beneath higher values.
    pub z_index: i64,
    /// Open-ended per-kind properties (fill, stroke, endpoints, text, etc.).
    pub props: serde_json::Value,
    /// User who created the object, if known.
    pub created_by: Option<ObjectId>,
    /// Monotonically increasing edit counter used for conflict detection.
    pub version: i64,
    /// Optional persistent grouping id for multi-object group operations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<ObjectId>,
}

/// Sparse update for a board object. Only present fields are applied.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PartialBoardObject {
    /// New x position, if being updated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,
    /// New y position, if being updated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
    /// New width, if being updated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    /// New height, if being updated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
    /// New rotation in degrees, if being updated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation: Option<f64>,
    /// New z-index, if being updated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z_index: Option<i64>,
    /// Props keys to merge or remove (null values delete keys).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub props: Option<serde_json::Value>,
    /// New version counter, if being updated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
    /// New grouping id. `Some(None)` clears grouping, `None` leaves unchanged.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<Option<ObjectId>>,
}

/// Typed access to common props fields from a `BoardObject.props` JSON value.
pub struct Props<'a> {
    value: &'a serde_json::Value,
}

impl<'a> Props<'a> {
    /// Wrap a reference to a `props` JSON value for typed access.
    #[must_use]
    pub fn new(value: &'a serde_json::Value) -> Self {
        Self { value }
    }

    /// Fill color as a CSS color string. Defaults to `"#D94B4B"` when absent.
    #[must_use]
    pub fn fill(&self) -> &str {
        self.value
            .get("fill")
            .and_then(|v| v.as_str())
            .unwrap_or("#D94B4B")
    }

    /// Stroke color as a CSS color string. Defaults to `"#1F1A17"` when absent.
    #[must_use]
    pub fn stroke(&self) -> &str {
        self.value
            .get("stroke")
            .and_then(|v| v.as_str())
            .unwrap_or("#1F1A17")
    }

    /// Stroke width in world units. Defaults to `0.0` when absent.
    #[must_use]
    pub fn stroke_width(&self) -> f64 {
        self.value
            .get("strokeWidth")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0)
    }

    /// Text color as a CSS color string.
    ///
    /// Resolution order:
    /// 1) Explicit `textColor`
    /// 2) Contrast-aware fallback from `fill`
    /// 3) `"#1F1A17"` when no color context is available
    #[must_use]
    pub fn text_color(&self) -> &str {
        if let Some(color) = self.value.get("textColor").and_then(|v| v.as_str()) {
            return color;
        }
        if let Some(fill) = self.value.get("fill").and_then(|v| v.as_str()) {
            return contrast_text_color(fill);
        }
        "#1F1A17"
    }

    /// Font size in pixels, if explicitly set by props.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn font_size(&self) -> Option<f64> {
        self.value
            .get("fontSize")
            .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|n| n as f64)))
    }

    /// Arrowhead style at endpoint A (the "head" of the arrow). Empty string when absent.
    #[must_use]
    pub fn head(&self) -> &str {
        self.value
            .get("head")
            .and_then(|v| v.as_str())
            .unwrap_or("")
    }

    /// Label text displayed on the object. Empty string when absent.
    #[must_use]
    pub fn text(&self) -> &str {
        self.value
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
    }

    /// Arrowhead style at endpoint B (the "foot" of the arrow). Empty string when absent.
    #[must_use]
    pub fn foot(&self) -> &str {
        self.value
            .get("foot")
            .and_then(|v| v.as_str())
            .unwrap_or("")
    }
}

fn contrast_text_color(fill: &str) -> &'static str {
    if let Some((r, g, b)) = parse_css_rgb(fill) {
        // Relative luminance in linear RGB.
        let l = relative_luminance(r, g, b);
        if l < 0.42 { "#F5F0E8" } else { "#1F1A17" }
    } else {
        "#1F1A17"
    }
}

fn parse_css_rgb(raw: &str) -> Option<(u8, u8, u8)> {
    let s = raw.trim();
    if let Some(hex) = s.strip_prefix('#') {
        return match hex.len() {
            3 => {
                let Ok(r) = u8::from_str_radix(&hex[0..1].repeat(2), 16) else {
                    return None;
                };
                let Ok(g) = u8::from_str_radix(&hex[1..2].repeat(2), 16) else {
                    return None;
                };
                let Ok(b) = u8::from_str_radix(&hex[2..3].repeat(2), 16) else {
                    return None;
                };
                Some((r, g, b))
            }
            6 => {
                let Ok(r) = u8::from_str_radix(&hex[0..2], 16) else {
                    return None;
                };
                let Ok(g) = u8::from_str_radix(&hex[2..4], 16) else {
                    return None;
                };
                let Ok(b) = u8::from_str_radix(&hex[4..6], 16) else {
                    return None;
                };
                Some((r, g, b))
            }
            _ => None,
        };
    }

    let open = s.find('(')?;
    let close = s.rfind(')')?;
    let func = s[..open].trim().to_ascii_lowercase();
    if func != "rgb" && func != "rgba" {
        return None;
    }
    let body = &s[open + 1..close];
    let mut parts = body.split(',').map(str::trim);
    let Ok(r) = parts.next()?.parse::<u8>() else {
        return None;
    };
    let Ok(g) = parts.next()?.parse::<u8>() else {
        return None;
    };
    let Ok(b) = parts.next()?.parse::<u8>() else {
        return None;
    };
    Some((r, g, b))
}

fn srgb_to_linear(v: u8) -> f64 {
    let c = f64::from(v) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn relative_luminance(r: u8, g: u8, b: u8) -> f64 {
    (0.2126 * srgb_to_linear(r)) + (0.7152 * srgb_to_linear(g)) + (0.0722 * srgb_to_linear(b))
}

/// In-memory store of board objects.
pub struct DocStore {
    objects: HashMap<ObjectId, BoardObject>,
    buckets: HashMap<(i32, i32), HashSet<ObjectId>>,
}

/// Axis-aligned world bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldBounds {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl WorldBounds {
    #[must_use]
    pub fn from_point(point_x: f64, point_y: f64) -> Self {
        Self { min_x: point_x, min_y: point_y, max_x: point_x, max_y: point_y }
    }

    #[must_use]
    pub fn expand(self, delta: f64) -> Self {
        Self {
            min_x: self.min_x - delta,
            min_y: self.min_y - delta,
            max_x: self.max_x + delta,
            max_y: self.max_y + delta,
        }
    }
}

const BUCKET_SIZE_WORLD: f64 = 256.0;

#[must_use]
pub fn object_world_bounds(obj: &BoardObject) -> WorldBounds {
    match obj.kind {
        ObjectKind::Line | ObjectKind::Arrow => {
            let a = obj
                .props
                .get("a")
                .and_then(serde_json::Value::as_object)
                .and_then(|point| Some((point.get("x")?.as_f64()?, point.get("y")?.as_f64()?)));
            let b = obj
                .props
                .get("b")
                .and_then(serde_json::Value::as_object)
                .and_then(|point| Some((point.get("x")?.as_f64()?, point.get("y")?.as_f64()?)));

            if let (Some((ax, ay)), Some((bx, by))) = (a, b) {
                return WorldBounds { min_x: ax.min(bx), min_y: ay.min(by), max_x: ax.max(bx), max_y: ay.max(by) };
            }
        }
        _ => {}
    }

    let min_x = obj.x.min(obj.x + obj.width);
    let min_y = obj.y.min(obj.y + obj.height);
    let max_x = obj.x.max(obj.x + obj.width);
    let max_y = obj.y.max(obj.y + obj.height);
    WorldBounds { min_x, min_y, max_x, max_y }
}

impl DocStore {
    /// Create an empty store.
    #[must_use]
    pub fn new() -> Self {
        Self { objects: HashMap::new(), buckets: HashMap::new() }
    }

    /// Insert or replace an object. If an object with the same `id` already
    /// exists it is overwritten.
    pub fn insert(&mut self, obj: BoardObject) {
        let id = obj.id;
        let new_bounds = object_world_bounds(&obj);
        if let Some(prev) = self.objects.insert(id, obj) {
            self.remove_from_buckets(id, object_world_bounds(&prev));
        }
        self.add_to_buckets(id, new_bounds);
    }

    /// Remove an object by id, returning it if it was present.
    pub fn remove(&mut self, id: &ObjectId) -> Option<BoardObject> {
        let removed = self.objects.remove(id)?;
        self.remove_from_buckets(*id, object_world_bounds(&removed));
        Some(removed)
    }

    /// Return a reference to an object by id.
    #[must_use]
    pub fn get(&self, id: &ObjectId) -> Option<&BoardObject> {
        self.objects.get(id)
    }

    /// Apply a partial update to an existing object. Returns false if the object doesn't exist.
    pub fn apply_partial(&mut self, id: &ObjectId, partial: &PartialBoardObject) -> bool {
        let Some(obj) = self.objects.get_mut(id) else {
            return false;
        };
        let old_bounds = object_world_bounds(obj);
        if let Some(x) = partial.x {
            obj.x = x;
        }
        if let Some(y) = partial.y {
            obj.y = y;
        }
        if let Some(w) = partial.width {
            obj.width = w;
        }
        if let Some(h) = partial.height {
            obj.height = h;
        }
        if let Some(r) = partial.rotation {
            obj.rotation = r;
        }
        if let Some(z) = partial.z_index {
            obj.z_index = z;
        }
        if let Some(v) = partial.version {
            obj.version = v;
        }
        if let Some(group_id) = partial.group_id {
            obj.group_id = group_id;
        }
        if let Some(ref props) = partial.props {
            let Some(incoming) = props.as_object() else {
                return false;
            };

            if !obj.props.is_object() {
                obj.props = serde_json::json!({});
            }

            if let Some(existing) = obj.props.as_object_mut() {
                for (k, v) in incoming {
                    if v.is_null() {
                        existing.remove(k);
                    } else {
                        existing.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        let new_bounds = object_world_bounds(obj);
        if old_bounds != new_bounds {
            self.remove_from_buckets(*id, old_bounds);
            self.add_to_buckets(*id, new_bounds);
        }
        true
    }

    /// Replace all objects with a full snapshot.
    pub fn load_snapshot(&mut self, objects: Vec<BoardObject>) {
        self.objects.clear();
        self.buckets.clear();
        for obj in objects {
            self.insert(obj);
        }
    }

    /// Return all objects sorted by `(z_index, id)` for draw-order.
    #[must_use]
    pub fn sorted_objects(&self) -> Vec<&BoardObject> {
        let mut objs: Vec<&BoardObject> = self.objects.values().collect();
        objs.sort_by(|a, b| a.z_index.cmp(&b.z_index).then_with(|| a.id.cmp(&b.id)));
        objs
    }

    /// Return objects that intersect the given world bounds, sorted by `(z_index, id)`.
    #[must_use]
    pub fn sorted_objects_in_bounds(&self, bounds: WorldBounds) -> Vec<&BoardObject> {
        let mut ids = HashSet::new();
        let min_bx = bucket_coord(bounds.min_x);
        let min_by = bucket_coord(bounds.min_y);
        let max_bx = bucket_coord(bounds.max_x);
        let max_by = bucket_coord(bounds.max_y);

        for by in min_by..=max_by {
            for bx in min_bx..=max_bx {
                if let Some(bucket) = self.buckets.get(&(bx, by)) {
                    ids.extend(bucket.iter().copied());
                }
            }
        }

        let mut objs = ids
            .into_iter()
            .filter_map(|id| self.objects.get(&id))
            .filter(|obj| {
                let obj_bounds = object_world_bounds(obj);
                !(obj_bounds.max_x < bounds.min_x
                    || obj_bounds.min_x > bounds.max_x
                    || obj_bounds.max_y < bounds.min_y
                    || obj_bounds.min_y > bounds.max_y)
            })
            .collect::<Vec<_>>();
        objs.sort_by(|a, b| a.z_index.cmp(&b.z_index).then_with(|| a.id.cmp(&b.id)));
        objs
    }

    /// Number of objects currently in the store.
    #[must_use]
    pub fn len(&self) -> usize {
        self.objects.len()
    }

    /// Returns `true` if the store contains no objects.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    fn add_to_buckets(&mut self, id: ObjectId, bounds: WorldBounds) {
        let min_bx = bucket_coord(bounds.min_x);
        let min_by = bucket_coord(bounds.min_y);
        let max_bx = bucket_coord(bounds.max_x);
        let max_by = bucket_coord(bounds.max_y);
        for by in min_by..=max_by {
            for bx in min_bx..=max_bx {
                self.buckets.entry((bx, by)).or_default().insert(id);
            }
        }
    }

    fn remove_from_buckets(&mut self, id: ObjectId, bounds: WorldBounds) {
        let min_bx = bucket_coord(bounds.min_x);
        let min_by = bucket_coord(bounds.min_y);
        let max_bx = bucket_coord(bounds.max_x);
        let max_by = bucket_coord(bounds.max_y);
        for by in min_by..=max_by {
            for bx in min_bx..=max_bx {
                let key = (bx, by);
                let mut empty = false;
                if let Some(bucket) = self.buckets.get_mut(&key) {
                    bucket.remove(&id);
                    empty = bucket.is_empty();
                }
                if empty {
                    self.buckets.remove(&key);
                }
            }
        }
    }
}

fn bucket_coord(world: f64) -> i32 {
    (world / BUCKET_SIZE_WORLD).floor() as i32
}

impl Default for DocStore {
    fn default() -> Self {
        Self::new()
    }
}
