//! Hit-testing: geometry primitives and composite object-picking.
//!
//! This module answers the question "what did the user click on?" It operates
//! entirely in world coordinates and is pure (no side-effects). Callers convert
//! screen points to world points via [`crate::camera::Camera`] before calling
//! into this module.
//!
//! The main entry point is [`hit_test`], which layers handle-priority logic
//! on top of the lower-level shape containment and segment-distance helpers.

#[cfg(test)]
#[path = "hit_test.rs"]
mod hit_test;

use crate::camera::{Camera, Point};
use crate::consts::{FRAC_PI_5, HANDLE_RADIUS_PX, ROTATE_HANDLE_OFFSET_PX, STAR_INNER_RATIO};
use crate::doc::{BoardObject, DocStore, ObjectId, ObjectKind};

/// Which part of an object was hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitPart {
    /// The interior fill area of a shape.
    Body,
    /// One of the eight cardinal/ordinal resize handles around a shape's bounding box.
    ResizeHandle(ResizeAnchor),
    /// The circular rotate handle above the N edge of a shape.
    RotateHandle,
    /// One of the two draggable endpoints of a line or arrow.
    EdgeEndpoint(EdgeEnd),
    /// The body of a line or arrow (between its two endpoints).
    EdgeBody,
}

/// Anchor position for resize handles.
///
/// Variants are named by compass direction. Order matches [`RESIZE_ANCHORS`]
/// and the array returned by [`resize_handle_positions`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeAnchor {
    /// Top center.
    N,
    /// Top-right corner.
    Ne,
    /// Right center.
    E,
    /// Bottom-right corner.
    Se,
    /// Bottom center.
    S,
    /// Bottom-left corner.
    Sw,
    /// Left center.
    W,
    /// Top-left corner.
    Nw,
}

/// Which end of an edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeEnd {
    /// The first endpoint, stored in the object's `"a"` prop.
    A,
    /// The second endpoint, stored in the object's `"b"` prop.
    B,
}

/// Result of a hit test.
#[derive(Debug, Clone, Copy)]
pub struct Hit {
    /// The object that was hit.
    pub object_id: ObjectId,
    /// Which part of that object was hit.
    pub part: HitPart,
}

// =============================================================
// Pure geometry primitives
// =============================================================

/// Rotate a point around an origin by the given angle in degrees.
#[must_use]
pub fn rotate_point(pt: Point, origin: Point, angle_deg: f64) -> Point {
    let rad = angle_deg.to_radians();
    let cos = rad.cos();
    let sin = rad.sin();
    let dx = pt.x - origin.x;
    let dy = pt.y - origin.y;
    Point { x: origin.x + dx * cos - dy * sin, y: origin.y + dx * sin + dy * cos }
}

/// Transform a world point into the local (unrotated) coordinate space of a
/// bounding box defined by `(x, y, w, h, rotation_deg)`. The returned point
/// is relative to the box's top-left corner in local space.
#[must_use]
pub fn world_to_local(pt: Point, x: f64, y: f64, w: f64, h: f64, rotation_deg: f64) -> Point {
    let center = Point { x: x + w / 2.0, y: y + h / 2.0 };
    let unrotated = rotate_point(pt, center, -rotation_deg);
    Point { x: unrotated.x - x, y: unrotated.y - y }
}

/// Test if a point is inside an axis-aligned rectangle `[0, w] x [0, h]`.
/// The point should already be in local space (via `world_to_local`).
#[must_use]
pub fn point_in_local_rect(local: Point, w: f64, h: f64) -> bool {
    local.x >= 0.0 && local.x <= w && local.y >= 0.0 && local.y <= h
}

/// Test if a world point is inside a (possibly rotated) rectangle.
#[must_use]
pub fn point_in_rect(pt: Point, x: f64, y: f64, w: f64, h: f64, rotation_deg: f64) -> bool {
    let local = world_to_local(pt, x, y, w, h, rotation_deg);
    point_in_local_rect(local, w, h)
}

/// Test if a point (in local space) is inside an ellipse inscribed in `[0, w] x [0, h]`.
#[must_use]
pub fn point_in_local_ellipse(local: Point, w: f64, h: f64) -> bool {
    if w <= 0.0 || h <= 0.0 {
        return false;
    }
    let cx = w / 2.0;
    let cy = h / 2.0;
    let dx = (local.x - cx) / cx;
    let dy = (local.y - cy) / cy;
    dx * dx + dy * dy <= 1.0
}

