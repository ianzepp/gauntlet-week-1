//! Tests for the Mermaid sequence diagram parser and layout engine.

use super::ast::{ArrowStyle, BlockKind, Event, NotePosition};
use super::{parse, render_to_objects};

// =============================================================================
// PARSER TESTS
// =============================================================================

#[test]
fn parse_simple_two_participants() {
    let input = r"
        sequenceDiagram
        participant Alice
        participant Bob
        Alice->>Bob: Hello
    ";
    let diagram = parse(input).unwrap();
    assert_eq!(diagram.participants.len(), 2);
    assert_eq!(diagram.participants[0].id, "Alice");
    assert_eq!(diagram.participants[1].id, "Bob");
    assert_eq!(diagram.events.len(), 1);
    if let Event::Message(msg) = &diagram.events[0] {
        assert_eq!(msg.from, "Alice");
        assert_eq!(msg.to, "Bob");
        assert_eq!(msg.text, "Hello");
        assert_eq!(msg.arrow, ArrowStyle::Solid);
    } else {
        panic!("expected Message event");
    }
}

#[test]
fn parse_auto_participant_creation() {
    let input = r"
        sequenceDiagram
        Alice->>Bob: Hi
        Bob->>Charlie: Forward
    ";
    let diagram = parse(input).unwrap();
    assert_eq!(diagram.participants.len(), 3);
    assert_eq!(diagram.participants[0].id, "Alice");
    assert_eq!(diagram.participants[1].id, "Bob");
    assert_eq!(diagram.participants[2].id, "Charlie");
}

#[test]
fn parse_all_arrow_styles() {
    let input = r"
        sequenceDiagram
        A->>B: solid
        A->>B: solid again
        A->B: solid open
        A-->>B: dashed
        A-->B: dashed open
        A-xB: solid cross
        A--xB: dashed cross
    ";
    let diagram = parse(input).unwrap();
    let arrows: Vec<ArrowStyle> = diagram
        .events
        .iter()
        .filter_map(|e| match e {
            Event::Message(m) => Some(m.arrow),
            _ => None,
        })
        .collect();
    assert_eq!(arrows.len(), 7);
    assert_eq!(arrows[0], ArrowStyle::Solid);
    assert_eq!(arrows[1], ArrowStyle::Solid);
    assert_eq!(arrows[2], ArrowStyle::SolidOpen);
    assert_eq!(arrows[3], ArrowStyle::Dashed);
    assert_eq!(arrows[4], ArrowStyle::DashedOpen);
    assert_eq!(arrows[5], ArrowStyle::SolidCross);
    assert_eq!(arrows[6], ArrowStyle::DashedCross);
}

#[test]
fn parse_notes_over() {
    let input = r"
        sequenceDiagram
        participant Alice
        participant Bob
        Note over Alice,Bob: Both participants
    ";
    let diagram = parse(input).unwrap();
    assert_eq!(diagram.events.len(), 1);
    if let Event::Note(note) = &diagram.events[0] {
        assert_eq!(note.position, NotePosition::Over);
        assert_eq!(note.over.len(), 2);
        assert_eq!(note.text, "Both participants");
    } else {
        panic!("expected Note event");
    }
}

#[test]
fn parse_notes_left_right() {
    let input = r"
        sequenceDiagram
        participant Alice
        Note left of Alice: Left note
        Note right of Alice: Right note
    ";
    let diagram = parse(input).unwrap();
    assert_eq!(diagram.events.len(), 2);
    if let Event::Note(note) = &diagram.events[0] {
        assert_eq!(note.position, NotePosition::LeftOf);
        assert_eq!(note.text, "Left note");
    } else {
        panic!("expected Note event");
    }
    if let Event::Note(note) = &diagram.events[1] {
        assert_eq!(note.position, NotePosition::RightOf);
        assert_eq!(note.text, "Right note");
    } else {
        panic!("expected Note event");
    }
}

#[test]
fn parse_loop_block() {
    let input = r"
        sequenceDiagram
        Alice->>Bob: Request
        loop Every minute
            Bob->>Alice: Heartbeat
        end
    ";
    let diagram = parse(input).unwrap();
    assert_eq!(diagram.events.len(), 2);
    if let Event::Block(block) = &diagram.events[1] {
        assert_eq!(block.kind, BlockKind::Loop);
        assert_eq!(block.label, "Every minute");
        assert_eq!(block.sections.len(), 1);
        assert_eq!(block.sections[0].events.len(), 1);
    } else {
        panic!("expected Block event");
    }
}

