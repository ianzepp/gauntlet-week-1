use serde_json::json;
use uuid::Uuid;

use super::*;
use crate::camera::Camera;
use crate::doc::{BoardObject, DocStore, ObjectKind};

const EPSILON: f64 = 1e-6;

fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() < EPSILON
}

fn make_node(kind: ObjectKind, x: f64, y: f64, w: f64, h: f64, rotation: f64) -> BoardObject {
    BoardObject {
        id: Uuid::new_v4(),
        board_id: Uuid::new_v4(),
        kind,
        x,
        y,
        width: w,
        height: h,
        rotation,
        z_index: 0,
        props: json!({}),
        created_by: None,
        version: 1,
    }
}

fn make_edge(kind: ObjectKind, ax: f64, ay: f64, bx: f64, by: f64) -> BoardObject {
    BoardObject {
        id: Uuid::new_v4(),
        board_id: Uuid::new_v4(),
        kind,
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
        rotation: 0.0,
        z_index: 0,
        props: json!({
            "a": {"type": "free", "x": ax, "y": ay},
            "b": {"type": "free", "x": bx, "y": by}
        }),
        created_by: None,
        version: 1,
    }
}

// =============================================================
// rotate_point
// =============================================================

#[test]
fn rotate_point_zero_degrees() {
    let p = rotate_point(Point::new(10.0, 0.0), Point::new(0.0, 0.0), 0.0);
    assert!(approx_eq(p.x, 10.0));
    assert!(approx_eq(p.y, 0.0));
}

#[test]
fn rotate_point_90_degrees() {
    let p = rotate_point(Point::new(10.0, 0.0), Point::new(0.0, 0.0), 90.0);
    assert!(approx_eq(p.x, 0.0));
    assert!(approx_eq(p.y, 10.0));
}

#[test]
fn rotate_point_180_degrees() {
    let p = rotate_point(Point::new(10.0, 0.0), Point::new(0.0, 0.0), 180.0);
    assert!(approx_eq(p.x, -10.0));
    assert!(approx_eq(p.y, 0.0));
}

#[test]
fn rotate_point_270_degrees() {
    let p = rotate_point(Point::new(10.0, 0.0), Point::new(0.0, 0.0), 270.0);
    assert!(approx_eq(p.x, 0.0));
    assert!(approx_eq(p.y, -10.0));
}

#[test]
fn rotate_point_360_is_identity() {
    let p = rotate_point(Point::new(3.0, 4.0), Point::new(1.0, 2.0), 360.0);
    assert!(approx_eq(p.x, 3.0));
    assert!(approx_eq(p.y, 4.0));
}

#[test]
fn rotate_point_around_nonzero_origin() {
    // Rotate (10, 5) around (5, 5) by 90 degrees => (5, 10)
    let p = rotate_point(Point::new(10.0, 5.0), Point::new(5.0, 5.0), 90.0);
    assert!(approx_eq(p.x, 5.0));
    assert!(approx_eq(p.y, 10.0));
}

#[test]
fn rotate_point_negative_angle() {
    let p = rotate_point(Point::new(10.0, 0.0), Point::new(0.0, 0.0), -90.0);
    assert!(approx_eq(p.x, 0.0));
    assert!(approx_eq(p.y, -10.0));
}

#[test]
fn rotate_point_45_degrees() {
    let p = rotate_point(Point::new(1.0, 0.0), Point::new(0.0, 0.0), 45.0);
    let expected = std::f64::consts::FRAC_1_SQRT_2;
    assert!(approx_eq(p.x, expected));
    assert!(approx_eq(p.y, expected));
}

// =============================================================
// world_to_local
// =============================================================

#[test]
fn world_to_local_no_rotation() {
    let local = world_to_local(Point::new(50.0, 40.0), 0.0, 0.0, 100.0, 80.0, 0.0);
    assert!(approx_eq(local.x, 50.0));
    assert!(approx_eq(local.y, 40.0));
}

#[test]
fn world_to_local_with_offset() {
    let local = world_to_local(Point::new(110.0, 220.0), 100.0, 200.0, 100.0, 80.0, 0.0);
    assert!(approx_eq(local.x, 10.0));
    assert!(approx_eq(local.y, 20.0));
}

