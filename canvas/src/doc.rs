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
    Rect,
    Ellipse,
    Diamond,
    Star,
    Line,
    Arrow,
}

/// A board object as stored in the document and on the wire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardObject {
    pub id: ObjectId,
    pub board_id: ObjectId,
    pub kind: ObjectKind,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub rotation: f64,
    pub z_index: i64,
    pub props: serde_json::Value,
    pub created_by: Option<ObjectId>,
    pub version: i64,
}

/// Sparse update for a board object. Only present fields are applied.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PartialBoardObject {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z_index: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub props: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
}

/// Typed access to common props fields from a `BoardObject.props` JSON value.
pub struct Props<'a> {
    value: &'a serde_json::Value,
}

impl<'a> Props<'a> {
    #[must_use]
    pub fn new(value: &'a serde_json::Value) -> Self {
        Self { value }
    }

    #[must_use]
    pub fn fill(&self) -> &str {
        self.value
            .get("fill")
            .and_then(|v| v.as_str())
            .unwrap_or("#D94B4B")
    }

    #[must_use]
    pub fn stroke(&self) -> &str {
        self.value
            .get("stroke")
            .and_then(|v| v.as_str())
            .unwrap_or("#1F1A17")
    }

    #[must_use]
    pub fn stroke_width(&self) -> f64 {
        self.value
            .get("stroke_width")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(1.0)
    }

    #[must_use]
    pub fn head(&self) -> &str {
        self.value
            .get("head")
            .and_then(|v| v.as_str())
            .unwrap_or("")
    }

    #[must_use]
    pub fn text(&self) -> &str {
        self.value
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
    }

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
    #[must_use]
    pub fn new() -> Self {
        Self { objects: HashMap::new() }
    }

    pub fn insert(&mut self, obj: BoardObject) {
        self.objects.insert(obj.id, obj);
    }

    pub fn remove(&mut self, id: &ObjectId) -> Option<BoardObject> {
        self.objects.remove(id)
    }

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
            // Shallow merge: update keys present in partial, leave others intact.
            if let (Some(existing), Some(incoming)) = (obj.props.as_object_mut(), props.as_object()) {
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

    #[must_use]
    pub fn len(&self) -> usize {
        self.objects.len()
    }

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