#[test]
fn parse_alt_with_else() {
    let input = r"
        sequenceDiagram
        Alice->>Bob: Request
        alt Success
            Bob->>Alice: 200 OK
        else Failure
            Bob->>Alice: 500 Error
        end
    ";
    let diagram = parse(input).unwrap();
    if let Event::Block(block) = &diagram.events[1] {
        assert_eq!(block.kind, BlockKind::Alt);
        assert_eq!(block.label, "Success");
        assert_eq!(block.sections.len(), 2);
        assert!(block.sections[0].label.is_none());
        assert_eq!(block.sections[1].label.as_deref(), Some("Failure"));
    } else {
        panic!("expected Block event");
    }
}

#[test]
fn parse_par_with_and() {
    let input = r"
        sequenceDiagram
        par Task A
            Alice->>Bob: Do A
        and Task B
            Alice->>Charlie: Do B
        end
    ";
    let diagram = parse(input).unwrap();
    if let Event::Block(block) = &diagram.events[0] {
        assert_eq!(block.kind, BlockKind::Par);
        assert_eq!(block.sections.len(), 2);
        assert_eq!(block.sections[1].label.as_deref(), Some("Task B"));
    } else {
        panic!("expected Block event");
    }
}

#[test]
fn parse_activate_deactivate() {
    let input = r"
        sequenceDiagram
        Alice->>Bob: Request
        activate Bob
        Bob->>Alice: Response
        deactivate Bob
    ";
    let diagram = parse(input).unwrap();
    assert_eq!(diagram.events.len(), 4);
    assert!(matches!(&diagram.events[1], Event::Activate(id) if id == "Bob"));
    assert!(matches!(&diagram.events[3], Event::Deactivate(id) if id == "Bob"));
}

#[test]
fn parse_participant_with_alias() {
    let input = r"
        sequenceDiagram
        participant A as Alice the Great
        participant B as Bob
        A->>B: Hello
    ";
    let diagram = parse(input).unwrap();
    assert_eq!(diagram.participants[0].id, "A");
    assert_eq!(diagram.participants[0].label, "Alice the Great");
    assert_eq!(diagram.participants[1].id, "B");
    assert_eq!(diagram.participants[1].label, "Bob");
}

#[test]
fn parse_without_header() {
    let input = r"
        Alice->>Bob: Works without header
    ";
    let diagram = parse(input).unwrap();
    assert_eq!(diagram.participants.len(), 2);
    assert_eq!(diagram.events.len(), 1);
}

#[test]
fn parse_comments_skipped() {
    let input = r"
        sequenceDiagram
        %% This is a comment
        Alice->>Bob: Hello
    ";
    let diagram = parse(input).unwrap();
    assert_eq!(diagram.events.len(), 1);
}

#[test]
fn parse_empty_input() {
    let diagram = parse("").unwrap();
    assert!(diagram.participants.is_empty());
    assert!(diagram.events.is_empty());
}

#[test]
fn parse_nested_block() {
    let input = r"
        sequenceDiagram
        loop Outer
            Alice->>Bob: Ping
            alt Check
                Bob->>Alice: OK
            else Error
                Bob->>Alice: Fail
            end
        end
    ";
    let diagram = parse(input).unwrap();
    if let Event::Block(outer) = &diagram.events[0] {
        assert_eq!(outer.kind, BlockKind::Loop);
        assert_eq!(outer.sections[0].events.len(), 2);
        if let Event::Block(inner) = &outer.sections[0].events[1] {
            assert_eq!(inner.kind, BlockKind::Alt);
            assert_eq!(inner.sections.len(), 2);
        } else {
            panic!("expected inner Block");
        }
    } else {
        panic!("expected outer Block");
    }
}

// =============================================================================
// LAYOUT TESTS
// =============================================================================

#[test]
fn layout_two_participants_three_messages() {
    let input = r"
        sequenceDiagram
        Alice->>Bob: First
        Bob->>Alice: Second
        Alice->>Bob: Third
    ";
    let diagram = parse(input).unwrap();
    let objects = render_to_objects(&diagram, 0.0, 0.0, 1.0);

    // Expected objects:
    // 2 top participant boxes + 2 lifelines + 2 bottom participant boxes = 6
    // 3 message arrows + 3 message labels = 6
    // Total: 12
    assert_eq!(objects.len(), 12);

    // Verify participant boxes exist.
    let rects: Vec<_> = objects.iter().filter(|o| o.kind == "rectangle").collect();
    assert_eq!(rects.len(), 4); // 2 top + 2 bottom

    // Verify lifelines exist.
    let lines: Vec<_> = objects.iter().filter(|o| o.kind == "line").collect();
    assert_eq!(lines.len(), 2);

    // Verify arrows exist.
    let arrows: Vec<_> = objects.iter().filter(|o| o.kind == "arrow").collect();
    assert_eq!(arrows.len(), 3);

    // Verify text labels exist.
    let texts: Vec<_> = objects.iter().filter(|o| o.kind == "text").collect();
    assert_eq!(texts.len(), 3);
}