#[test]
fn world_to_local_center_stays_center() {
    // Center of a box at (100, 200) with w=60, h=40 is (130, 220).
    // In local space, center should be (30, 20) regardless of rotation.
    for angle in [0.0, 45.0, 90.0, 135.0, 180.0, 270.0] {
        let local = world_to_local(Point::new(130.0, 220.0), 100.0, 200.0, 60.0, 40.0, angle);
        assert!(approx_eq(local.x, 30.0), "angle={angle}: local.x={}", local.x);
        assert!(approx_eq(local.y, 20.0), "angle={angle}: local.y={}", local.y);
    }
}

// =============================================================
// point_in_rect
// =============================================================

#[test]
fn rect_center_hit() {
    assert!(point_in_rect(Point::new(50.0, 40.0), 0.0, 0.0, 100.0, 80.0, 0.0));
}

#[test]
fn rect_corner_hit() {
    assert!(point_in_rect(Point::new(0.0, 0.0), 0.0, 0.0, 100.0, 80.0, 0.0));
    assert!(point_in_rect(Point::new(100.0, 80.0), 0.0, 0.0, 100.0, 80.0, 0.0));
}

#[test]
fn rect_outside_miss() {
    assert!(!point_in_rect(Point::new(-1.0, 40.0), 0.0, 0.0, 100.0, 80.0, 0.0));
    assert!(!point_in_rect(Point::new(101.0, 40.0), 0.0, 0.0, 100.0, 80.0, 0.0));
    assert!(!point_in_rect(Point::new(50.0, -1.0), 0.0, 0.0, 100.0, 80.0, 0.0));
    assert!(!point_in_rect(Point::new(50.0, 81.0), 0.0, 0.0, 100.0, 80.0, 0.0));
}

#[test]
fn rect_with_offset() {
    assert!(point_in_rect(Point::new(150.0, 250.0), 100.0, 200.0, 100.0, 80.0, 0.0));
    assert!(!point_in_rect(Point::new(99.0, 250.0), 100.0, 200.0, 100.0, 80.0, 0.0));
}

#[test]
fn rect_rotated_90() {
    // A 100x80 rect at origin rotated 90 degrees around its center (50, 40).
    // After rotation, the rect spans roughly (10, -10) to (90, 90) in world space.
    // Center should still hit.
    assert!(point_in_rect(Point::new(50.0, 40.0), 0.0, 0.0, 100.0, 80.0, 90.0));
    // A point that was inside unrotated but outside rotated should miss.
    assert!(!point_in_rect(Point::new(5.0, 5.0), 0.0, 0.0, 100.0, 80.0, 90.0));
}

#[test]
fn rect_rotated_45_center_hits() {
    assert!(point_in_rect(Point::new(50.0, 40.0), 0.0, 0.0, 100.0, 80.0, 45.0));
}

// =============================================================
// point_in_ellipse
// =============================================================

#[test]
fn ellipse_center_hit() {
    assert!(point_in_ellipse(Point::new(50.0, 40.0), 0.0, 0.0, 100.0, 80.0, 0.0));
}

#[test]
fn ellipse_on_edge_hit() {
    // Right edge: (100, 40) is on the ellipse boundary.
    assert!(point_in_ellipse(Point::new(100.0, 40.0), 0.0, 0.0, 100.0, 80.0, 0.0));
}

#[test]
fn ellipse_corner_miss() {
    // Corners of the bounding box are outside the ellipse.
    assert!(!point_in_ellipse(Point::new(0.0, 0.0), 0.0, 0.0, 100.0, 80.0, 0.0));
    assert!(!point_in_ellipse(Point::new(100.0, 80.0), 0.0, 0.0, 100.0, 80.0, 0.0));
}

#[test]
fn ellipse_outside_miss() {
    assert!(!point_in_ellipse(Point::new(-1.0, 40.0), 0.0, 0.0, 100.0, 80.0, 0.0));
    assert!(!point_in_ellipse(Point::new(101.0, 40.0), 0.0, 0.0, 100.0, 80.0, 0.0));
}

#[test]
fn ellipse_zero_size_miss() {
    assert!(!point_in_ellipse(Point::new(0.0, 0.0), 0.0, 0.0, 0.0, 0.0, 0.0));
}

#[test]
fn ellipse_rotated_center_hits() {
    assert!(point_in_ellipse(Point::new(50.0, 40.0), 0.0, 0.0, 100.0, 80.0, 45.0));
}

#[test]
fn ellipse_with_offset() {
    assert!(point_in_ellipse(Point::new(150.0, 240.0), 100.0, 200.0, 100.0, 80.0, 0.0));
}