/// Test if a world point is inside a (possibly rotated) ellipse.
#[must_use]
pub fn point_in_ellipse(pt: Point, x: f64, y: f64, w: f64, h: f64, rotation_deg: f64) -> bool {
    let local = world_to_local(pt, x, y, w, h, rotation_deg);
    point_in_local_ellipse(local, w, h)
}

/// Test if a point (in local space) is inside a diamond inscribed in `[0, w] x [0, h]`.
/// The diamond has vertices at the midpoints of each edge.
#[must_use]
pub fn point_in_local_diamond(local: Point, w: f64, h: f64) -> bool {
    if w <= 0.0 || h <= 0.0 {
        return false;
    }
    // Diamond is |dx/cx| + |dy/cy| <= 1 where cx = w/2, cy = h/2.
    let cx = w / 2.0;
    let cy = h / 2.0;
    let dx = (local.x - cx).abs() / cx;
    let dy = (local.y - cy).abs() / cy;
    dx + dy <= 1.0
}

/// Test if a world point is inside a (possibly rotated) diamond.
#[must_use]
pub fn point_in_diamond(pt: Point, x: f64, y: f64, w: f64, h: f64, rotation_deg: f64) -> bool {
    let local = world_to_local(pt, x, y, w, h, rotation_deg);
    point_in_local_diamond(local, w, h)
}

/// Test if a point (in local space) is inside a 5-point star inscribed in `[0, w] x [0, h]`.
/// Inner radius is 0.5 * outer radius. Uses polygon winding.
#[must_use]
pub fn point_in_local_star(local: Point, w: f64, h: f64) -> bool {
    if w <= 0.0 || h <= 0.0 {
        return false;
    }
    let cx = w / 2.0;
    let cy = h / 2.0;
    let outer = (cx, cy);
    let inner = (cx * STAR_INNER_RATIO, cy * STAR_INNER_RATIO);

    // Generate 10-point polygon (alternating outer/inner vertices).
    // Start at top (angle = -90 degrees).
    let step = FRAC_PI_5;
    let offset = std::f64::consts::FRAC_PI_2;
    let mut vertices = [(0.0, 0.0); 10];
    #[allow(clippy::cast_precision_loss)] // i is at most 9
    for (i, vertex) in vertices.iter_mut().enumerate() {
        let angle = step.mul_add(i as f64, -offset);
        let (rx, ry) = if i % 2 == 0 { outer } else { inner };
        *vertex = (cx + rx * angle.cos(), cy + ry * angle.sin());
    }

    point_in_polygon(local.x, local.y, &vertices)
}

/// Test if a world point is inside a (possibly rotated) star.
#[must_use]
pub fn point_in_star(pt: Point, x: f64, y: f64, w: f64, h: f64, rotation_deg: f64) -> bool {
    let local = world_to_local(pt, x, y, w, h, rotation_deg);
    point_in_local_star(local, w, h)
}

/// Ray-casting (even-odd) polygon containment test.
#[must_use]
pub fn point_in_polygon(px: f64, py: f64, vertices: &[(f64, f64)]) -> bool {
    let n = vertices.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = vertices[i];
        let (xj, yj) = vertices[j];
        if ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Squared distance from a point to a line segment (a -> b).
#[must_use]
pub fn distance_sq_to_segment(pt: Point, a: Point, b: Point) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-12 {
        // Degenerate segment (a == b).
        let ex = pt.x - a.x;
        let ey = pt.y - a.y;
        return ex * ex + ey * ey;
    }
    let t = ((pt.x - a.x) * dx + (pt.y - a.y) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);
    let proj_x = a.x + t * dx;
    let proj_y = a.y + t * dy;
    let ex = pt.x - proj_x;
    let ey = pt.y - proj_y;
    ex * ex + ey * ey
}

/// Distance from a point to a line segment.
#[must_use]
pub fn distance_to_segment(pt: Point, a: Point, b: Point) -> f64 {
    distance_sq_to_segment(pt, a, b).sqrt()
}

/// Test if a point is within `radius` of a center point.
#[must_use]
pub fn point_near_point(pt: Point, center: Point, radius: f64) -> bool {
    let dx = pt.x - center.x;
    let dy = pt.y - center.y;
    dx * dx + dy * dy <= radius * radius
}

// =============================================================
// Edge endpoint helpers
// =============================================================

/// Extract endpoint A from a board object's props. Returns None if missing.
#[must_use]
pub fn edge_endpoint_a(obj: &BoardObject) -> Option<Point> {
    let a = obj.props.get("a")?;
    let x = a.get("x")?.as_f64()?;
    let y = a.get("y")?.as_f64()?;
    Some(Point { x, y })
}