#[test]
fn layout_empty_diagram() {
    let input = "sequenceDiagram";
    let diagram = parse(input).unwrap();
    let objects = render_to_objects(&diagram, 0.0, 0.0, 1.0);
    assert!(objects.is_empty());
}

#[test]
fn layout_with_offset_and_scale() {
    let input = r"
        sequenceDiagram
        participant Alice
        participant Bob
        Alice->>Bob: Hello
    ";
    let diagram = parse(input).unwrap();
    let objects_default = render_to_objects(&diagram, 0.0, 0.0, 1.0);
    let objects_offset = render_to_objects(&diagram, 100.0, 200.0, 1.0);
    let objects_scaled = render_to_objects(&diagram, 0.0, 0.0, 2.0);

    assert_eq!(objects_default.len(), objects_offset.len());
    assert_eq!(objects_default.len(), objects_scaled.len());

    // Offset objects should be shifted.
    for (d, o) in objects_default.iter().zip(objects_offset.iter()) {
        let dx = (o.x - d.x - 100.0).abs();
        let dy = (o.y - d.y - 200.0).abs();
        assert!(dx < 0.01, "x offset mismatch: {dx}");
        assert!(dy < 0.01, "y offset mismatch: {dy}");
    }

    // Scaled objects should have doubled dimensions (skip zero-width objects).
    for (d, s) in objects_default.iter().zip(objects_scaled.iter()) {
        if d.width > 0.0 {
            let w_ratio = s.width / d.width;
            assert!((w_ratio - 2.0).abs() < 0.01, "width scale mismatch: {w_ratio}");
        }
    }
}

#[test]
fn layout_with_activation() {
    let input = r"
        sequenceDiagram
        Alice->>Bob: Request
        activate Bob
        Bob->>Alice: Response
        deactivate Bob
    ";
    let diagram = parse(input).unwrap();
    let objects = render_to_objects(&diagram, 0.0, 0.0, 1.0);

    // Check that activation rectangle exists (thin rectangle on Bob's lifeline).
    let rects: Vec<_> = objects.iter().filter(|o| o.kind == "rectangle").collect();
    // 4 participant boxes + 1 activation bar = 5
    assert_eq!(rects.len(), 5);
}

#[test]
fn layout_with_block_creates_frame() {
    let input = r"
        sequenceDiagram
        loop Every second
            Alice->>Bob: Ping
        end
    ";
    let diagram = parse(input).unwrap();
    let objects = render_to_objects(&diagram, 0.0, 0.0, 1.0);

    let frames: Vec<_> = objects.iter().filter(|o| o.kind == "frame").collect();
    assert_eq!(frames.len(), 1);
    let title = frames[0]
        .props
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(title.contains("loop"), "frame title should contain 'loop': {title}");
}

#[test]
fn layout_with_note() {
    let input = r"
        sequenceDiagram
        participant Alice
        Note right of Alice: Important
    ";
    let diagram = parse(input).unwrap();
    let objects = render_to_objects(&diagram, 0.0, 0.0, 1.0);

    let notes: Vec<_> = objects.iter().filter(|o| o.kind == "sticky_note").collect();
    assert_eq!(notes.len(), 1);
    let text = notes[0]
        .props
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert_eq!(text, "Important");
}

#[test]
fn end_to_end_full_diagram() {
    let input = r"
        sequenceDiagram
        participant Client
        participant Server
        participant DB
        Client->>Server: GET /api/data
        activate Server
        Server->>DB: SELECT * FROM data
        activate DB
        DB-->>Server: ResultSet
        deactivate DB
        Note right of Server: Process results
        Server-->>Client: 200 OK
        deactivate Server
    ";
    let diagram = parse(input).unwrap();
    assert_eq!(diagram.participants.len(), 3);
    assert_eq!(diagram.events.len(), 9); // 4 messages + 2 activate + 2 deactivate + 1 note

    let objects = render_to_objects(&diagram, 50.0, 100.0, 1.5);
    assert!(!objects.is_empty());

    // Verify all positions are offset.
    for obj in &objects {
        assert!(obj.x >= 50.0 - 200.0, "x should be near origin: {}", obj.x);
    }

    // Verify object types are present.
    let kinds: Vec<&str> = objects.iter().map(|o| o.kind.as_str()).collect();
    assert!(kinds.contains(&"rectangle"));
    assert!(kinds.contains(&"line"));
    assert!(kinds.contains(&"text"));
    assert!(kinds.contains(&"sticky_note"));
}
