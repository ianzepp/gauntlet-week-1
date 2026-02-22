use super::*;
use std::collections::HashMap;

fn obj(id: &str, kind: &str) -> BoardObject {
    BoardObject {
        id: id.to_owned(),
        board_id: "b1".to_owned(),
        kind: kind.to_owned(),
        x: 0.0,
        y: 0.0,
        width: Some(100.0),
        height: Some(80.0),
        rotation: 0.0,
        z_index: 0,
        props: serde_json::json!({}),
        created_by: None,
        version: 1,
        group_id: None,
    }
}

#[test]
fn extract_clip_parses_duration_and_events() {
    let mut host = obj("host", "frame");
    host.props = serde_json::json!({
        "animation": {
            "durationMs": 1000,
            "loop": true,
            "events": [
                { "tMs": 200, "op": "delete", "targetId": "x" },
                { "tMs": 100, "op": "update", "targetId": "x", "patch": { "x": 20 } }
            ]
        }
    });
    let clip = extract_clip_from_object(&host).expect("clip should parse");
    assert_eq!(clip.duration_ms, 1000.0);
    assert!(clip.looped);
    assert_eq!(clip.events.len(), 2);
    assert!(clip.events[0].t_ms <= clip.events[1].t_ms);
}

#[test]
fn project_clip_scene_applies_create_update_delete() {
    let mut base = HashMap::new();
    base.insert("a".to_owned(), obj("a", "rectangle"));

    let create_obj = BoardObject {
        id: "b".to_owned(),
        board_id: "b1".to_owned(),
        kind: "rectangle".to_owned(),
        x: 10.0,
        y: 20.0,
        width: Some(40.0),
        height: Some(30.0),
        rotation: 0.0,
        z_index: 1,
        props: serde_json::json!({ "fill": "#000000" }),
        created_by: None,
        version: 1,
        group_id: None,
    };
    let clip = AnimationClip {
        duration_ms: 1000.0,
        looped: false,
        scope_object_ids: Some(vec!["b".to_owned()]),
        events: vec![
            AnimationEvent { t_ms: 100.0, op: AnimationOp::Create { object: create_obj } },
            AnimationEvent {
                t_ms: 200.0,
                op: AnimationOp::Update {
                    target_id: "b".to_owned(),
                    patch: serde_json::json!({ "x": 99.0, "props": { "fill": "#ffffff" } }),
                },
            },
            AnimationEvent { t_ms: 300.0, op: AnimationOp::Delete { target_id: "b".to_owned() } },
        ],
    };

    let at_250 = project_clip_scene(&base, Some("b1"), &clip, 250.0);
    assert!(at_250.contains_key("a"));
    let b = at_250.get("b").expect("created object should exist");
    assert_eq!(b.x, 99.0);
    assert_eq!(b.props["fill"], "#ffffff");

    let at_350 = project_clip_scene(&base, Some("b1"), &clip, 350.0);
    assert!(!at_350.contains_key("b"));
}

#[test]
fn resolve_active_clip_prefers_selected_clip() {
    let mut board = BoardState::default();
    let mut host = obj("clip-1", "frame");
    host.props = serde_json::json!({
        "animation": { "durationMs": 500, "events": [] }
    });
    board.objects.insert("clip-1".to_owned(), host);
    board.selection.insert("clip-1".to_owned());

    let ui = UiState::default();
    let (id, clip) = resolve_active_clip(&board, &ui).expect("selected clip should resolve");
    assert_eq!(id, "clip-1");
    assert_eq!(clip.duration_ms, 500.0);
}

#[test]
fn project_clip_scene_interpolates_motion_between_updates() {
    let mut base = HashMap::new();
    let mut seed = obj("ball1", "ellipse");
    seed.y = 100.0;
    base.insert("ball1".to_owned(), seed.clone());

    let mut create_obj = seed;
    create_obj.id = "ball1".to_owned();
    let clip = AnimationClip {
        duration_ms: 2000.0,
        looped: false,
        scope_object_ids: Some(vec!["ball1".to_owned()]),
        events: vec![
            AnimationEvent { t_ms: 0.0, op: AnimationOp::Create { object: create_obj } },
            AnimationEvent {
                t_ms: 1000.0,
                op: AnimationOp::Update { target_id: "ball1".to_owned(), patch: serde_json::json!({ "y": 300.0 }) },
            },
            AnimationEvent {
                t_ms: 2000.0,
                op: AnimationOp::Update { target_id: "ball1".to_owned(), patch: serde_json::json!({ "y": 100.0 }) },
            },
        ],
    };

    let at_500 = project_clip_scene(&base, Some("b1"), &clip, 500.0);
    let y_500 = at_500.get("ball1").expect("ball1 exists").y;
    assert!((y_500 - 200.0).abs() < 0.001);

    let at_1500 = project_clip_scene(&base, Some("b1"), &clip, 1500.0);
    let y_1500 = at_1500.get("ball1").expect("ball1 exists").y;
    assert!((y_1500 - 200.0).abs() < 0.001);
}
