//! AST types for Mermaid sequence diagrams.

/// A parsed Mermaid sequence diagram.
#[derive(Debug, Clone)]
pub struct SequenceDiagram {
    pub participants: Vec<Participant>,
    pub events: Vec<Event>,
}

/// A named participant (actor) in the sequence diagram.
#[derive(Debug, Clone)]
pub struct Participant {
    pub id: String,
    pub label: String,
}

/// A top-level event in the diagram.
#[derive(Debug, Clone)]
pub enum Event {
    Message(Message),
    Note(Note),
    Block(Block),
    Activate(String),
    Deactivate(String),
}

/// A message arrow between two participants.
#[derive(Debug, Clone)]
pub struct Message {
    pub from: String,
    pub to: String,
    pub text: String,
    pub arrow: ArrowStyle,
}

/// Arrow style for messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowStyle {
    /// `->>` solid line with filled arrowhead
    Solid,
    /// `->` solid line with open arrowhead
    SolidOpen,
    /// `-->>` dashed line with filled arrowhead
    Dashed,
    /// `-->` dashed line with open arrowhead
    DashedOpen,
    /// `-x` solid line with cross
    SolidCross,
    /// `--x` dashed line with cross
    DashedCross,
}

/// A note attached to one or more participants.
#[derive(Debug, Clone)]
pub struct Note {
    pub over: Vec<String>,
    pub text: String,
    pub position: NotePosition,
}

/// Where a note is positioned relative to participants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotePosition {
    Over,
    LeftOf,
    RightOf,
}

/// A control flow block (loop, alt, opt, par, critical, break).
#[derive(Debug, Clone)]
pub struct Block {
    pub kind: BlockKind,
    pub label: String,
    pub sections: Vec<BlockSection>,
}

/// A section within a block, separated by `else` or `and`.
#[derive(Debug, Clone)]
pub struct BlockSection {
    pub label: Option<String>,
    pub events: Vec<Event>,
}

/// The kind of control flow block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Loop,
    Alt,
    Opt,
    Par,
    Critical,
    Break,
}