/// Extract endpoint B from a board object's props. Returns None if missing.
#[must_use]
pub fn edge_endpoint_b(obj: &BoardObject) -> Option<Point> {
    let b = obj.props.get("b")?;
    let x = b.get("x")?.as_f64()?;
    let y = b.get("y")?.as_f64()?;
    Some(Point { x, y })
}

/// Resolve endpoint A from object props.
///
/// Supports:
/// - free endpoints: `{ "x": f64, "y": f64 }`
/// - attached endpoints:
///   `{ "type": "attached", "object_id": "<uuid>", "ux": f64, "uy": f64 }`
#[must_use]
pub fn edge_endpoint_a_resolved(obj: &BoardObject, doc: &DocStore) -> Option<Point> {
    resolve_edge_endpoint(obj, "a", doc)
}

/// Resolve endpoint B from object props.
///
/// Supports:
/// - free endpoints: `{ "x": f64, "y": f64 }`
/// - attached endpoints:
///   `{ "type": "attached", "object_id": "<uuid>", "ux": f64, "uy": f64 }`
#[must_use]
pub fn edge_endpoint_b_resolved(obj: &BoardObject, doc: &DocStore) -> Option<Point> {
    resolve_edge_endpoint(obj, "b", doc)
}

fn resolve_edge_endpoint(obj: &BoardObject, key: &str, doc: &DocStore) -> Option<Point> {
    let endpoint = obj.props.get(key)?;
    if endpoint.get("type").and_then(serde_json::Value::as_str) == Some("attached") {
        let maybe_attached = endpoint
            .get("object_id")
            .and_then(serde_json::Value::as_str)
            .and_then(|s| uuid::Uuid::parse_str(s).ok())
            .and_then(|object_id| {
                let target = doc.get(&object_id)?;
                let ux = endpoint.get("ux").and_then(serde_json::Value::as_f64)?;
                let uy = endpoint.get("uy").and_then(serde_json::Value::as_f64)?;
                Some(attached_anchor_world_point(target, ux, uy))
            });
        if maybe_attached.is_some() {
            return maybe_attached;
        }
    }

    let x = endpoint.get("x")?.as_f64()?;
    let y = endpoint.get("y")?.as_f64()?;
    Some(Point { x, y })
}

/// Convert normalized local anchor coordinates into world point on an object.
///
/// `ux` and `uy` are in [0, 1] in the object's unrotated local box.
#[must_use]
pub fn attached_anchor_world_point(obj: &BoardObject, ux: f64, uy: f64) -> Point {
    let ux = ux.clamp(0.0, 1.0);
    let uy = uy.clamp(0.0, 1.0);
    let local_world = Point { x: obj.x + (ux * obj.width), y: obj.y + (uy * obj.height) };
    let center = Point { x: obj.x + (obj.width * 0.5), y: obj.y + (obj.height * 0.5) };
    rotate_point(local_world, center, obj.rotation)
}

// =============================================================
// Resize handle positions (in world space)
// =============================================================

/// The 8 resize handle positions for a bounding box, in world coordinates,
/// accounting for rotation.
#[must_use]
pub fn resize_handle_positions(x: f64, y: f64, w: f64, h: f64, rotation_deg: f64) -> [Point; 8] {
    let center = Point { x: x + w / 2.0, y: y + h / 2.0 };
    let local_positions = [
        Point { x: x + w / 2.0, y },        // N
        Point { x: x + w, y },              // Ne
        Point { x: x + w, y: y + h / 2.0 }, // E
        Point { x: x + w, y: y + h },       // Se
        Point { x: x + w / 2.0, y: y + h }, // S
        Point { x, y: y + h },              // Sw
        Point { x, y: y + h / 2.0 },        // W
        Point { x, y },                     // Nw
    ];
    let mut result = [Point { x: 0.0, y: 0.0 }; 8];
    for (i, lp) in local_positions.iter().enumerate() {
        result[i] = rotate_point(*lp, center, rotation_deg);
    }
    result
}

/// The rotate handle position: above the N handle by `ROTATE_HANDLE_OFFSET_PX / zoom` in world space.
#[must_use]
pub fn rotate_handle_position(x: f64, y: f64, w: f64, h: f64, rotation_deg: f64, zoom: f64) -> Point {
    let center = Point { x: x + w / 2.0, y: y + h / 2.0 };
    let offset_world = ROTATE_HANDLE_OFFSET_PX / zoom;
    let local = Point { x: x + w / 2.0, y: y - offset_world };
    rotate_point(local, center, rotation_deg)
}