#[test]
fn ellipse_just_inside() {
    // Point just inside right edge.
    assert!(point_in_ellipse(Point::new(99.0, 40.0), 0.0, 0.0, 100.0, 80.0, 0.0));
}

#[test]
fn ellipse_just_outside() {
    // Point just outside right edge.
    assert!(!point_in_ellipse(Point::new(101.0, 40.0), 0.0, 0.0, 100.0, 80.0, 0.0));
}

// =============================================================
// point_in_diamond
// =============================================================

#[test]
fn diamond_center_hit() {
    assert!(point_in_diamond(Point::new(50.0, 40.0), 0.0, 0.0, 100.0, 80.0, 0.0));
}

#[test]
fn diamond_vertex_hit() {
    // Top vertex: (50, 0)
    assert!(point_in_diamond(Point::new(50.0, 0.0), 0.0, 0.0, 100.0, 80.0, 0.0));
    // Right vertex: (100, 40)
    assert!(point_in_diamond(Point::new(100.0, 40.0), 0.0, 0.0, 100.0, 80.0, 0.0));
}

#[test]
fn diamond_corner_miss() {
    // Corners of bounding box should be outside.
    assert!(!point_in_diamond(Point::new(0.0, 0.0), 0.0, 0.0, 100.0, 80.0, 0.0));
    assert!(!point_in_diamond(Point::new(100.0, 0.0), 0.0, 0.0, 100.0, 80.0, 0.0));
    assert!(!point_in_diamond(Point::new(100.0, 80.0), 0.0, 0.0, 100.0, 80.0, 0.0));
    assert!(!point_in_diamond(Point::new(0.0, 80.0), 0.0, 0.0, 100.0, 80.0, 0.0));
}

#[test]
fn diamond_zero_size_miss() {
    assert!(!point_in_diamond(Point::new(0.0, 0.0), 0.0, 0.0, 0.0, 0.0, 0.0));
}

#[test]
fn diamond_rotated_center_hits() {
    assert!(point_in_diamond(Point::new(50.0, 40.0), 0.0, 0.0, 100.0, 80.0, 30.0));
}

#[test]
fn diamond_on_edge() {
    // Midpoint of top-right edge: halfway between (50, 0) and (100, 40) = (75, 20)
    assert!(point_in_diamond(Point::new(75.0, 20.0), 0.0, 0.0, 100.0, 80.0, 0.0));
}

// =============================================================
// point_in_star
// =============================================================

#[test]
fn star_center_hit() {
    assert!(point_in_local_star(Point::new(50.0, 50.0), 100.0, 100.0));
}

#[test]
fn star_tip_hit() {
    // Just inside the top tip of a 100x100 star (exact vertex is on boundary).
    assert!(point_in_local_star(Point::new(50.0, 1.0), 100.0, 100.0));
}

#[test]
fn star_corner_miss() {
    // Corner of bounding box should be outside the star.
    assert!(!point_in_local_star(Point::new(0.0, 0.0), 100.0, 100.0));
    assert!(!point_in_local_star(Point::new(100.0, 0.0), 100.0, 100.0));
}

#[test]
fn star_between_points_miss() {
    // In the concavity between two star points — should be outside.
    // The concavity near top-right: around (80, 15) should miss for a 100x100 star.
    assert!(!point_in_local_star(Point::new(85.0, 10.0), 100.0, 100.0));
}

#[test]
fn star_zero_size_miss() {
    assert!(!point_in_local_star(Point::new(0.0, 0.0), 0.0, 0.0));
}

#[test]
fn star_world_rotated_center_hits() {
    assert!(point_in_star(Point::new(50.0, 50.0), 0.0, 0.0, 100.0, 100.0, 72.0));
}

#[test]
fn star_non_square_center_hit() {
    // Star inscribed in a wide rectangle.
    assert!(point_in_local_star(Point::new(100.0, 40.0), 200.0, 80.0));
}

// =============================================================
// point_in_polygon
// =============================================================

#[test]
fn polygon_triangle_inside() {
    let tri = [(0.0, 0.0), (10.0, 0.0), (5.0, 10.0)];
    assert!(point_in_polygon(5.0, 3.0, &tri));
}

#[test]
fn polygon_triangle_outside() {
    let tri = [(0.0, 0.0), (10.0, 0.0), (5.0, 10.0)];
    assert!(!point_in_polygon(0.0, 10.0, &tri));
}

