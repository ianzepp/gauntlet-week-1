use web_sys::HtmlCanvasElement;

use crate::camera::{Camera, Point};
use crate::consts::{MIN_SHAPE_SIZE, ZOOM_FACTOR, ZOOM_MAX, ZOOM_MIN};
use crate::doc::{BoardObject, DocStore, ObjectId, ObjectKind, PartialBoardObject, Props};
use crate::hit::{self, EdgeEnd, HitPart, ResizeAnchor};
use crate::input::{Button, InputState, Key, Modifiers, Tool, UiState, WheelDelta};

#[cfg(test)]
#[path = "engine_test.rs"]
mod engine_test;

/// Actions returned from input handlers for the host to process.
#[derive(Debug, Clone)]
pub enum Action {
    None,
    ObjectCreated(BoardObject),
    ObjectUpdated {
        id: ObjectId,
        fields: PartialBoardObject,
    },
    ObjectDeleted {
        id: ObjectId,
    },
    EditTextRequested {
        id: ObjectId,
        head: String,
        text: String,
        foot: String,
    },
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
        let Some(obj) = self.doc.get(id) else {
            return Action::None;
        };

        let existing = Props::new(&obj.props);
        if existing.head() == head && existing.text() == text && existing.foot() == foot {
            return Action::None;
        }

