//! Input model: tools, modifier keys, mouse buttons, and the gesture state machine.
//!
//! This module defines the types consumed by the input engine. `Tool` and
//! `Modifiers` capture the user's intent at the time of a pointer event.
//! `InputState` is the active gesture being tracked between pointer-down and
//! pointer-up, carrying all context needed to compute incremental deltas and
//! emit final document mutations on release.

#[cfg(test)]
#[path = "input_test.rs"]
mod input_test;

use crate::camera::Point;
use crate::doc::ObjectId;
use crate::hit::{EdgeEnd, ResizeAnchor};

/// Which tool is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tool {
    /// Pointer / selection tool (default).
    #[default]
    Select,
    /// Draw a rectangle.
    Rect,
    /// Create a text object.
    Text,
    /// Draw an ellipse.
    Ellipse,
    /// Draw a diamond.
    Diamond,
    /// Draw a five-point star.
    Star,
    /// Draw a straight line segment.
    Line,
    /// Draw a directed arrow.
    Arrow,
}

impl Tool {
    /// Whether this tool creates a node shape (rect, ellipse, diamond, star).
    #[must_use]
    pub fn is_shape(self) -> bool {
        matches!(self, Self::Rect | Self::Text | Self::Ellipse | Self::Diamond | Self::Star)
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
    /// Shift key is held.
    pub shift: bool,
    /// Ctrl key is held.
    pub ctrl: bool,
    /// Alt / Option key is held.
    pub alt: bool,
    /// Meta / Command key is held.
    pub meta: bool,
}

/// Mouse button identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    /// Left mouse button (or single-finger tap).
    Primary,
    /// Middle mouse button (scroll wheel click).
    Middle,
    /// Right mouse button (or two-finger tap).
    Secondary,
}

/// A keyboard key (simplified for v0).
///
/// The inner string holds the key name as reported by the browser (e.g. `"Delete"`, `"Escape"`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key(pub String);

/// Wheel / trackpad scroll delta.
#[derive(Debug, Clone, Copy)]
pub struct WheelDelta {
    /// Horizontal scroll amount in pixels.
    pub dx: f64,
    /// Vertical scroll amount in pixels (positive = down).
    pub dy: f64,
}

/// Persistent UI state visible to the renderer.
#[derive(Debug, Clone, Default)]
pub struct UiState {
    /// Currently active drawing tool.
    pub tool: Tool,
    /// The id of the currently selected object, if any.
    pub selected_id: Option<ObjectId>,
}

/// Internal state for the input state machine.
///
/// Each active variant carries gesture context needed to compute deltas and
/// emit final actions on pointer-up.
#[derive(Debug, Clone)]
pub enum InputState {
    /// No gesture in progress; waiting for the next pointer-down.
    Idle,
    /// The user is panning the canvas by dragging with no object selected.
    Panning {
        /// Screen-space position of the previous pointer event, used to compute pan delta.
        last_screen: Point,
    },
    /// The user is moving an existing object across the canvas.
    DraggingObject {
        /// Id of the object being dragged.
        id: ObjectId,
        /// World-space position of the pointer at the previous event.
        last_world: Point,
        /// Object x at the start of the drag, used to snap or revert.
        orig_x: f64,
        /// Object y at the start of the drag, used to snap or revert.
        orig_y: f64,
    },
    /// The user is drawing a new shape by dragging from an anchor corner.
    DrawingShape {
        /// Id of the newly created (provisional) object being sized.
        id: ObjectId,
        /// The world-space corner where the drag started; used to derive the bounding box.
        anchor_world: Point,
    },
    /// The user is resizing an object by dragging one of its eight handles.
    ResizingObject {
        /// Id of the object being resized.
        id: ObjectId,
        /// Which corner/edge handle is being dragged.
        anchor: ResizeAnchor,
        /// World-space pointer position at the start of the resize.
        start_world: Point,
        /// Object x at the start of the resize.
        orig_x: f64,
        /// Object y at the start of the resize.
        orig_y: f64,
        /// Object width at the start of the resize.
        orig_w: f64,
        /// Object height at the start of the resize.
        orig_h: f64,
    },
    /// The user is rotating an object by dragging the rotate handle.
    RotatingObject {
        /// Id of the object being rotated.
        id: ObjectId,
        /// World-space center of the object; the rotation pivot.
        center: Point,
        /// Rotation in degrees at the start of the gesture, used to compute delta.
        orig_rotation: f64,
    },
    /// The user is repositioning one endpoint of an edge object.
    DraggingEdgeEndpoint {
        /// Id of the edge object being edited.
        id: ObjectId,
        /// Which endpoint (A or B) is being dragged.
        end: EdgeEnd,
    },
}

impl Default for InputState {
    fn default() -> Self {
        Self::Idle
    }
}
