//! Layout engine: converts a parsed sequence diagram AST into board object descriptors.

use super::ast::{ArrowStyle, Block, BlockKind, Event, Note, NotePosition, SequenceDiagram};

// Layout constants (in logical pixels, before scale).
const PARTICIPANT_BOX_W: f64 = 120.0;
const PARTICIPANT_BOX_H: f64 = 40.0;
const PARTICIPANT_SPACING: f64 = 200.0;
const MESSAGE_ROW_HEIGHT: f64 = 50.0;
const ACTIVATION_BAR_W: f64 = 12.0;
const NOTE_W: f64 = 140.0;
const NOTE_H: f64 = 50.0;
const BLOCK_PADDING: f64 = 20.0;
const LIFELINE_DASH_PATTERN: &str = "8,4";

/// A descriptor for a board object to create.
#[derive(Debug, Clone)]
pub struct ObjectDescriptor {
    pub kind: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub props: serde_json::Value,
}

/// Convert a parsed sequence diagram into a list of board object descriptors.
///
/// Objects are positioned starting from `(origin_x, origin_y)` and scaled by `scale`.
#[must_use]
pub fn render_to_objects(diagram: &SequenceDiagram, origin_x: f64, origin_y: f64, scale: f64) -> Vec<ObjectDescriptor> {
    let mut objects = Vec::new();
    let participant_count = diagram.participants.len();
    if participant_count == 0 {
        return objects;
    }

    // Compute participant x-centers.
    let centers: Vec<f64> = (0..participant_count)
        .map(|i| {
            #[allow(clippy::cast_precision_loss)]
            let center = i as f64 * PARTICIPANT_SPACING;
            center
        })
        .collect();

    let total_w = if participant_count > 1 {
        centers[participant_count - 1] + PARTICIPANT_BOX_W
    } else {
        PARTICIPANT_BOX_W
    };

    // Count total event rows for lifeline height.
    let event_rows = count_event_rows(&diagram.events);
    #[allow(clippy::cast_precision_loss)]
    let content_height = event_rows as f64 * MESSAGE_ROW_HEIGHT;
    let lifeline_top = PARTICIPANT_BOX_H;
    let lifeline_bottom = lifeline_top + content_height + MESSAGE_ROW_HEIGHT;

    // --- Top participant boxes ---
    for (i, p) in diagram.participants.iter().enumerate() {
        let cx = centers[i];
        objects.push(make_rect(
            cx - PARTICIPANT_BOX_W / 2.0,
            0.0,
            PARTICIPANT_BOX_W,
            PARTICIPANT_BOX_H,
            &p.label,
            "#E3F2FD",
            "#1565C0",
        ));
    }

    // --- Lifelines (dashed vertical lines) ---
    for (i, _p) in diagram.participants.iter().enumerate() {
        let cx = centers[i];
        objects.push(make_lifeline(cx, lifeline_top, lifeline_bottom));
    }

    // --- Bottom participant boxes ---
    for (i, p) in diagram.participants.iter().enumerate() {
        let cx = centers[i];
        objects.push(make_rect(
            cx - PARTICIPANT_BOX_W / 2.0,
            lifeline_bottom,
            PARTICIPANT_BOX_W,
            PARTICIPANT_BOX_H,
            &p.label,
            "#E3F2FD",
            "#1565C0",
        ));
    }

    // --- Events ---
    let participant_index = |id: &str| -> Option<usize> { diagram.participants.iter().position(|p| p.id == id) };

    let mut row = 0;
    render_events(
        &diagram.events,
        &centers,
        &participant_index,
        lifeline_top,
        &mut row,
        total_w,
        &mut objects,
    );

    // Apply scale and origin offset.
    for obj in &mut objects {
        obj.x = origin_x + obj.x * scale;
        obj.y = origin_y + obj.y * scale;
        obj.width *= scale;
        obj.height *= scale;
    }

    objects
}