/// [`ResizeAnchor`] variants in the same order as [`resize_handle_positions`],
/// used to zip positions with their anchor identifiers during hit-testing.
pub const RESIZE_ANCHORS: [ResizeAnchor; 8] = [
    ResizeAnchor::N,
    ResizeAnchor::Ne,
    ResizeAnchor::E,
    ResizeAnchor::Se,
    ResizeAnchor::S,
    ResizeAnchor::Sw,
    ResizeAnchor::W,
    ResizeAnchor::Nw,
];

// =============================================================
// Composite hit test
// =============================================================

/// Test which object (if any) is under `world_pt`.
///
/// If an object is selected, its handles are tested first (resize, rotate,
/// edge endpoints). Then all objects are tested in reverse draw order
/// (top-most first).
#[must_use]
pub fn hit_test(world_pt: Point, doc: &DocStore, camera: &Camera, selected_id: Option<ObjectId>) -> Option<Hit> {
    let handle_radius_world = camera.screen_dist_to_world(HANDLE_RADIUS_PX);

    // 1. Test selected object handles first.
    if let Some(sel_id) = selected_id {
        if let Some(obj) = doc.get(&sel_id) {
            if let Some(hit) = hit_test_handles(world_pt, obj, doc, handle_radius_world, camera.zoom) {
                return Some(hit);
            }
        }
    }

    // 2. Test all objects in reverse draw order (topmost first).
    let sorted = doc.sorted_objects();
    for obj in sorted.iter().rev() {
        if let Some(part) = hit_test_body(world_pt, obj, doc, handle_radius_world) {
            return Some(Hit { object_id: obj.id, part });
        }
    }

    None
}

/// Test handles (resize, rotate, edge endpoints) on a single selected object.
fn hit_test_handles(world_pt: Point, obj: &BoardObject, doc: &DocStore, radius: f64, zoom: f64) -> Option<Hit> {
    match obj.kind {
        ObjectKind::Line | ObjectKind::Arrow => {
            // Edge endpoints.
            if let Some(a) = edge_endpoint_a_resolved(obj, doc) {
                if point_near_point(world_pt, a, radius) {
                    return Some(Hit { object_id: obj.id, part: HitPart::EdgeEndpoint(EdgeEnd::A) });
                }
            }
            if let Some(b) = edge_endpoint_b_resolved(obj, doc) {
                if point_near_point(world_pt, b, radius) {
                    return Some(Hit { object_id: obj.id, part: HitPart::EdgeEndpoint(EdgeEnd::B) });
                }
            }
            None
        }
        _ => {
            // Rotate handle.
            let rh = rotate_handle_position(obj.x, obj.y, obj.width, obj.height, obj.rotation, zoom);
            if point_near_point(world_pt, rh, radius) {
                return Some(Hit { object_id: obj.id, part: HitPart::RotateHandle });
            }

            // Resize handles.
            let handles = resize_handle_positions(obj.x, obj.y, obj.width, obj.height, obj.rotation);
            for (i, handle_pos) in handles.iter().enumerate() {
                if point_near_point(world_pt, *handle_pos, radius) {
                    return Some(Hit { object_id: obj.id, part: HitPart::ResizeHandle(RESIZE_ANCHORS[i]) });
                }
            }
            None
        }
    }
}

/// Test the body/interior of a single object.
fn hit_test_body(world_pt: Point, obj: &BoardObject, doc: &DocStore, edge_radius: f64) -> Option<HitPart> {
    match obj.kind {
        ObjectKind::Rect => {
            if point_in_rect(world_pt, obj.x, obj.y, obj.width, obj.height, obj.rotation) {
                Some(HitPart::Body)
            } else {
                None
            }
        }
        ObjectKind::Ellipse => {
            if point_in_ellipse(world_pt, obj.x, obj.y, obj.width, obj.height, obj.rotation) {
                Some(HitPart::Body)
            } else {
                None
            }
        }
        ObjectKind::Diamond => {
            if point_in_diamond(world_pt, obj.x, obj.y, obj.width, obj.height, obj.rotation) {
                Some(HitPart::Body)
            } else {
                None
            }
        }
        ObjectKind::Star => {
            if point_in_star(world_pt, obj.x, obj.y, obj.width, obj.height, obj.rotation) {
                Some(HitPart::Body)
            } else {
                None
            }
        }
        ObjectKind::Line | ObjectKind::Arrow => {
            let a = edge_endpoint_a_resolved(obj, doc)?;
            let b = edge_endpoint_b_resolved(obj, doc)?;
            if distance_to_segment(world_pt, a, b) <= edge_radius {
                Some(HitPart::EdgeBody)
            } else {
                None
            }
        }
    }
}
