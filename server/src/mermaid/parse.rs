//! Recursive descent parser for Mermaid sequence diagram syntax.

use super::ast::{
    ArrowStyle, Block, BlockKind, BlockSection, Event, Message, Note, NotePosition, Participant, SequenceDiagram,
};

/// Parse Mermaid sequence diagram text into an AST.
///
/// Accepts text with or without the `sequenceDiagram` header line.
///
/// # Errors
///
/// Returns a descriptive error string if parsing fails.
pub fn parse(input: &str) -> Result<SequenceDiagram, String> {
    let lines: Vec<&str> = input
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("%%"))
        .collect();

    let mut participants: Vec<Participant> = Vec::new();
    let mut participant_ids: Vec<String> = Vec::new();
    let mut pos = 0;

    // Skip optional header.
    if pos < lines.len() && lines[pos].eq_ignore_ascii_case("sequenceDiagram") {
        pos += 1;
    }

    let events = parse_events(&lines, &mut participants, &mut participant_ids, &mut pos, None)?;

    Ok(SequenceDiagram { participants, events })
}

/// Parse events until EOF or an `end` / section-separator keyword.
fn parse_events(
    lines: &[&str],
    participants: &mut Vec<Participant>,
    participant_ids: &mut Vec<String>,
    pos: &mut usize,
    stop_keywords: Option<&[&str]>,
) -> Result<Vec<Event>, String> {
    let mut events = Vec::new();

    while *pos < lines.len() {
        let line = lines[*pos];

        // Check stop keywords (for block parsing).
        if let Some(stops) = stop_keywords {
            let lower = line.to_ascii_lowercase();
            if stops
                .iter()
                .any(|kw| lower == *kw || lower.starts_with(&format!("{kw} ")))
            {
                break;
            }
        }

        // Skip the sequenceDiagram keyword if encountered again.
        if line.eq_ignore_ascii_case("sequenceDiagram") {
            *pos += 1;
            continue;
        }

        // Participant / actor declaration.
        if let Some(rest) = strip_keyword(line, "participant") {
            register_participant(rest, participants, participant_ids);
            *pos += 1;
            continue;
        }
        if let Some(rest) = strip_keyword(line, "actor") {
            register_participant(rest, participants, participant_ids);
            *pos += 1;
            continue;
        }

        // Activate / deactivate.
        if let Some(rest) = strip_keyword(line, "activate") {
            let id = rest.trim().to_owned();
            ensure_participant(&id, participants, participant_ids);
            events.push(Event::Activate(id));
            *pos += 1;
            continue;
        }
        if let Some(rest) = strip_keyword(line, "deactivate") {
            let id = rest.trim().to_owned();
            ensure_participant(&id, participants, participant_ids);
            events.push(Event::Deactivate(id));
            *pos += 1;
            continue;
        }

        // Note.
        if line.to_ascii_lowercase().starts_with("note ") {
            let note = parse_note(line, participants, participant_ids)?;
            events.push(Event::Note(note));
            *pos += 1;
            continue;
        }

        // Block keywords.
        if let Some(block_kind) = try_block_keyword(line) {
            let label = line
                .split_once(char::is_whitespace)
                .map_or("", |(_, rest)| rest)
                .trim()
                .to_owned();
            *pos += 1;
            let block = parse_block(lines, participants, participant_ids, pos, block_kind, &label)?;
            events.push(Event::Block(block));
            continue;
        }

        // Try message arrow.
        if let Some(msg) = try_parse_message(line, participants, participant_ids) {
            events.push(Event::Message(msg));
            *pos += 1;
            continue;
        }

        // Unknown line — skip.
        *pos += 1;
    }

    Ok(events)
}

/// Parse a block (loop, alt, opt, par, critical, break) including sections.
fn parse_block(
    lines: &[&str],
    participants: &mut Vec<Participant>,
    participant_ids: &mut Vec<String>,
    pos: &mut usize,
    kind: BlockKind,
    label: &str,
) -> Result<Block, String> {
    let section_sep = match kind {
        BlockKind::Alt => "else",
        BlockKind::Par => "and",
        BlockKind::Critical => "option",
        _ => "\x00_never_match",
    };

    let stop_keywords: Vec<&str> = vec!["end", section_sep];
    let mut sections = Vec::new();

    // Parse first section.
    let events = parse_events(lines, participants, participant_ids, pos, Some(&stop_keywords))?;
    sections.push(BlockSection { label: None, events });

    // Parse additional sections.
    while *pos < lines.len() {
        let line = lines[*pos];
        let lower = line.to_ascii_lowercase();

        if lower == "end" {
            *pos += 1;
            break;
        }

        if lower.starts_with(section_sep) {
            let section_label = line
                .get(section_sep.len()..)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned);
            *pos += 1;
            let events = parse_events(lines, participants, participant_ids, pos, Some(&stop_keywords))?;
            sections.push(BlockSection { label: section_label, events });
        } else {
            // Unexpected line; break to avoid infinite loop.
            break;
        }
    }

    Ok(Block { kind, label: label.to_owned(), sections })
}