/// Recursively render events, advancing the row counter.
fn render_events(
    events: &[Event],
    centers: &[f64],
    participant_index: &dyn Fn(&str) -> Option<usize>,
    lifeline_top: f64,
    row: &mut usize,
    total_w: f64,
    objects: &mut Vec<ObjectDescriptor>,
) {
    for event in events {
        match event {
            Event::Message(msg) => {
                let from_idx = participant_index(&msg.from);
                let to_idx = participant_index(&msg.to);
                if let (Some(fi), Some(ti)) = (from_idx, to_idx) {
                    let from_x = centers[fi];
                    let to_x = centers[ti];
                    #[allow(clippy::cast_precision_loss)]
                    let y = lifeline_top + (*row as f64 + 0.5) * MESSAGE_ROW_HEIGHT;

                    // Arrow line.
                    let (is_dashed, _is_cross) = match msg.arrow {
                        ArrowStyle::Dashed | ArrowStyle::DashedOpen | ArrowStyle::DashedCross => (true, false),
                        ArrowStyle::SolidCross => (false, true),
                        _ => (false, false),
                    };
                    let is_open = matches!(msg.arrow, ArrowStyle::SolidOpen | ArrowStyle::DashedOpen);

                    let kind = if is_open || is_dashed { "line" } else { "arrow" };
                    objects.push(make_message_arrow(kind, from_x, to_x, y, is_dashed));

                    // Label above arrow.
                    if !msg.text.is_empty() {
                        let mid_x = f64::midpoint(from_x, to_x);
                        let label_w = f64::max(
                            #[allow(clippy::cast_precision_loss)]
                            {
                                msg.text.len() as f64 * 8.0
                            },
                            60.0,
                        );
                        objects.push(make_text(mid_x - label_w / 2.0, y - 20.0, label_w, 18.0, &msg.text));
                    }
                }
                *row += 1;
            }
            Event::Note(note) => {
                render_note(note, centers, participant_index, lifeline_top, *row, objects);
                *row += 1;
            }
            Event::Block(block) => {
                render_block(block, centers, participant_index, lifeline_top, row, total_w, objects);
            }
            Event::Activate(id) => {
                // Draw a thin activation rectangle starting at current row.
                if let Some(idx) = participant_index(id) {
                    let cx = centers[idx];
                    #[allow(clippy::cast_precision_loss)]
                    let y = lifeline_top + *row as f64 * MESSAGE_ROW_HEIGHT;
                    objects.push(make_activation(cx, y, MESSAGE_ROW_HEIGHT));
                }
            }
            Event::Deactivate(_) => {
                // Deactivation is implicit — no extra object needed.
            }
        }
    }
}

fn render_note(
    note: &Note,
    centers: &[f64],
    participant_index: &dyn Fn(&str) -> Option<usize>,
    lifeline_top: f64,
    row: usize,
    objects: &mut Vec<ObjectDescriptor>,
) {
    let first_idx = note.over.first().and_then(|id| participant_index(id));
    let Some(idx) = first_idx else {
        return;
    };
    let cx = centers[idx];

    #[allow(clippy::cast_precision_loss)]
    let y = lifeline_top + (row as f64 + 0.3) * MESSAGE_ROW_HEIGHT;

    let x = match note.position {
        NotePosition::LeftOf => cx - PARTICIPANT_BOX_W / 2.0 - NOTE_W - 10.0,
        NotePosition::RightOf => cx + PARTICIPANT_BOX_W / 2.0 + 10.0,
        NotePosition::Over => {
            if note.over.len() > 1 {
                let last_idx = note
                    .over
                    .last()
                    .and_then(|id| participant_index(id))
                    .unwrap_or(idx);
                let last_cx = centers[last_idx];
                f64::midpoint(cx, last_cx) - NOTE_W / 2.0
            } else {
                cx - NOTE_W / 2.0
            }
        }
    };

    objects.push(ObjectDescriptor {
        kind: "sticky_note".into(),
        x,
        y,
        width: NOTE_W,
        height: NOTE_H,
        props: serde_json::json!({
            "text": note.text,
            "fill": "#FFF9C4",
            "stroke": "#F9A825",
            "strokeWidth": 1,
            "fontSize": 14,
            "textColor": "#1F1A17"
        }),
    });
}

fn render_block(
    block: &Block,
    centers: &[f64],
    participant_index: &dyn Fn(&str) -> Option<usize>,
    lifeline_top: f64,
    row: &mut usize,
    total_w: f64,
    objects: &mut Vec<ObjectDescriptor>,
) {
    let start_row = *row;
    let kind_label = match block.kind {
        BlockKind::Loop => "loop",
        BlockKind::Alt => "alt",
        BlockKind::Opt => "opt",
        BlockKind::Par => "par",
        BlockKind::Critical => "critical",
        BlockKind::Break => "break",
    };
    let title = if block.label.is_empty() {
        kind_label.to_owned()
    } else {
        format!("{kind_label} [{label}]", label = block.label)
    };

    // Render section events.
    for (i, section) in block.sections.iter().enumerate() {
        if i > 0 {
            // Dashed separator line for else/and sections.
            #[allow(clippy::cast_precision_loss)]
            let sep_y = lifeline_top + *row as f64 * MESSAGE_ROW_HEIGHT;
            let sep_label = section.label.as_deref().unwrap_or("");
            objects.push(make_block_separator(
                -BLOCK_PADDING,
                sep_y,
                total_w + 2.0 * BLOCK_PADDING,
                sep_label,
            ));
        }
        render_events(&section.events, centers, participant_index, lifeline_top, row, total_w, objects);
        // Ensure at least one row per section.
        if section.events.is_empty() {
            *row += 1;
        }
    }

    // Create frame around the block.
    #[allow(clippy::cast_precision_loss)]
    let frame_y = lifeline_top + start_row as f64 * MESSAGE_ROW_HEIGHT - 5.0;
    #[allow(clippy::cast_precision_loss)]
    let frame_h = (*row - start_row) as f64 * MESSAGE_ROW_HEIGHT + 10.0;
    objects.push(ObjectDescriptor {
        kind: "frame".into(),
        x: -BLOCK_PADDING,
        y: frame_y,
        width: total_w + 2.0 * BLOCK_PADDING,
        height: frame_h,
        props: serde_json::json!({
            "title": title,
            "stroke": "#78909C",
            "strokeWidth": 1
        }),
    });
}

