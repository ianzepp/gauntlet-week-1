#[cfg(test)]
#[path = "input_test.rs"]
mod input_test;

use crate::doc::ObjectId;

/// Which tool is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tool {
    #[default]
    Select,
    Rect,
    Ellipse,
    Diamond,
    Star,
    Line,
    Arrow,
}

/// Keyboard/mouse modifier keys held during an event.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

/// Mouse button identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Primary,
    Middle,
    Secondary,
}

/// A keyboard key (simplified for v0).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key(pub String);

/// Wheel / trackpad scroll delta.
#[derive(Debug, Clone, Copy)]
pub struct WheelDelta {
    pub dx: f64,
    pub dy: f64,
}

/// Persistent UI state visible to the renderer.
#[derive(Debug, Clone, Default)]
pub struct UiState {
    pub tool: Tool,
    pub selected_id: Option<ObjectId>,
}

/// Internal state for the input state machine.
#[derive(Debug, Clone)]
pub enum InputState {
    Idle,
    Panning,
    DraggingObject,
    DrawingShape,
    DraggingEdgeEndpoint,
}

impl Default for InputState {
    fn default() -> Self {
        Self::Idle
    }
}