#[test]
fn polygon_degenerate_line_miss() {
    let line = [(0.0, 0.0), (10.0, 0.0)];
    assert!(!point_in_polygon(5.0, 0.0, &line));
}

#[test]
fn polygon_empty_miss() {
    assert!(!point_in_polygon(0.0, 0.0, &[]));
}

#[test]
fn polygon_square_inside() {
    let sq = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
    assert!(point_in_polygon(5.0, 5.0, &sq));
}

#[test]
fn polygon_square_outside() {
    let sq = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
    assert!(!point_in_polygon(11.0, 5.0, &sq));
}

// =============================================================
// distance_to_segment / distance_sq_to_segment
// =============================================================

#[test]
fn segment_point_on_segment() {
    let d = distance_to_segment(Point::new(5.0, 0.0), Point::new(0.0, 0.0), Point::new(10.0, 0.0));
    assert!(approx_eq(d, 0.0));
}

#[test]
fn segment_point_perpendicular() {
    let d = distance_to_segment(Point::new(5.0, 3.0), Point::new(0.0, 0.0), Point::new(10.0, 0.0));
    assert!(approx_eq(d, 3.0));
}

#[test]
fn segment_point_past_end_a() {
    let d = distance_to_segment(Point::new(-3.0, 0.0), Point::new(0.0, 0.0), Point::new(10.0, 0.0));
    assert!(approx_eq(d, 3.0));
}

#[test]
fn segment_point_past_end_b() {
    let d = distance_to_segment(Point::new(13.0, 0.0), Point::new(0.0, 0.0), Point::new(10.0, 0.0));
    assert!(approx_eq(d, 3.0));
}

#[test]
fn segment_degenerate_point() {
    let d = distance_to_segment(Point::new(3.0, 4.0), Point::new(0.0, 0.0), Point::new(0.0, 0.0));
    assert!(approx_eq(d, 5.0));
}

#[test]
fn segment_diagonal() {
    // Segment from (0,0) to (10,10), point at (10,0) -> distance = 10/sqrt(2).
    let d = distance_to_segment(Point::new(10.0, 0.0), Point::new(0.0, 0.0), Point::new(10.0, 10.0));
    assert!(approx_eq(d, 10.0 / std::f64::consts::SQRT_2));
}

#[test]
fn segment_point_at_start() {
    let d = distance_to_segment(Point::new(0.0, 0.0), Point::new(0.0, 0.0), Point::new(10.0, 0.0));
    assert!(approx_eq(d, 0.0));
}

#[test]
fn segment_point_at_end() {
    let d = distance_to_segment(Point::new(10.0, 0.0), Point::new(0.0, 0.0), Point::new(10.0, 0.0));
    assert!(approx_eq(d, 0.0));
}

#[test]
fn distance_sq_matches_distance() {
    let a = Point::new(0.0, 0.0);
    let b = Point::new(10.0, 0.0);
    let pt = Point::new(5.0, 3.0);
    let dsq = distance_sq_to_segment(pt, a, b);
    let d = distance_to_segment(pt, a, b);
    assert!(approx_eq(dsq, d * d));
}

// =============================================================
// point_near_point
// =============================================================

#[test]
fn near_point_exact_center() {
    assert!(point_near_point(Point::new(5.0, 5.0), Point::new(5.0, 5.0), 1.0));
}

#[test]
fn near_point_on_radius() {
    assert!(point_near_point(Point::new(6.0, 5.0), Point::new(5.0, 5.0), 1.0));
}

#[test]
fn near_point_just_outside() {
    assert!(!point_near_point(Point::new(6.01, 5.0), Point::new(5.0, 5.0), 1.0));
}

#[test]
fn near_point_zero_radius() {
    assert!(point_near_point(Point::new(5.0, 5.0), Point::new(5.0, 5.0), 0.0));
    assert!(!point_near_point(Point::new(5.1, 5.0), Point::new(5.0, 5.0), 0.0));
}

// =============================================================
// edge_endpoint_a / edge_endpoint_b
// =============================================================

#[test]
fn edge_endpoint_a_present() {
    let obj = make_edge(ObjectKind::Line, 10.0, 20.0, 100.0, 200.0);
    let a = edge_endpoint_a(&obj).unwrap();
    assert!(approx_eq(a.x, 10.0));
    assert!(approx_eq(a.y, 20.0));
}

