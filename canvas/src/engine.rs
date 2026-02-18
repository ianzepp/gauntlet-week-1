use web_sys::HtmlCanvasElement;

use crate::camera::{Camera, Point};
use crate::doc::{BoardObject, DocStore, ObjectId, PartialBoardObject};
use crate::input::{Button, InputState, Key, Modifiers, Tool, UiState, WheelDelta};

#[cfg(test)]
#[path = "engine_test.rs"]
mod engine_test;

/// Actions returned from input handlers for the host to process.
#[derive(Debug, Clone)]
pub enum Action {
    None,
    ObjectCreated(BoardObject),
    ObjectUpdated { id: ObjectId, fields: PartialBoardObject },
    ObjectDeleted { id: ObjectId },
    EditTextRequested { id: ObjectId, head: String, text: String, foot: String },
    SetCursor(String),
    RenderNeeded,
}

/// Core engine state — all logic that doesn't depend on the canvas element.
///
/// Separated from `Engine` so it can be tested without WASM/browser dependencies.
pub struct EngineCore {
    pub doc: DocStore,
    pub camera: Camera,
    pub ui: UiState,
    pub input: InputState,
    pub viewport_width: f64,
    pub viewport_height: f64,
    pub dpr: f64,
}

impl Default for EngineCore {
    fn default() -> Self {
        Self {
            doc: DocStore::new(),
            camera: Camera::default(),
            ui: UiState::default(),
            input: InputState::default(),
            viewport_width: 0.0,
            viewport_height: 0.0,
            dpr: 1.0,
        }
    }
}

impl EngineCore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    // --- Data inputs ---

    /// Hydrate the document from a server snapshot.
    pub fn load_snapshot(&mut self, objects: Vec<BoardObject>) {
        self.doc.load_snapshot(objects);
    }

    /// Apply a server broadcast: object created.
    pub fn apply_create(&mut self, object: BoardObject) {
        self.doc.insert(object);
    }

    /// Apply a server broadcast: object updated.
    pub fn apply_update(&mut self, id: &ObjectId, fields: &PartialBoardObject) {
        self.doc.apply_partial(id, fields);
    }

    /// Apply a server broadcast: object deleted.
    pub fn apply_delete(&mut self, id: &ObjectId) {
        self.doc.remove(id);
        if self.ui.selected_id.as_ref() == Some(id) {
            self.ui.selected_id = None;
        }
    }

    // --- Tool / text ---

    /// Set the active tool.
    pub fn set_tool(&mut self, tool: Tool) {
        self.ui.tool = tool;
    }

    /// Commit text from the host editor back into the object's props.
    pub fn set_text(&mut self, id: &ObjectId, head: String, text: String, foot: String) -> Action {
        let partial = PartialBoardObject {
            props: Some(serde_json::json!({
                "head": head,
                "text": text,
                "foot": foot,
            })),
            ..Default::default()
        };
        self.doc.apply_partial(id, &partial);
        Action::ObjectUpdated { id: *id, fields: partial }
    }

    // --- Queries ---

    /// The currently selected object, if any.
    #[must_use]
    pub fn selection(&self) -> Option<ObjectId> {
        self.ui.selected_id
    }

    /// The current camera state.
    #[must_use]
    pub fn camera(&self) -> Camera {
        self.camera
    }

    /// Look up an object by ID.
    #[must_use]
    pub fn object(&self, id: &ObjectId) -> Option<&BoardObject> {
        self.doc.get(id)
    }
}

/// The full canvas engine. Wraps `EngineCore` and owns the browser canvas element.
pub struct Engine {
    #[allow(dead_code)]
    canvas: HtmlCanvasElement,
    pub core: EngineCore,
}

impl Engine {
    /// Create a new engine bound to the given canvas element.
    #[must_use]
    pub fn new(canvas: HtmlCanvasElement) -> Self {
        Self { canvas, core: EngineCore::new() }
    }

    // --- Delegated data inputs ---

    pub fn load_snapshot(&mut self, objects: Vec<BoardObject>) {
        self.core.load_snapshot(objects);
    }

    pub fn apply_create(&mut self, object: BoardObject) {
        self.core.apply_create(object);
    }

    pub fn apply_update(&mut self, id: &ObjectId, fields: &PartialBoardObject) {
        self.core.apply_update(id, fields);
    }

    pub fn apply_delete(&mut self, id: &ObjectId) {
        self.core.apply_delete(id);
    }

    pub fn set_tool(&mut self, tool: Tool) {
        self.core.set_tool(tool);
    }

    pub fn set_text(&mut self, id: &ObjectId, head: String, text: String, foot: String) -> Action {
        self.core.set_text(id, head, text, foot)
    }

    // --- Viewport ---

    /// Update viewport dimensions and device pixel ratio.
    pub fn set_viewport(&mut self, _width_css: f64, _height_css: f64, _dpr: f64) {
        todo!()
    }

    // --- Input events ---

    pub fn on_pointer_down(&mut self, _screen_pt: Point, _button: Button, _modifiers: Modifiers) -> Vec<Action> {
        todo!()
    }

    pub fn on_pointer_move(&mut self, _screen_pt: Point, _modifiers: Modifiers) -> Vec<Action> {
        todo!()
    }

    pub fn on_pointer_up(&mut self, _screen_pt: Point, _button: Button, _modifiers: Modifiers) -> Vec<Action> {
        todo!()
    }

    pub fn on_wheel(&mut self, _screen_pt: Point, _delta: WheelDelta, _modifiers: Modifiers) -> Vec<Action> {
        todo!()
    }

    pub fn on_key_down(&mut self, _key: Key, _modifiers: Modifiers) -> Vec<Action> {
        todo!()
    }

    pub fn on_key_up(&mut self, _key: Key, _modifiers: Modifiers) -> Vec<Action> {
        todo!()
    }

    // --- Render ---

    /// Draw the current state to the canvas.
    pub fn render(&self) {
        todo!()
    }

    // --- Delegated queries ---

    #[must_use]
    pub fn selection(&self) -> Option<ObjectId> {
        self.core.selection()
    }

    #[must_use]
    pub fn camera(&self) -> Camera {
        self.core.camera()
    }

    #[must_use]
    pub fn object(&self, id: &ObjectId) -> Option<&BoardObject> {
        self.core.object(id)
    }
}
