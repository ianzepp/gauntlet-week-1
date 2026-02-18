use super::*;

// =============================================================
// HitPart
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

// =============================================================
// ResizeAnchor
// =============================================================

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

// =============================================================
// EdgeEnd
// =============================================================

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

// =============================================================
// Hit
// =============================================================

#[test]
fn hit_stores_object_id_and_part() {
    let id = uuid::Uuid::new_v4();
    let hit = Hit { object_id: id, part: HitPart::Body };
    assert_eq!(hit.object_id, id);
    assert_eq!(hit.part, HitPart::Body);
}

#[test]
fn hit_with_resize_handle() {
    let id = uuid::Uuid::new_v4();
    let hit = Hit { object_id: id, part: HitPart::ResizeHandle(ResizeAnchor::Nw) };
    assert_eq!(hit.part, HitPart::ResizeHandle(ResizeAnchor::Nw));
}

#[test]
fn hit_with_edge_endpoint() {
    let id = uuid::Uuid::new_v4();
    let hit = Hit { object_id: id, part: HitPart::EdgeEndpoint(EdgeEnd::B) };
    assert_eq!(hit.part, HitPart::EdgeEndpoint(EdgeEnd::B));
}

#[test]
fn hit_debug_format() {
    let id = uuid::Uuid::nil();
    let hit = Hit { object_id: id, part: HitPart::RotateHandle };
    let s = format!("{hit:?}");
    assert!(s.contains("RotateHandle"));
}

#[test]
fn hit_clone() {
    let id = uuid::Uuid::new_v4();
    let hit = Hit { object_id: id, part: HitPart::EdgeBody };
    let hit2 = hit;
    assert_eq!(hit2.object_id, id);
    assert_eq!(hit2.part, HitPart::EdgeBody);
}