#[test]
fn edge_endpoint_b_present() {
    let obj = make_edge(ObjectKind::Arrow, 10.0, 20.0, 100.0, 200.0);
    let b = edge_endpoint_b(&obj).unwrap();
    assert!(approx_eq(b.x, 100.0));
    assert!(approx_eq(b.y, 200.0));
}

#[test]
fn edge_endpoint_a_missing() {
    let obj = make_node(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0, 0.0);
    assert!(edge_endpoint_a(&obj).is_none());
}

#[test]
fn edge_endpoint_b_missing() {
    let obj = make_node(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0, 0.0);
    assert!(edge_endpoint_b(&obj).is_none());
}

#[test]
fn edge_endpoint_partial_props() {
    let obj = BoardObject {
        id: Uuid::new_v4(),
        board_id: Uuid::new_v4(),
        kind: ObjectKind::Line,
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
        rotation: 0.0,
        z_index: 0,
        props: json!({"a": {"type": "free", "x": 5.0}}), // missing y
        created_by: None,
        version: 1,
    };
    assert!(edge_endpoint_a(&obj).is_none());
}

// =============================================================
// resize_handle_positions
// =============================================================

#[test]
fn resize_handles_no_rotation() {
    let handles = resize_handle_positions(0.0, 0.0, 100.0, 80.0, 0.0);
    // N: (50, 0)
    assert!(approx_eq(handles[0].x, 50.0));
    assert!(approx_eq(handles[0].y, 0.0));
    // Se: (100, 80)
    assert!(approx_eq(handles[3].x, 100.0));
    assert!(approx_eq(handles[3].y, 80.0));
    // Nw: (0, 0)
    assert!(approx_eq(handles[7].x, 0.0));
    assert!(approx_eq(handles[7].y, 0.0));
}

#[test]
fn resize_handles_with_offset() {
    let handles = resize_handle_positions(100.0, 200.0, 60.0, 40.0, 0.0);
    // N: (130, 200)
    assert!(approx_eq(handles[0].x, 130.0));
    assert!(approx_eq(handles[0].y, 200.0));
    // Se: (160, 240)
    assert!(approx_eq(handles[3].x, 160.0));
    assert!(approx_eq(handles[3].y, 240.0));
}

#[test]
fn resize_handles_rotation_preserves_center_distance() {
    // After rotation, each handle should be the same distance from center.
    let handles_0 = resize_handle_positions(0.0, 0.0, 100.0, 80.0, 0.0);
    let handles_45 = resize_handle_positions(0.0, 0.0, 100.0, 80.0, 45.0);
    let center = Point::new(50.0, 40.0);
    for i in 0..8 {
        let d0 = ((handles_0[i].x - center.x).powi(2) + (handles_0[i].y - center.y).powi(2)).sqrt();
        let d45 = ((handles_45[i].x - center.x).powi(2) + (handles_45[i].y - center.y).powi(2)).sqrt();
        assert!(approx_eq(d0, d45), "handle {i}: d0={d0}, d45={d45}");
    }
}

// =============================================================
// rotate_handle_position
// =============================================================

#[test]
fn rotate_handle_above_n() {
    let rh = rotate_handle_position(0.0, 0.0, 100.0, 80.0, 0.0, 1.0);
    // Should be at (50, -ROTATE_HANDLE_OFFSET_PX) at zoom 1.
    assert!(approx_eq(rh.x, 50.0));
    assert!(approx_eq(rh.y, -ROTATE_HANDLE_OFFSET_PX));
}

#[test]
fn rotate_handle_zoom_scales_offset() {
    let rh1 = rotate_handle_position(0.0, 0.0, 100.0, 80.0, 0.0, 1.0);
    let rh2 = rotate_handle_position(0.0, 0.0, 100.0, 80.0, 0.0, 2.0);
    // At zoom 2, the offset in world units is half.
    let expected_y_z1 = -ROTATE_HANDLE_OFFSET_PX;
    let expected_y_z2 = -ROTATE_HANDLE_OFFSET_PX / 2.0;
    assert!(approx_eq(rh1.y, expected_y_z1));
    assert!(approx_eq(rh2.y, expected_y_z2));
}

// =============================================================
// Composite hit_test: node shapes
// =============================================================

#[test]
fn hit_test_rect_body() {
    let mut doc = DocStore::new();
    let obj = make_node(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0, 0.0);
    let id = obj.id;
    doc.insert(obj);
    let cam = Camera::default();

    let hit = hit_test(Point::new(50.0, 40.0), &doc, &cam, None);
    assert!(hit.is_some());
    let h = hit.unwrap();
    assert_eq!(h.object_id, id);
    assert_eq!(h.part, HitPart::Body);
}

