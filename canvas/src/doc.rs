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

use std::collections::HashMap;

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
    /// Embedded YouTube tile rendered as a retro TV shell.
    Youtube,
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

    /// Stroke width in world units. Defaults to `1.0` when absent.
    #[must_use]
    pub fn stroke_width(&self) -> f64 {
        self.value
            .get("stroke_width")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(1.0)
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

/// In-memory store of board objects.
pub struct DocStore {
    objects: HashMap<ObjectId, BoardObject>,
}

impl DocStore {
    /// Create an empty store.
    #[must_use]
    pub fn new() -> Self {
        Self { objects: HashMap::new() }
    }

    /// Insert or replace an object. If an object with the same `id` already
    /// exists it is overwritten.
    pub fn insert(&mut self, obj: BoardObject) {
        self.objects.insert(obj.id, obj);
    }

    /// Remove an object by id, returning it if it was present.
    pub fn remove(&mut self, id: &ObjectId) -> Option<BoardObject> {
        self.objects.remove(id)
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
        true
    }

    /// Replace all objects with a full snapshot.
    pub fn load_snapshot(&mut self, objects: Vec<BoardObject>) {
        self.objects.clear();
        for obj in objects {
            self.objects.insert(obj.id, obj);
        }
    }

    /// Return all objects sorted by `(z_index, id)` for draw-order.
    #[must_use]
    pub fn sorted_objects(&self) -> Vec<&BoardObject> {
        let mut objs: Vec<&BoardObject> = self.objects.values().collect();
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
}

impl Default for DocStore {
    fn default() -> Self {
        Self::new()
    }
}