// ---- helpers ----

fn make_rect(x: f64, y: f64, w: f64, h: f64, label: &str, fill: &str, stroke: &str) -> ObjectDescriptor {
    ObjectDescriptor {
        kind: "rectangle".into(),
        x,
        y,
        width: w,
        height: h,
        props: serde_json::json!({
            "text": label,
            "fill": fill,
            "stroke": stroke,
            "strokeWidth": 2,
            "fontSize": 14,
            "textColor": "#1F1A17"
        }),
    }
}

fn make_lifeline(cx: f64, top: f64, bottom: f64) -> ObjectDescriptor {
    ObjectDescriptor {
        kind: "line".into(),
        x: cx,
        y: top,
        width: 0.0,
        height: bottom - top,
        props: serde_json::json!({
            "a": { "x": cx, "y": top },
            "b": { "x": cx, "y": bottom },
            "stroke": "#90A4AE",
            "strokeWidth": 1,
            "dashPattern": LIFELINE_DASH_PATTERN
        }),
    }
}

fn make_message_arrow(kind: &str, from_x: f64, to_x: f64, y: f64, is_dashed: bool) -> ObjectDescriptor {
    let x = from_x.min(to_x);
    let w = (from_x - to_x).abs().max(1.0);
    let mut props = serde_json::json!({
        "a": { "x": from_x, "y": y },
        "b": { "x": to_x, "y": y },
        "stroke": "#1F1A17",
        "strokeWidth": 1.5
    });
    if is_dashed {
        props
            .as_object_mut()
            .map(|m| m.insert("dashPattern".into(), serde_json::json!(LIFELINE_DASH_PATTERN)));
    }
    ObjectDescriptor { kind: kind.into(), x, y: y - 1.0, width: w, height: 2.0, props }
}

fn make_text(x: f64, y: f64, w: f64, font_size: f64, text: &str) -> ObjectDescriptor {
    ObjectDescriptor {
        kind: "text".into(),
        x,
        y,
        width: w,
        height: font_size + 4.0,
        props: serde_json::json!({
            "text": text,
            "fontSize": font_size,
            "textColor": "#1F1A17"
        }),
    }
}

fn make_activation(cx: f64, y: f64, height: f64) -> ObjectDescriptor {
    ObjectDescriptor {
        kind: "rectangle".into(),
        x: cx - ACTIVATION_BAR_W / 2.0,
        y,
        width: ACTIVATION_BAR_W,
        height,
        props: serde_json::json!({
            "fill": "#BBDEFB",
            "stroke": "#1565C0",
            "strokeWidth": 1
        }),
    }
}

fn make_block_separator(x: f64, y: f64, w: f64, label: &str) -> ObjectDescriptor {
    let mut props = serde_json::json!({
        "a": { "x": x, "y": y },
        "b": { "x": x + w, "y": y },
        "stroke": "#78909C",
        "strokeWidth": 1,
        "dashPattern": LIFELINE_DASH_PATTERN
    });
    if !label.is_empty() {
        props
            .as_object_mut()
            .map(|m| m.insert("text".into(), serde_json::json!(format!("[{label}]"))));
    }
    ObjectDescriptor { kind: "line".into(), x, y, width: w, height: 0.0, props }
}

/// Count the number of "rows" consumed by a list of events (for lifeline sizing).
fn count_event_rows(events: &[Event]) -> usize {
    let mut rows = 0;
    for event in events {
        match event {
            Event::Message(_) | Event::Note(_) => rows += 1,
            Event::Block(block) => {
                for section in &block.sections {
                    let section_rows = count_event_rows(&section.events);
                    rows += if section_rows == 0 { 1 } else { section_rows };
                }
            }
            Event::Activate(_) | Event::Deactivate(_) => {}
        }
    }
    rows
}