/// Try to parse a message from a line containing an arrow pattern.
fn try_parse_message(
    line: &str,
    participants: &mut Vec<Participant>,
    participant_ids: &mut Vec<String>,
) -> Option<Message> {
    // Arrow patterns ordered longest-first to avoid prefix conflicts.
    const ARROWS: &[(&str, ArrowStyle)] = &[
        ("-->>", ArrowStyle::Dashed),
        ("->>", ArrowStyle::Solid),
        ("-->", ArrowStyle::DashedOpen),
        ("--x", ArrowStyle::DashedCross),
        ("-x", ArrowStyle::SolidCross),
        ("->", ArrowStyle::SolidOpen),
    ];

    for &(pattern, arrow) in ARROWS {
        if let Some(idx) = line.find(pattern) {
            let from = line[..idx].trim();
            let rest = line[idx + pattern.len()..].trim();

            // The rest should be "To: text" — split on first ':'
            let (to, text) = if let Some((t, txt)) = rest.split_once(':') {
                (t.trim(), txt.trim())
            } else {
                (rest, "")
            };

            if from.is_empty() || to.is_empty() {
                continue;
            }

            ensure_participant(from, participants, participant_ids);
            ensure_participant(to, participants, participant_ids);

            return Some(Message { from: from.to_owned(), to: to.to_owned(), text: text.to_owned(), arrow });
        }
    }

    None
}

/// Parse a `Note` from a line like `Note over A,B: text` or `Note left of A: text`.
fn parse_note(
    line: &str,
    participants: &mut Vec<Participant>,
    participant_ids: &mut Vec<String>,
) -> Result<Note, String> {
    // Strip "Note " prefix (case-insensitive).
    let rest = &line[5..]; // "Note " is 5 chars

    let (position, after_pos) = if rest.to_ascii_lowercase().starts_with("left of ") {
        (NotePosition::LeftOf, &rest[8..])
    } else if rest.to_ascii_lowercase().starts_with("right of ") {
        (NotePosition::RightOf, &rest[9..])
    } else if rest.to_ascii_lowercase().starts_with("over ") {
        (NotePosition::Over, &rest[5..])
    } else {
        return Err(format!("invalid note syntax: {line}"));
    };

    let (participants_str, text) = after_pos
        .split_once(':')
        .ok_or_else(|| format!("note missing colon: {line}"))?;

    let over: Vec<String> = participants_str
        .split(',')
        .map(|s| {
            let id = s.trim().to_owned();
            ensure_participant(&id, participants, participant_ids);
            id
        })
        .collect();

    Ok(Note { over, text: text.trim().to_owned(), position })
}

/// Strip a keyword prefix (case-insensitive) and return the rest.
fn strip_keyword<'a>(line: &'a str, keyword: &str) -> Option<&'a str> {
    let lower = line.to_ascii_lowercase();
    if lower.starts_with(keyword) {
        let rest = &line[keyword.len()..];
        if rest.is_empty() || rest.starts_with(char::is_whitespace) {
            return Some(rest.trim());
        }
    }
    None
}

/// Check if a line starts with a block keyword and return its kind.
fn try_block_keyword(line: &str) -> Option<BlockKind> {
    let lower = line.to_ascii_lowercase();
    let word = lower.split_whitespace().next()?;
    match word {
        "loop" => Some(BlockKind::Loop),
        "alt" => Some(BlockKind::Alt),
        "opt" => Some(BlockKind::Opt),
        "par" => Some(BlockKind::Par),
        "critical" => Some(BlockKind::Critical),
        "break" => Some(BlockKind::Break),
        _ => None,
    }
}

/// Register a participant from a declaration like `Alice` or `Alice as The Alice`.
fn register_participant(rest: &str, participants: &mut Vec<Participant>, participant_ids: &mut Vec<String>) {
    let (id, label) = if let Some((name, alias)) = rest.split_once(" as ") {
        (name.trim().to_owned(), alias.trim().to_owned())
    } else {
        let id = rest.trim().to_owned();
        let label = id.clone();
        (id, label)
    };

    if !id.is_empty() && !participant_ids.contains(&id) {
        participant_ids.push(id.clone());
        participants.push(Participant { id, label });
    }
}

/// Ensure a participant exists by id, creating it if necessary.
fn ensure_participant(id: &str, participants: &mut Vec<Participant>, participant_ids: &mut Vec<String>) {
    if !participant_ids.contains(&id.to_owned()) {
        participant_ids.push(id.to_owned());
        participants.push(Participant { id: id.to_owned(), label: id.to_owned() });
    }
}
