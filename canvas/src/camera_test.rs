#![allow(clippy::clone_on_copy, clippy::float_cmp)]

use super::*;

const EPSILON: f64 = 1e-10;

fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() < EPSILON
}

fn point_approx_eq(a: Point, b: Point) -> bool {
    approx_eq(a.x, b.x) && approx_eq(a.y, b.y)
}

// --- Point ---

#[test]
fn point_new() {
    let p = Point::new(3.0, 4.0);
    assert_eq!(p.x, 3.0);
    assert_eq!(p.y, 4.0);
}

#[test]
fn point_clone() {
    let p = Point::new(1.0, 2.0);
    let q = p;
    assert!(point_approx_eq(p, q));
}

#[test]
fn point_equality() {
    let a = Point::new(1.0, 2.0);
    let b = Point::new(1.0, 2.0);
    assert_eq!(a, b);
}

#[test]
fn point_inequality() {
    let a = Point::new(1.0, 2.0);
    let b = Point::new(1.0, 3.0);
    assert_ne!(a, b);
}

#[test]
fn point_debug_format() {
    let p = Point::new(1.0, 2.0);
    let s = format!("{p:?}");
    assert!(s.contains("Point"));
}

// --- Camera defaults ---

#[test]
fn camera_default_pan_is_zero() {
    let cam = Camera::default();
    assert_eq!(cam.pan_x, 0.0);
    assert_eq!(cam.pan_y, 0.0);
}

#[test]
fn camera_default_zoom_is_one() {
    let cam = Camera::default();
    assert_eq!(cam.zoom, 1.0);
}

// --- screen_to_world ---

#[test]
fn screen_to_world_identity() {
    let cam = Camera::default();
    let world = cam.screen_to_world(Point::new(50.0, 75.0));
    assert!(point_approx_eq(world, Point::new(50.0, 75.0)));
}

#[test]
fn screen_to_world_with_zoom() {
    let cam = Camera { pan_x: 0.0, pan_y: 0.0, zoom: 4.0 };
    let world = cam.screen_to_world(Point::new(40.0, 80.0));
    assert!(approx_eq(world.x, 10.0));
    assert!(approx_eq(world.y, 20.0));
}

#[test]
fn screen_to_world_with_pan() {
    let cam = Camera { pan_x: 100.0, pan_y: 50.0, zoom: 1.0 };
    let world = cam.screen_to_world(Point::new(100.0, 50.0));
    assert!(point_approx_eq(world, Point::new(0.0, 0.0)));
}

#[test]
fn screen_to_world_with_pan_and_zoom() {
    let cam = Camera { pan_x: 20.0, pan_y: 10.0, zoom: 2.0 };
    // screen (20, 10) -> world (0, 0) because (20-20)/2 = 0, (10-10)/2 = 0
    let world = cam.screen_to_world(Point::new(20.0, 10.0));
    assert!(point_approx_eq(world, Point::new(0.0, 0.0)));
}

#[test]
fn screen_to_world_negative_coords() {
    let cam = Camera { pan_x: 0.0, pan_y: 0.0, zoom: 1.0 };
    let world = cam.screen_to_world(Point::new(-10.0, -20.0));
    assert!(point_approx_eq(world, Point::new(-10.0, -20.0)));
}

#[test]
fn screen_to_world_origin() {
    let cam = Camera { pan_x: 50.0, pan_y: 30.0, zoom: 2.0 };
    let world = cam.screen_to_world(Point::new(0.0, 0.0));
    assert!(approx_eq(world.x, -25.0));
    assert!(approx_eq(world.y, -15.0));
}

// --- world_to_screen ---

#[test]
fn world_to_screen_identity() {
    let cam = Camera::default();
    let screen = cam.world_to_screen(Point::new(50.0, 75.0));
    assert!(point_approx_eq(screen, Point::new(50.0, 75.0)));
}