#[test]
fn hit_test_rect_miss() {
    let mut doc = DocStore::new();
    doc.insert(make_node(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0, 0.0));
    let cam = Camera::default();

    let hit = hit_test(Point::new(200.0, 200.0), &doc, &cam, None);
    assert!(hit.is_none());
}

#[test]
fn hit_test_ellipse_body() {
    let mut doc = DocStore::new();
    let obj = make_node(ObjectKind::Ellipse, 0.0, 0.0, 100.0, 80.0, 0.0);
    let id = obj.id;
    doc.insert(obj);
    let cam = Camera::default();

    let hit = hit_test(Point::new(50.0, 40.0), &doc, &cam, None);
    assert!(hit.is_some());
    assert_eq!(hit.unwrap().object_id, id);
}

#[test]
fn hit_test_ellipse_corner_miss() {
    let mut doc = DocStore::new();
    doc.insert(make_node(ObjectKind::Ellipse, 0.0, 0.0, 100.0, 80.0, 0.0));
    let cam = Camera::default();

    // Bounding box corner — outside the ellipse.
    let hit = hit_test(Point::new(2.0, 2.0), &doc, &cam, None);
    assert!(hit.is_none());
}

#[test]
fn hit_test_diamond_body() {
    let mut doc = DocStore::new();
    let obj = make_node(ObjectKind::Diamond, 0.0, 0.0, 100.0, 80.0, 0.0);
    let id = obj.id;
    doc.insert(obj);
    let cam = Camera::default();

    let hit = hit_test(Point::new(50.0, 40.0), &doc, &cam, None);
    assert!(hit.is_some());
    assert_eq!(hit.unwrap().object_id, id);
}

#[test]
fn hit_test_star_body() {
    let mut doc = DocStore::new();
    let obj = make_node(ObjectKind::Star, 0.0, 0.0, 100.0, 100.0, 0.0);
    let id = obj.id;
    doc.insert(obj);
    let cam = Camera::default();

    let hit = hit_test(Point::new(50.0, 50.0), &doc, &cam, None);
    assert!(hit.is_some());
    assert_eq!(hit.unwrap().object_id, id);
}

// =============================================================
// Composite hit_test: edges
// =============================================================

#[test]
fn hit_test_line_body() {
    let mut doc = DocStore::new();
    let obj = make_edge(ObjectKind::Line, 0.0, 0.0, 100.0, 0.0);
    let id = obj.id;
    doc.insert(obj);
    let cam = Camera::default();

    // Point on the line.
    let hit = hit_test(Point::new(50.0, 0.0), &doc, &cam, None);
    assert!(hit.is_some());
    let h = hit.unwrap();
    assert_eq!(h.object_id, id);
    assert_eq!(h.part, HitPart::EdgeBody);
}

#[test]
fn hit_test_line_near_body() {
    let mut doc = DocStore::new();
    let obj = make_edge(ObjectKind::Line, 0.0, 0.0, 100.0, 0.0);
    let id = obj.id;
    doc.insert(obj);
    let cam = Camera::default();

    // Point slightly off the line (within handle radius).
    let hit = hit_test(Point::new(50.0, 5.0), &doc, &cam, None);
    assert!(hit.is_some());
    assert_eq!(hit.unwrap().object_id, id);
}

#[test]
fn hit_test_line_far_miss() {
    let mut doc = DocStore::new();
    doc.insert(make_edge(ObjectKind::Line, 0.0, 0.0, 100.0, 0.0));
    let cam = Camera::default();

    let hit = hit_test(Point::new(50.0, 50.0), &doc, &cam, None);
    assert!(hit.is_none());
}

#[test]
fn hit_test_arrow_body() {
    let mut doc = DocStore::new();
    let obj = make_edge(ObjectKind::Arrow, 10.0, 10.0, 200.0, 10.0);
    let id = obj.id;
    doc.insert(obj);
    let cam = Camera::default();

    let hit = hit_test(Point::new(100.0, 10.0), &doc, &cam, None);
    assert!(hit.is_some());
    assert_eq!(hit.unwrap().object_id, id);
    assert_eq!(hit.unwrap().part, HitPart::EdgeBody);
}

// =============================================================
// Composite hit_test: handle priority
// =============================================================