        let partial = PartialBoardObject {
            props: Some(serde_json::json!({
                "head": head,
                "text": text,
                "foot": foot,
            })),
            ..Default::default()
        };
        if self.doc.apply_partial(id, &partial) {
            Action::ObjectUpdated { id: *id, fields: partial }
        } else {
            Action::None
        }
    }

    // --- Viewport ---

    /// Update viewport dimensions and device pixel ratio.
    pub fn set_viewport(&mut self, width_css: f64, height_css: f64, dpr: f64) {
        self.viewport_width = width_css;
        self.viewport_height = height_css;
        self.dpr = dpr;
    }

    // --- Input events ---

    /// Handle a pointer-down event. Returns actions for the host.
    pub fn on_pointer_down(&mut self, screen_pt: Point, button: Button, _modifiers: Modifiers) -> Vec<Action> {
        let world_pt = self.camera.screen_to_world(screen_pt);
        let mut actions = Vec::new();

        // Middle button always pans.
        if button == Button::Middle {
            self.input = InputState::Panning { last_screen: screen_pt };
            actions.push(Action::SetCursor("grab".into()));
            return actions;
        }

        // Only handle primary button from here.
        if button != Button::Primary {
            return actions;
        }

        match self.ui.tool {
            Tool::Select => {
                self.handle_select_down(screen_pt, world_pt, &mut actions);
            }
            tool if tool.is_shape() => {
                self.handle_shape_tool_down(world_pt, tool, &mut actions);
            }
            tool if tool.is_edge() => {
                self.handle_edge_tool_down(world_pt, tool, &mut actions);
            }
            _ => {}
        }

        actions
    }

    /// Handle a pointer-move event. Returns actions for the host.
    pub fn on_pointer_move(&mut self, screen_pt: Point, _modifiers: Modifiers) -> Vec<Action> {
        let world_pt = self.camera.screen_to_world(screen_pt);

        match self.input.clone() {
            InputState::Idle => Vec::new(),
            InputState::Panning { last_screen } => {
                let dx = screen_pt.x - last_screen.x;
                let dy = screen_pt.y - last_screen.y;
                self.camera.pan_x += dx;
                self.camera.pan_y += dy;
                self.input = InputState::Panning { last_screen: screen_pt };
                vec![Action::RenderNeeded]
            }
            InputState::DraggingObject { id, last_world, orig_x, orig_y } => {
                let dx = world_pt.x - last_world.x;
                let dy = world_pt.y - last_world.y;
                if let Some(obj) = self.doc.get(&id) {
                    let new_x = obj.x + dx;
                    let new_y = obj.y + dy;
                    let partial = PartialBoardObject { x: Some(new_x), y: Some(new_y), ..Default::default() };
                    self.doc.apply_partial(&id, &partial);
                }
                self.input = InputState::DraggingObject { id, last_world: world_pt, orig_x, orig_y };
                vec![Action::RenderNeeded]
            }
            InputState::DrawingShape { id, anchor_world } => {
                self.handle_drawing_move(id, anchor_world, world_pt);
                vec![Action::RenderNeeded]
            }
            InputState::ResizingObject { id, anchor, start_world, orig_x, orig_y, orig_w, orig_h } => {
                let rotation = self.doc.get(&id).map_or(0.0, |obj| obj.rotation);
                let center = Point::new(orig_x + orig_w / 2.0, orig_y + orig_h / 2.0);
                let start_local = hit::rotate_point(start_world, center, -rotation);
                let current_local = hit::rotate_point(world_pt, center, -rotation);
                let dx = current_local.x - start_local.x;
                let dy = current_local.y - start_local.y;
                self.apply_resize(id, anchor, dx, dy, orig_x, orig_y, orig_w, orig_h);
                self.input = InputState::ResizingObject { id, anchor, start_world, orig_x, orig_y, orig_w, orig_h };
                vec![Action::RenderNeeded]
            }
            InputState::RotatingObject { id, center, orig_rotation: _ } => {
                let angle = (world_pt.y - center.y)
                    .atan2(world_pt.x - center.x)
                    .to_degrees()
                    + 90.0;
                let partial = PartialBoardObject { rotation: Some(angle), ..Default::default() };
                self.doc.apply_partial(&id, &partial);
                self.input = InputState::RotatingObject { id, center, orig_rotation: angle };
                vec![Action::RenderNeeded]
            }
            InputState::DraggingEdgeEndpoint { id, end } => {
                self.apply_edge_endpoint_move(&id, end, world_pt);
                vec![Action::RenderNeeded]
            }
        }
    }

    /// Handle a pointer-up event. Returns actions for the host.
    pub fn on_pointer_up(&mut self, _screen_pt: Point, _button: Button, _modifiers: Modifiers) -> Vec<Action> {
        let prev_state = std::mem::replace(&mut self.input, InputState::Idle);
        let mut actions = Vec::new();

        match prev_state {
            InputState::Idle => {}
            InputState::Panning { .. } => {
                actions.push(Action::RenderNeeded);
            }
            InputState::DraggingObject { id, orig_x, orig_y, .. } => {
                if let Some(obj) = self.doc.get(&id) {
                    let partial = PartialBoardObject { x: Some(obj.x), y: Some(obj.y), ..Default::default() };
                    // Only emit update if position actually changed.
                    if (obj.x - orig_x).abs() > f64::EPSILON || (obj.y - orig_y).abs() > f64::EPSILON {
                        actions.push(Action::ObjectUpdated { id, fields: partial });
                    }
                }
            }
            InputState::DrawingShape { id, .. } => {
                if let Some(obj) = self.doc.get(&id) {
                    let is_edge = matches!(obj.kind, ObjectKind::Line | ObjectKind::Arrow);
                    let too_small = !is_edge && obj.width.abs() < MIN_SHAPE_SIZE && obj.height.abs() < MIN_SHAPE_SIZE;

                    if too_small {
                        self.doc.remove(&id);
                    } else {
                        actions.push(Action::ObjectCreated(obj.clone()));
                    }
                }
                self.ui.tool = Tool::Select;
            }
            InputState::ResizingObject { id, .. } => {
                if let Some(obj) = self.doc.get(&id) {
                    let partial = PartialBoardObject {
                        x: Some(obj.x),
                        y: Some(obj.y),
                        width: Some(obj.width),
                        height: Some(obj.height),
                        ..Default::default()
                    };
                    actions.push(Action::ObjectUpdated { id, fields: partial });
                }
            }
            InputState::RotatingObject { id, .. } => {
                if let Some(obj) = self.doc.get(&id) {
                    let partial = PartialBoardObject { rotation: Some(obj.rotation), ..Default::default() };
                    actions.push(Action::ObjectUpdated { id, fields: partial });
                }
            }
            InputState::DraggingEdgeEndpoint { id, .. } => {
                if let Some(obj) = self.doc.get(&id) {
                    let partial = PartialBoardObject { props: Some(obj.props.clone()), ..Default::default() };
                    actions.push(Action::ObjectUpdated { id, fields: partial });
                }
            }
        }

        actions
    }

    /// Handle a wheel/scroll event. Returns actions for the host.
    pub fn on_wheel(&mut self, screen_pt: Point, delta: WheelDelta, modifiers: Modifiers) -> Vec<Action> {
        if modifiers.ctrl || modifiers.meta {
            // Zoom toward cursor.
            let factor = if delta.dy < 0.0 { ZOOM_FACTOR } else { 1.0 / ZOOM_FACTOR };
            let new_zoom = (self.camera.zoom * factor).clamp(ZOOM_MIN, ZOOM_MAX);
            let ratio = new_zoom / self.camera.zoom;

            // Adjust pan so the world point under the cursor stays fixed.
            self.camera.pan_x = screen_pt.x - ratio * (screen_pt.x - self.camera.pan_x);
            self.camera.pan_y = screen_pt.y - ratio * (screen_pt.y - self.camera.pan_y);
            self.camera.zoom = new_zoom;
        } else {
            // Pan.
            self.camera.pan_x -= delta.dx;
            self.camera.pan_y -= delta.dy;
        }
        vec![Action::RenderNeeded]
    }

    /// Handle a key-down event. Returns actions for the host.
    pub fn on_key_down(&mut self, key: Key, _modifiers: Modifiers) -> Vec<Action> {
        let mut actions = Vec::new();

        match key.0.as_str() {
            "Delete" | "Backspace" => {
                if let Some(id) = self.ui.selected_id.take() {
                    self.doc.remove(&id);
                    actions.push(Action::ObjectDeleted { id });
                    actions.push(Action::RenderNeeded);
                }
            }
            "Escape" => {
                // Cancel active gesture and deselect.
                self.input = InputState::Idle;
                if self.ui.selected_id.take().is_some() {
                    actions.push(Action::RenderNeeded);
                }
            }
            "Enter" => {
                if let Some(id) = self.ui.selected_id {
                    if let Some(obj) = self.doc.get(&id) {
                        let props = Props::new(&obj.props);
                        actions.push(Action::EditTextRequested {
                            id,
                            head: props.head().to_owned(),
                            text: props.text().to_owned(),
                            foot: props.foot().to_owned(),
                        });
                    }
                }
            }
            _ => {}
        }

        actions
    }

    /// Handle a key-up event. No-op for v0.
    pub fn on_key_up(&mut self, _key: Key, _modifiers: Modifiers) -> Vec<Action> {
        Vec::new()
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

    // =============================================================
    // Private helpers
    // =============================================================

    fn handle_select_down(&mut self, screen_pt: Point, world_pt: Point, actions: &mut Vec<Action>) {
        let hit = hit::hit_test(world_pt, &self.doc, &self.camera, self.ui.selected_id);

        if let Some(h) = hit {
            match h.part {
                HitPart::ResizeHandle(anchor) => {
                    if let Some(obj) = self.doc.get(&h.object_id) {
                        self.input = InputState::ResizingObject {
                            id: h.object_id,
                            anchor,
                            start_world: world_pt,
                            orig_x: obj.x,
                            orig_y: obj.y,
                            orig_w: obj.width,
                            orig_h: obj.height,
                        };
                    }
                }
                HitPart::RotateHandle => {
                    if let Some(obj) = self.doc.get(&h.object_id) {
                        let center = Point::new(obj.x + obj.width / 2.0, obj.y + obj.height / 2.0);
                        self.input =
                            InputState::RotatingObject { id: h.object_id, center, orig_rotation: obj.rotation };
                    }
                }
                HitPart::EdgeEndpoint(end) => {
                    self.ui.selected_id = Some(h.object_id);
                    self.input = InputState::DraggingEdgeEndpoint { id: h.object_id, end };
                    actions.push(Action::RenderNeeded);
                }
                HitPart::Body | HitPart::EdgeBody => {
                    self.ui.selected_id = Some(h.object_id);
                    if let Some(obj) = self.doc.get(&h.object_id) {
                        self.input = InputState::DraggingObject {
                            id: h.object_id,
                            last_world: world_pt,
                            orig_x: obj.x,
                            orig_y: obj.y,
                        };
                    }
                    actions.push(Action::RenderNeeded);
                }
            }
        } else {
            // Click on empty space: deselect.
            if self.ui.selected_id.take().is_some() {
                actions.push(Action::RenderNeeded);
            }
            // Also start panning on empty space drag.
            self.input = InputState::Panning { last_screen: screen_pt };
        }
    }

    fn handle_shape_tool_down(&mut self, world_pt: Point, tool: Tool, actions: &mut Vec<Action>) {
        let kind = match tool {
            Tool::Rect => ObjectKind::Rect,
            Tool::Ellipse => ObjectKind::Ellipse,
            Tool::Diamond => ObjectKind::Diamond,
            Tool::Star => ObjectKind::Star,
            _ => return,
        };
        let obj = self.create_default_object(kind, world_pt.x, world_pt.y, 0.0, 0.0);
        let id = obj.id;
        self.doc.insert(obj);
        self.ui.selected_id = Some(id);
        self.input = InputState::DrawingShape { id, anchor_world: world_pt };
        actions.push(Action::RenderNeeded);
    }

    fn handle_edge_tool_down(&mut self, world_pt: Point, tool: Tool, actions: &mut Vec<Action>) {
        let kind = match tool {
            Tool::Line => ObjectKind::Line,
            Tool::Arrow => ObjectKind::Arrow,
            _ => return,
        };
        let mut obj = self.create_default_object(kind, world_pt.x, world_pt.y, 0.0, 0.0);
        obj.props = serde_json::json!({
            "a": { "type": "free", "x": world_pt.x, "y": world_pt.y },
            "b": { "type": "free", "x": world_pt.x, "y": world_pt.y },
        });
        let id = obj.id;
        self.doc.insert(obj);
        self.ui.selected_id = Some(id);
        self.input = InputState::DrawingShape { id, anchor_world: world_pt };
        actions.push(Action::RenderNeeded);
    }

    fn handle_drawing_move(&mut self, id: ObjectId, anchor_world: Point, world_pt: Point) {
        if let Some(obj) = self.doc.get(&id) {
            let is_edge = matches!(obj.kind, ObjectKind::Line | ObjectKind::Arrow);
            if is_edge {
                // Update endpoint B.
                let partial = PartialBoardObject {
                    props: Some(serde_json::json!({
                        "b": { "type": "free", "x": world_pt.x, "y": world_pt.y },
                    })),
                    ..Default::default()
                };
                self.doc.apply_partial(&id, &partial);
            } else {
                // Update x/y/width/height from anchor and current pointer.
                let x = anchor_world.x.min(world_pt.x);
                let y = anchor_world.y.min(world_pt.y);
                let w = (world_pt.x - anchor_world.x).abs();
                let h = (world_pt.y - anchor_world.y).abs();
                let partial = PartialBoardObject {
                    x: Some(x),
                    y: Some(y),
                    width: Some(w),
                    height: Some(h),
                    ..Default::default()
                };
                self.doc.apply_partial(&id, &partial);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_resize(
        &mut self,
        id: ObjectId,
        anchor: ResizeAnchor,
        dx: f64,
        dy: f64,
        orig_x: f64,
        orig_y: f64,
        orig_w: f64,
        orig_h: f64,
    ) {
        let mut left = orig_x;
        let mut top = orig_y;
        let mut right = orig_x + orig_w;
        let mut bottom = orig_y + orig_h;

        match anchor {
            ResizeAnchor::N => {
                top += dy;
                top = top.min(bottom);
            }
            ResizeAnchor::S => {
                bottom += dy;
                bottom = bottom.max(top);
            }
            ResizeAnchor::E => {
                right += dx;
                right = right.max(left);
            }
            ResizeAnchor::W => {
                left += dx;
                left = left.min(right);
            }
            ResizeAnchor::Ne => {
                top += dy;
                top = top.min(bottom);
                right += dx;
                right = right.max(left);
            }
            ResizeAnchor::Nw => {
                top += dy;
                top = top.min(bottom);
                left += dx;
                left = left.min(right);
            }
            ResizeAnchor::Se => {
                bottom += dy;
                bottom = bottom.max(top);
                right += dx;
                right = right.max(left);
            }
            ResizeAnchor::Sw => {
                bottom += dy;
                bottom = bottom.max(top);
                left += dx;
                left = left.min(right);
            }
        }

        let x = left;
        let y = top;
        let w = right - left;
        let h = bottom - top;

        let partial =
            PartialBoardObject { x: Some(x), y: Some(y), width: Some(w), height: Some(h), ..Default::default() };
        self.doc.apply_partial(&id, &partial);
    }

    fn apply_edge_endpoint_move(&mut self, id: &ObjectId, end: EdgeEnd, world_pt: Point) {
        let key = match end {
            EdgeEnd::A => "a",
            EdgeEnd::B => "b",
        };
        let partial = PartialBoardObject {
            props: Some(serde_json::json!({
                key: { "type": "free", "x": world_pt.x, "y": world_pt.y },
            })),
            ..Default::default()
        };
        self.doc.apply_partial(id, &partial);
    }

    fn create_default_object(&self, kind: ObjectKind, x: f64, y: f64, width: f64, height: f64) -> BoardObject {
        BoardObject {
            id: uuid::Uuid::new_v4(),
            board_id: uuid::Uuid::nil(),
            kind,
            x,
            y,
            width,
            height,
            rotation: 0.0,
            z_index: self.next_z_index(),
            props: serde_json::json!({
                "fill": "#D94B4B",
                "stroke": "#1F1A17",
                "stroke_width": 1,
            }),
            created_by: None,
            version: 1,
        }
    }

    fn next_z_index(&self) -> i64 {
        self.doc
            .sorted_objects()
            .last()
            .map_or(0, |obj| obj.z_index + 1)
    }
}

/// The full canvas engine. Wraps `EngineCore` and owns the browser canvas element.
pub struct Engine {
    _canvas: HtmlCanvasElement,
    pub core: EngineCore,
}

impl Engine {
    /// Create a new engine bound to the given canvas element.
    #[must_use]
    pub fn new(canvas: HtmlCanvasElement) -> Self {
        Self { _canvas: canvas, core: EngineCore::new() }
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
    pub fn set_viewport(&mut self, width_css: f64, height_css: f64, dpr: f64) {
        self.core.set_viewport(width_css, height_css, dpr);
    }

    // --- Input events (delegated) ---

    pub fn on_pointer_down(&mut self, screen_pt: Point, button: Button, modifiers: Modifiers) -> Vec<Action> {
        self.core.on_pointer_down(screen_pt, button, modifiers)
    }

    pub fn on_pointer_move(&mut self, screen_pt: Point, modifiers: Modifiers) -> Vec<Action> {
        self.core.on_pointer_move(screen_pt, modifiers)
    }

    pub fn on_pointer_up(&mut self, screen_pt: Point, button: Button, modifiers: Modifiers) -> Vec<Action> {
        self.core.on_pointer_up(screen_pt, button, modifiers)
    }

    pub fn on_wheel(&mut self, screen_pt: Point, delta: WheelDelta, modifiers: Modifiers) -> Vec<Action> {
        self.core.on_wheel(screen_pt, delta, modifiers)
    }

    pub fn on_key_down(&mut self, key: Key, modifiers: Modifiers) -> Vec<Action> {
        self.core.on_key_down(key, modifiers)
    }

    pub fn on_key_up(&mut self, key: Key, modifiers: Modifiers) -> Vec<Action> {
        self.core.on_key_up(key, modifiers)
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
