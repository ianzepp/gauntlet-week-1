#[cfg(test)]
#[path = "input_test.rs"]
mod input_test;

use crate::camera::Point;
use crate::doc::ObjectId;
use crate::hit::{EdgeEnd, ResizeAnchor};

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

impl Tool {
    /// Whether this tool creates a node shape (rect, ellipse, diamond, star).
    #[must_use]
    pub fn is_shape(self) -> bool {
        matches!(self, Self::Rect | Self::Ellipse | Self::Diamond | Self::Star)
    }

    /// Whether this tool creates an edge (line, arrow).
    #[must_use]
    pub fn is_edge(self) -> bool {
        matches!(self, Self::Line | Self::Arrow)
    }
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
///
/// Each active variant carries gesture context needed to compute deltas and
/// emit final actions on pointer-up.
#[derive(Debug, Clone)]
pub enum InputState {
    Idle,
    Panning {
        last_screen: Point,
    },
    DraggingObject {
        id: ObjectId,
        last_world: Point,
        orig_x: f64,
        orig_y: f64,
    },
    DrawingShape {
        id: ObjectId,
        anchor_world: Point,
    },
    ResizingObject {
        id: ObjectId,
        anchor: ResizeAnchor,
        start_world: Point,
        orig_x: f64,
        orig_y: f64,
        orig_w: f64,
        orig_h: f64,
    },
    RotatingObject {
        id: ObjectId,
        center: Point,
        orig_rotation: f64,
    },
    DraggingEdgeEndpoint {
        id: ObjectId,
        end: EdgeEnd,
    },
}

impl Default for InputState {
    fn default() -> Self {
        Self::Idle
    }
}