#[test]
fn hit_test_selected_resize_handle() {
    let mut doc = DocStore::new();
    let obj = make_node(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0, 0.0);
    let id = obj.id;
    doc.insert(obj);
    let cam = Camera::default();

    // Se handle is at (100, 80). Click right on it.
    let hit = hit_test(Point::new(100.0, 80.0), &doc, &cam, Some(id));
    assert!(hit.is_some());
    let h = hit.unwrap();
    assert_eq!(h.object_id, id);
    assert_eq!(h.part, HitPart::ResizeHandle(ResizeAnchor::Se));
}

#[test]
fn hit_test_selected_rotate_handle() {
    let mut doc = DocStore::new();
    let obj = make_node(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0, 0.0);
    let id = obj.id;
    doc.insert(obj);
    let cam = Camera::default();

    // Rotate handle at (50, -ROTATE_HANDLE_OFFSET_PX) at zoom 1.
    let hit = hit_test(Point::new(50.0, -ROTATE_HANDLE_OFFSET_PX), &doc, &cam, Some(id));
    assert!(hit.is_some());
    let h = hit.unwrap();
    assert_eq!(h.object_id, id);
    assert_eq!(h.part, HitPart::RotateHandle);
}

#[test]
fn hit_test_selected_edge_endpoint_a() {
    let mut doc = DocStore::new();
    let obj = make_edge(ObjectKind::Line, 10.0, 20.0, 200.0, 150.0);
    let id = obj.id;
    doc.insert(obj);
    let cam = Camera::default();

    let hit = hit_test(Point::new(10.0, 20.0), &doc, &cam, Some(id));
    assert!(hit.is_some());
    let h = hit.unwrap();
    assert_eq!(h.object_id, id);
    assert_eq!(h.part, HitPart::EdgeEndpoint(EdgeEnd::A));
}

#[test]
fn hit_test_selected_edge_endpoint_b() {
    let mut doc = DocStore::new();
    let obj = make_edge(ObjectKind::Arrow, 10.0, 20.0, 200.0, 150.0);
    let id = obj.id;
    doc.insert(obj);
    let cam = Camera::default();

    let hit = hit_test(Point::new(200.0, 150.0), &doc, &cam, Some(id));
    assert!(hit.is_some());
    let h = hit.unwrap();
    assert_eq!(h.object_id, id);
    assert_eq!(h.part, HitPart::EdgeEndpoint(EdgeEnd::B));
}

#[test]
fn hit_test_handles_only_for_selected() {
    let mut doc = DocStore::new();
    let obj = make_node(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0, 0.0);
    let id = obj.id;
    doc.insert(obj);
    let cam = Camera::default();

    // Click on Se handle position, but object is NOT selected.
    // Should get Body hit, not ResizeHandle.
    let hit = hit_test(Point::new(100.0, 80.0), &doc, &cam, None);
    assert!(hit.is_some());
    let h = hit.unwrap();
    assert_eq!(h.object_id, id);
    assert_eq!(h.part, HitPart::Body);
}

// =============================================================
// Composite hit_test: draw order (topmost wins)
// =============================================================

#[test]
fn hit_test_topmost_wins() {
    let mut doc = DocStore::new();
    let bottom = BoardObject { z_index: 0, ..make_node(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0, 0.0) };
    let top = BoardObject { z_index: 1, ..make_node(ObjectKind::Rect, 0.0, 0.0, 100.0, 80.0, 0.0) };
    let top_id = top.id;
    doc.insert(bottom);
    doc.insert(top);
    let cam = Camera::default();

    let hit = hit_test(Point::new(50.0, 40.0), &doc, &cam, None);
    assert!(hit.is_some());
    assert_eq!(hit.unwrap().object_id, top_id);
}

#[test]
fn hit_test_empty_doc() {
    let doc = DocStore::new();
    let cam = Camera::default();
    assert!(hit_test(Point::new(50.0, 50.0), &doc, &cam, None).is_none());
}

// =============================================================
// Composite hit_test: zoom affects edge hit radius
// =============================================================

#[test]
fn hit_test_edge_zoom_affects_radius() {
    let mut doc = DocStore::new();
    let obj = make_edge(ObjectKind::Line, 0.0, 0.0, 100.0, 0.0);
    doc.insert(obj);

    // At zoom 1, handle radius is 8px in world. Point at y=7 should hit.
    let cam1 = Camera { zoom: 1.0, ..Camera::default() };
    assert!(hit_test(Point::new(50.0, 7.0), &doc, &cam1, None).is_some());

    // At zoom 4, handle radius is 2px in world. Point at y=7 should miss.
    let cam4 = Camera { zoom: 4.0, ..Camera::default() };
    assert!(hit_test(Point::new(50.0, 7.0), &doc, &cam4, None).is_none());
}

