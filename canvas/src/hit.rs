#[cfg(test)]
#[path = "hit_test.rs"]
mod hit_test;

use crate::camera::{Camera, Point};
use crate::doc::{DocStore, ObjectId};

/// Which part of an object was hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitPart {
    Body,
    ResizeHandle(ResizeAnchor),
    RotateHandle,
    EdgeEndpoint(EdgeEnd),
    EdgeBody,
}

/// Anchor position for resize handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeAnchor {
    N,
    Ne,
    E,
    Se,
    S,
    Sw,
    W,
    Nw,
}

/// Which end of an edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeEnd {
    A,
    B,
}

/// Result of a hit test.
#[derive(Debug, Clone, Copy)]
pub struct Hit {
    pub object_id: ObjectId,
    pub part: HitPart,
}

/// Test which object (if any) is under `world_pt`, checking selected object handles first.
///
/// Not yet implemented.
#[must_use]
pub fn hit_test(_world_pt: Point, _doc: &DocStore, _camera: &Camera, _selected_id: Option<ObjectId>) -> Option<Hit> {
    todo!()
}