#[test]
fn world_to_screen_with_zoom() {
    let cam = Camera { pan_x: 0.0, pan_y: 0.0, zoom: 2.0 };
    let screen = cam.world_to_screen(Point::new(10.0, 20.0));
    assert!(approx_eq(screen.x, 20.0));
    assert!(approx_eq(screen.y, 40.0));
}

#[test]
fn world_to_screen_with_pan() {
    let cam = Camera { pan_x: 100.0, pan_y: 50.0, zoom: 1.0 };
    let screen = cam.world_to_screen(Point::new(0.0, 0.0));
    assert!(approx_eq(screen.x, 100.0));
    assert!(approx_eq(screen.y, 50.0));
}

#[test]
fn world_to_screen_with_pan_and_zoom() {
    let cam = Camera { pan_x: 20.0, pan_y: 10.0, zoom: 3.0 };
    let screen = cam.world_to_screen(Point::new(5.0, 5.0));
    // 5*3 + 20 = 35, 5*3 + 10 = 25
    assert!(approx_eq(screen.x, 35.0));
    assert!(approx_eq(screen.y, 25.0));
}

#[test]
fn world_to_screen_negative_world() {
    let cam = Camera { pan_x: 0.0, pan_y: 0.0, zoom: 1.0 };
    let screen = cam.world_to_screen(Point::new(-10.0, -20.0));
    assert!(point_approx_eq(screen, Point::new(-10.0, -20.0)));
}

// --- Round trips ---

#[test]
fn round_trip_identity() {
    let cam = Camera::default();
    let world = Point::new(100.0, 200.0);
    let screen = cam.world_to_screen(world);
    let back = cam.screen_to_world(screen);
    assert!(point_approx_eq(world, back));
}

#[test]
fn round_trip_with_pan_and_zoom() {
    let cam = Camera { pan_x: 50.0, pan_y: -30.0, zoom: 2.0 };
    let world = Point::new(100.0, 200.0);
    let screen = cam.world_to_screen(world);
    let back = cam.screen_to_world(screen);
    assert!(point_approx_eq(world, back));
}

#[test]
fn round_trip_fractional_zoom() {
    let cam = Camera { pan_x: 13.7, pan_y: -42.3, zoom: 0.75 };
    let world = Point::new(333.3, -999.9);
    let back = cam.screen_to_world(cam.world_to_screen(world));
    assert!(point_approx_eq(world, back));
}

#[test]
fn round_trip_screen_first() {
    let cam = Camera { pan_x: 10.0, pan_y: 20.0, zoom: 1.5 };
    let screen = Point::new(400.0, 300.0);
    let back = cam.world_to_screen(cam.screen_to_world(screen));
    assert!(point_approx_eq(screen, back));
}

// --- screen_dist_to_world ---

#[test]
fn screen_dist_to_world_identity_at_zoom_one() {
    let cam = Camera::default();
    assert!(approx_eq(cam.screen_dist_to_world(42.0), 42.0));
}

#[test]
fn screen_dist_to_world_with_zoom() {
    let cam = Camera { pan_x: 0.0, pan_y: 0.0, zoom: 2.0 };
    assert!(approx_eq(cam.screen_dist_to_world(10.0), 5.0));
}

#[test]
fn screen_dist_to_world_fractional_zoom() {
    let cam = Camera { pan_x: 0.0, pan_y: 0.0, zoom: 0.5 };
    assert!(approx_eq(cam.screen_dist_to_world(10.0), 20.0));
}

#[test]
fn screen_dist_to_world_zero() {
    let cam = Camera { pan_x: 0.0, pan_y: 0.0, zoom: 3.0 };
    assert!(approx_eq(cam.screen_dist_to_world(0.0), 0.0));
}

#[test]
fn screen_dist_to_world_ignores_pan() {
    let cam = Camera { pan_x: 999.0, pan_y: -999.0, zoom: 4.0 };
    assert!(approx_eq(cam.screen_dist_to_world(8.0), 2.0));
}