// =============================================================
// HitPart / ResizeAnchor / EdgeEnd (type tests from original)
// =============================================================

#[test]
fn hit_part_body_equality() {
    assert_eq!(HitPart::Body, HitPart::Body);
}

#[test]
fn hit_part_resize_handle_equality() {
    assert_eq!(HitPart::ResizeHandle(ResizeAnchor::N), HitPart::ResizeHandle(ResizeAnchor::N));
    assert_ne!(HitPart::ResizeHandle(ResizeAnchor::N), HitPart::ResizeHandle(ResizeAnchor::S));
}

#[test]
fn hit_part_variants_distinct() {
    assert_ne!(HitPart::Body, HitPart::RotateHandle);
    assert_ne!(HitPart::Body, HitPart::EdgeBody);
    assert_ne!(HitPart::RotateHandle, HitPart::EdgeBody);
    assert_ne!(HitPart::EdgeEndpoint(EdgeEnd::A), HitPart::EdgeEndpoint(EdgeEnd::B));
    assert_ne!(HitPart::EdgeEndpoint(EdgeEnd::A), HitPart::Body);
}

#[test]
fn hit_part_debug_format() {
    let s = format!("{:?}", HitPart::Body);
    assert_eq!(s, "Body");
    let s = format!("{:?}", HitPart::ResizeHandle(ResizeAnchor::Ne));
    assert!(s.contains("Ne"));
}

#[test]
fn resize_anchor_all_variants_distinct() {
    let variants = [
        ResizeAnchor::N,
        ResizeAnchor::Ne,
        ResizeAnchor::E,
        ResizeAnchor::Se,
        ResizeAnchor::S,
        ResizeAnchor::Sw,
        ResizeAnchor::W,
        ResizeAnchor::Nw,
    ];
    for (i, a) in variants.iter().enumerate() {
        for (j, b) in variants.iter().enumerate() {
            if i == j {
                assert_eq!(a, b);
            } else {
                assert_ne!(a, b);
            }
        }
    }
}

#[test]
fn resize_anchor_clone_and_copy() {
    let a = ResizeAnchor::Se;
    let b = a;
    let c = a.clone();
    assert_eq!(a, b);
    assert_eq!(a, c);
}

#[test]
fn edge_end_equality() {
    assert_eq!(EdgeEnd::A, EdgeEnd::A);
    assert_eq!(EdgeEnd::B, EdgeEnd::B);
    assert_ne!(EdgeEnd::A, EdgeEnd::B);
}

#[test]
fn edge_end_clone_and_copy() {
    let a = EdgeEnd::A;
    let b = a;
    assert_eq!(a, b);
}

#[test]
fn hit_stores_object_id_and_part() {
    let id = Uuid::new_v4();
    let hit = Hit { object_id: id, part: HitPart::Body };
    assert_eq!(hit.object_id, id);
    assert_eq!(hit.part, HitPart::Body);
}

#[test]
fn hit_with_resize_handle() {
    let id = Uuid::new_v4();
    let hit = Hit { object_id: id, part: HitPart::ResizeHandle(ResizeAnchor::Nw) };
    assert_eq!(hit.part, HitPart::ResizeHandle(ResizeAnchor::Nw));
}

#[test]
fn hit_with_edge_endpoint() {
    let id = Uuid::new_v4();
    let hit = Hit { object_id: id, part: HitPart::EdgeEndpoint(EdgeEnd::B) };
    assert_eq!(hit.part, HitPart::EdgeEndpoint(EdgeEnd::B));
}

#[test]
fn hit_debug_format() {
    let id = Uuid::nil();
    let hit = Hit { object_id: id, part: HitPart::RotateHandle };
    let s = format!("{hit:?}");
    assert!(s.contains("RotateHandle"));
}

#[test]
fn hit_clone() {
    let id = Uuid::new_v4();
    let hit = Hit { object_id: id, part: HitPart::EdgeBody };
    let hit2 = hit;
    assert_eq!(hit2.object_id, id);
    assert_eq!(hit2.part, HitPart::EdgeBody);
}
