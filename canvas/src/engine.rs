use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::camera::{Camera, Point};
use crate::consts::{MIN_SHAPE_SIZE, ZOOM_FACTOR, ZOOM_MAX, ZOOM_MIN};
use crate::doc::{BoardObject, DocStore, ObjectId, ObjectKind, PartialBoardObject, Props, WorldBounds};
use crate::hit::{self, EdgeEnd, HitPart, ResizeAnchor};
use crate::input::{Button, DragAxis, InputState, Key, Modifiers, SelectionRect, Tool, UiState, WheelDelta};
use crate::render;

const EDGE_ATTACH_SNAP_PX: f64 = 16.0;

#[cfg(test)]
#[path = "engine_test.rs"]
mod engine_test;

/// Actions returned from input handlers for the host to process.
#[derive(Debug, Clone)]
pub enum Action {
    /// No action needed; the host should do nothing.
    None,
    /// A new object was finalized and should be persisted to the server.
    ObjectCreated(BoardObject),
    /// An existing object was mutated locally and the delta should be sent to the server.
    ObjectUpdated {
        /// ID of the object that changed.
        id: ObjectId,
        /// Sparse set of changed fields.
        fields: PartialBoardObject,
    },
    /// An object was removed and the host should broadcast the deletion.
    ObjectDeleted {
        /// ID of the deleted object.
        id: ObjectId,
    },
    /// The host should open the text-editing overlay for this object.
    EditTextRequested {
        /// ID of the object whose text should be edited.
        id: ObjectId,
        /// Current value of the `head` text field.
        head: String,
        /// Current value of the body `text` field.
        text: String,
        /// Current value of the `foot` text field.
        foot: String,
    },
    /// The host should update the CSS cursor to the given value.
    SetCursor(String),
    /// The canvas has changed and the host should call `render()`.
    RenderNeeded,
}

/// Core engine state — all logic that doesn't depend on the canvas element.
///
/// Separated from `Engine` so it can be tested without WASM/browser dependencies.
/// All input handling, gesture tracking, and document mutations live here.
/// [`Engine`] wraps this with a reference to the browser canvas.
pub struct EngineCore {
    /// The in-memory document: all board objects keyed by ID.
    pub doc: DocStore,
    /// Current pan/zoom state for the infinite canvas.
    pub camera: Camera,
    /// Persistent UI state shared with the renderer (active tool, selection).
    pub ui: UiState,
    /// Current gesture state machine state.
    pub input: InputState,
    /// Viewport width in CSS pixels (not physical pixels).
    pub viewport_width: f64,
    /// Viewport height in CSS pixels (not physical pixels).
    pub viewport_height: f64,
    /// Device pixel ratio, used to scale canvas backing store.
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
    /// Create a new engine core with an empty document and default camera.
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
        if let InputState::DraggingObject { ids, originals, .. } = &mut self.input
            && ids.contains(id)
            && let Some((_, ox, oy)) = originals.iter_mut().find(|(oid, _, _)| oid == id)
        {
            if let Some(x) = fields.x {
                *ox = x;
            }
            if let Some(y) = fields.y {
                *oy = y;
            }
        }
    }

    /// Apply a server broadcast: object deleted.
    pub fn apply_delete(&mut self, id: &ObjectId) {
        self.doc.remove(id);
        self.ui.selected_ids.remove(id);
    }

    // --- Tool / text ---

    /// Set the active tool.
    pub fn set_tool(&mut self, tool: Tool) {
        if self.ui.tool != tool {
            // Tool switches should not inherit an in-flight gesture from the
            // previous tool (for example a stuck panning state).
            self.input = InputState::Idle;
            self.ui.marquee = None;
        }
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

    /// Set canvas view rotation in degrees.
    pub fn set_view_rotation_deg(&mut self, deg: f64) {
        self.camera.view_rotation_deg = deg;
    }

    /// Current canvas view rotation in degrees.
    #[must_use]
    pub fn view_rotation_deg(&self) -> f64 {
        self.camera.view_rotation_deg
    }

    // --- Input events ---

    /// Handle a pointer-down event. Returns actions for the host.
    pub fn on_pointer_down(&mut self, screen_pt: Point, button: Button, modifiers: Modifiers) -> Vec<Action> {
        let world_pt = self.screen_to_world(screen_pt);
        let mut actions = Vec::new();

        // Middle button, space+drag, or hand tool always pans.
        if button == Button::Middle
            || (button == Button::Primary && (self.ui.space_pan || self.ui.tool == Tool::Hand))
        {
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
                self.handle_select_down(world_pt, modifiers, &mut actions);
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
    pub fn on_pointer_move(&mut self, screen_pt: Point, modifiers: Modifiers) -> Vec<Action> {
        let world_pt = self.screen_to_world(screen_pt);

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
            InputState::DraggingObject { ids, last_world: _, start_world, originals, mut axis_lock, duplicated } => {
                let mut dx = world_pt.x - start_world.x;
                let mut dy = world_pt.y - start_world.y;
                if modifiers.shift {
                    if axis_lock.is_none() {
                        axis_lock = Some(if dx.abs() >= dy.abs() { DragAxis::X } else { DragAxis::Y });
                    }
                    match axis_lock {
                        Some(DragAxis::X) => dy = 0.0,
                        Some(DragAxis::Y) => dx = 0.0,
                        None => {}
                    }
                } else {
                    axis_lock = None;
                }

                for (id, ox, oy) in &originals {
                    let partial = PartialBoardObject { x: Some(*ox + dx), y: Some(*oy + dy), ..Default::default() };
                    self.doc.apply_partial(id, &partial);
                }
                self.input = InputState::DraggingObject {
                    ids,
                    last_world: world_pt,
                    start_world,
                    originals,
                    axis_lock,
                    duplicated,
                };
                vec![Action::RenderNeeded]
            }
            InputState::MarqueeSelecting { anchor_world, .. } => {
                self.update_marquee(anchor_world, world_pt);
                self.input = InputState::MarqueeSelecting { anchor_world, last_world: world_pt };
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
                self.apply_resize(id, anchor, dx, dy, orig_x, orig_y, orig_w, orig_h, rotation);
                self.input = InputState::ResizingObject { id, anchor, start_world, orig_x, orig_y, orig_w, orig_h };
                vec![Action::RenderNeeded]
            }
            InputState::RotatingObject { id, center, orig_rotation } => {
                let angle = (world_pt.y - center.y)
                    .atan2(world_pt.x - center.x)
                    .to_degrees()
                    + 90.0;
                let delta = normalize_angle_delta(angle - orig_rotation);

                if let Some(obj) = self.doc.get(&id).cloned()
                    && obj.kind == ObjectKind::Frame
                    && delta.abs() > f64::EPSILON
                {
                    let child_ids = self.grouped_children_in_frame(&obj);
                    self.rotate_children_around_pivot(&child_ids, center, delta);
                }
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
            InputState::DraggingObject { ids, originals, duplicated, .. } => {
                for id in &ids {
                    if let Some(obj) = self.doc.get(id) {
                        let mut changed = true;
                        if let Some((_, ox, oy)) = originals.iter().find(|(orig_id, _, _)| orig_id == id) {
                            changed = (obj.x - *ox).abs() > f64::EPSILON || (obj.y - *oy).abs() > f64::EPSILON;
                        }
                        if changed {
                            let partial = PartialBoardObject { x: Some(obj.x), y: Some(obj.y), ..Default::default() };
                            actions.push(Action::ObjectUpdated { id: *id, fields: partial });
                        }
                    }
                }
                if duplicated {
                    for id in &ids {
                        if let Some(obj) = self.doc.get(id).cloned() {
                            actions.push(Action::ObjectCreated(obj));
                        }
                    }
                }
            }
            InputState::MarqueeSelecting { anchor_world, last_world } => {
                self.ui.marquee = None;
                self.select_by_marquee(anchor_world, last_world);
                actions.push(Action::RenderNeeded);
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

                    // Rotating a frame is a grouped transform: persist child geometry + rotation.
                    if obj.kind == ObjectKind::Frame {
                        let child_ids = self.grouped_children_in_frame(obj);
                        for child_id in child_ids {
                            if let Some(child) = self.doc.get(&child_id) {
                                actions.push(Action::ObjectUpdated {
                                    id: child_id,
                                    fields: PartialBoardObject {
                                        x: Some(child.x),
                                        y: Some(child.y),
                                        rotation: Some(child.rotation),
                                        ..Default::default()
                                    },
                                });
                            }
                        }
                    }
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
            let anchor_world = self.screen_to_world(screen_pt);
            let factor = if delta.dy < 0.0 { ZOOM_FACTOR } else { 1.0 / ZOOM_FACTOR };
            self.camera.zoom = (self.camera.zoom * factor).clamp(ZOOM_MIN, ZOOM_MAX);

            // Adjust pan so the world point under the cursor stays fixed,
            // accounting for view rotation around viewport center.
            let center = self.viewport_center();
            let unrotated_screen = hit::rotate_point(screen_pt, center, -self.camera.view_rotation_deg);
            self.camera.pan_x = unrotated_screen.x - (anchor_world.x * self.camera.zoom);
            self.camera.pan_y = unrotated_screen.y - (anchor_world.y * self.camera.zoom);
        } else {
            // Pan.
            self.camera.pan_x -= delta.dx;
            self.camera.pan_y -= delta.dy;
        }
        vec![Action::RenderNeeded]
    }

    /// Handle a key-down event. Returns actions for the host.
    pub fn on_key_down(&mut self, key: Key, modifiers: Modifiers) -> Vec<Action> {
        let mut actions = Vec::new();
        let accel = modifiers.ctrl || modifiers.meta;

        match key.0.as_str() {
            " " => {
                self.ui.space_pan = true;
            }
            "Delete" | "Backspace" => {
                let selected = self.ui.selected_ids.iter().copied().collect::<Vec<_>>();
                let mut deleted_any = false;
                for id in selected {
                    self.doc.remove(&id);
                    self.ui.selected_ids.remove(&id);
                    actions.push(Action::ObjectDeleted { id });
                    deleted_any = true;
                }
                if deleted_any {
                    actions.push(Action::RenderNeeded);
                }
            }
            "Escape" => {
                // Cancel active gesture and deselect.
                self.input = InputState::Idle;
                self.ui.marquee = None;
                if !self.ui.selected_ids.is_empty() {
                    self.ui.selected_ids.clear();
                    actions.push(Action::RenderNeeded);
                }
            }
            "Enter" => {
                if self.ui.selected_ids.len() == 1
                    && let Some(id) = self.primary_selection()
                {
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
            "a" | "A" if accel => {
                self.ui.selected_ids = self
                    .doc
                    .sorted_objects()
                    .into_iter()
                    .map(|o| o.id)
                    .collect();
                actions.push(Action::RenderNeeded);
            }
            "g" | "G" if accel && modifiers.shift => {
                for id in self.ui.selected_ids.iter().copied().collect::<Vec<_>>() {
                    let partial = PartialBoardObject { group_id: Some(None), ..Default::default() };
                    self.doc.apply_partial(&id, &partial);
                    actions.push(Action::ObjectUpdated { id, fields: partial });
                }
                actions.push(Action::RenderNeeded);
            }
            "g" | "G" if accel => {
                if self.ui.selected_ids.len() >= 2 {
                    let group_id = uuid::Uuid::new_v4();
                    for id in self.ui.selected_ids.iter().copied().collect::<Vec<_>>() {
                        let partial = PartialBoardObject { group_id: Some(Some(group_id)), ..Default::default() };
                        self.doc.apply_partial(&id, &partial);
                        actions.push(Action::ObjectUpdated { id, fields: partial });
                    }
                    actions.push(Action::RenderNeeded);
                }
            }
            "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight" => {
                let step = if modifiers.shift { 10.0 } else { 1.0 };
                let (dx, dy) = match key.0.as_str() {
                    "ArrowUp" => (0.0, -step),
                    "ArrowDown" => (0.0, step),
                    "ArrowLeft" => (-step, 0.0),
                    "ArrowRight" => (step, 0.0),
                    _ => (0.0, 0.0),
                };
                for id in self.ui.selected_ids.iter().copied().collect::<Vec<_>>() {
                    if let Some(obj) = self.doc.get(&id) {
                        let partial =
                            PartialBoardObject { x: Some(obj.x + dx), y: Some(obj.y + dy), ..Default::default() };
                        self.doc.apply_partial(&id, &partial);
                        actions.push(Action::ObjectUpdated { id, fields: partial });
                    }
                }
                actions.push(Action::RenderNeeded);
            }
            _ => {}
        }

        actions
    }

    /// Handle a key-up event. No-op for v0.
    pub fn on_key_up(&mut self, key: Key, _modifiers: Modifiers) -> Vec<Action> {
        if key.0 == " " {
            self.ui.space_pan = false;
        }
        Vec::new()
    }

    // --- Queries ---

    /// The currently selected object, if any.
    #[must_use]
    pub fn selection(&self) -> Option<ObjectId> {
        self.primary_selection()
    }

    /// All selected ids (order not guaranteed).
    #[must_use]
    pub fn selections(&self) -> Vec<ObjectId> {
        self.ui.selected_ids.iter().copied().collect()
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

    fn handle_select_down(&mut self, world_pt: Point, modifiers: Modifiers, actions: &mut Vec<Action>) {
        let selected_for_handles = if self.ui.selected_ids.len() == 1 {
            self.primary_selection()
        } else {
            None
        };
        let hit = hit::hit_test(world_pt, &self.doc, &self.camera, selected_for_handles);

        if let Some(h) = hit {
            match h.part {
                HitPart::ResizeHandle(anchor) => {
                    if self.ui.selected_ids.len() == 1
                        && self.ui.selected_ids.contains(&h.object_id)
                        && let Some(obj) = self.doc.get(&h.object_id)
                    {
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
                    if self.ui.selected_ids.len() == 1
                        && self.ui.selected_ids.contains(&h.object_id)
                        && let Some(obj) = self.doc.get(&h.object_id)
                    {
                        let center = Point::new(obj.x + obj.width / 2.0, obj.y + obj.height / 2.0);
                        self.input =
                            InputState::RotatingObject { id: h.object_id, center, orig_rotation: obj.rotation };
                    }
                }
                HitPart::EdgeEndpoint(end) => {
                    self.ui.selected_ids.clear();
                    self.ui.selected_ids.insert(h.object_id);
                    self.input = InputState::DraggingEdgeEndpoint { id: h.object_id, end };
                    actions.push(Action::RenderNeeded);
                }
                HitPart::Body | HitPart::EdgeBody => {
                    if modifiers.shift {
                        if self.ui.selected_ids.contains(&h.object_id) {
                            self.ui.selected_ids.remove(&h.object_id);
                        } else {
                            self.ui.selected_ids.insert(h.object_id);
                        }
                        actions.push(Action::RenderNeeded);
                        return;
                    }

                    let hit_already_selected = self.ui.selected_ids.contains(&h.object_id);
                    if !hit_already_selected {
                        self.ui.selected_ids.clear();
                        self.ui.selected_ids.insert(h.object_id);
                        // First click selects only. Require a subsequent drag gesture
                        // on an already-selected object to start object movement.
                        actions.push(Action::RenderNeeded);
                        return;
                    }
                    let mut drag_ids = self.ui.selected_ids.iter().copied().collect::<Vec<_>>();
                    drag_ids.sort_unstable();
                    if modifiers.alt {
                        drag_ids = self.duplicate_objects(&drag_ids);
                        self.ui.selected_ids.clear();
                        self.ui.selected_ids.extend(drag_ids.iter().copied());
                    }
                    let originals = drag_ids
                        .iter()
                        .filter_map(|id| self.doc.get(id).map(|obj| (*id, obj.x, obj.y)))
                        .collect::<Vec<_>>();
                    if !originals.is_empty() {
                        self.input = InputState::DraggingObject {
                            ids: drag_ids,
                            last_world: world_pt,
                            start_world: world_pt,
                            originals,
                            axis_lock: None,
                            duplicated: modifiers.alt,
                        };
                    }
                    actions.push(Action::RenderNeeded);
                }
            }
        } else {
            // Click on empty space: deselect.
            if !self.ui.selected_ids.is_empty() {
                self.ui.selected_ids.clear();
                actions.push(Action::RenderNeeded);
            }
            self.input = InputState::MarqueeSelecting { anchor_world: world_pt, last_world: world_pt };
            self.update_marquee(world_pt, world_pt);
            actions.push(Action::RenderNeeded);
        }
    }

    fn handle_shape_tool_down(&mut self, world_pt: Point, tool: Tool, actions: &mut Vec<Action>) {
        let kind = match tool {
            Tool::Rect => ObjectKind::Rect,
            Tool::Text => ObjectKind::Text,
            Tool::Diamond => ObjectKind::Diamond,
            Tool::Ellipse => ObjectKind::Ellipse,
            Tool::Star => ObjectKind::Star,
            _ => return,
        };
        let obj = self.create_default_object(kind, world_pt.x, world_pt.y, 0.0, 0.0);
        let id = obj.id;
        self.doc.insert(obj);
        self.ui.selected_ids.clear();
        self.ui.selected_ids.insert(id);
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
        self.ui.selected_ids.clear();
        self.ui.selected_ids.insert(id);
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
        rotation: f64,
    ) {
        // Resize in the object's local (unrotated) space, centered at origin.
        // This keeps handle anchoring consistent at arbitrary rotations.
        let mut left = -orig_w * 0.5;
        let mut top = -orig_h * 0.5;
        let mut right = orig_w * 0.5;
        let mut bottom = orig_h * 0.5;

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

        let w = right - left;
        let h = bottom - top;
        let local_center_offset = Point::new((left + right) * 0.5, (top + bottom) * 0.5);
        let orig_center = Point::new(orig_x + orig_w * 0.5, orig_y + orig_h * 0.5);
        let world_center = hit::rotate_point(
            Point::new(orig_center.x + local_center_offset.x, orig_center.y + local_center_offset.y),
            orig_center,
            rotation,
        );
        let x = world_center.x - w * 0.5;
        let y = world_center.y - h * 0.5;

        let partial =
            PartialBoardObject { x: Some(x), y: Some(y), width: Some(w), height: Some(h), ..Default::default() };
        self.doc.apply_partial(&id, &partial);
    }

    fn apply_edge_endpoint_move(&mut self, id: &ObjectId, end: EdgeEnd, world_pt: Point) {
        let key = match end {
            EdgeEnd::A => "a",
            EdgeEnd::B => "b",
        };
        let endpoint = if let Some((target_id, ux, uy, snapped_world)) = self.find_edge_attachment_target(*id, world_pt)
        {
            serde_json::json!({
                "type": "attached",
                "object_id": target_id,
                "ux": ux,
                "uy": uy,
                "x": snapped_world.x,
                "y": snapped_world.y
            })
        } else {
            serde_json::json!({
                "type": "free",
                "x": world_pt.x,
                "y": world_pt.y
            })
        };
        let partial = PartialBoardObject {
            props: Some(serde_json::json!({
                key: endpoint,
            })),
            ..Default::default()
        };
        self.doc.apply_partial(id, &partial);
    }

    fn find_edge_attachment_target(&self, edge_id: ObjectId, world_pt: Point) -> Option<(ObjectId, f64, f64, Point)> {
        let snap_radius = self.camera.screen_dist_to_world(EDGE_ATTACH_SNAP_PX);
        let mut best: Option<(ObjectId, f64, f64, Point, f64)> = None;

        let objects = self.doc.sorted_objects_in_bounds(WorldBounds::from_point(world_pt.x, world_pt.y).expand(snap_radius));
        for obj in objects.iter().rev() {
            if obj.id == edge_id {
                continue;
            }
            if matches!(obj.kind, ObjectKind::Line | ObjectKind::Arrow) {
                continue;
            }

            let (ux, uy, snapped_world) = anchor_on_object_boundary(obj, world_pt);
            let dist = hit::distance_to_segment(world_pt, snapped_world, snapped_world);
            if dist > snap_radius {
                continue;
            }

            match best {
                Some((_, _, _, _, best_dist)) if dist >= best_dist => {}
                _ => best = Some((obj.id, ux, uy, snapped_world, dist)),
            }
        }

        best.map(|(id, ux, uy, world, _)| (id, ux, uy, world))
    }

    fn grouped_children_in_frame(&self, frame: &BoardObject) -> Vec<ObjectId> {
        self.doc
            .sorted_objects()
            .into_iter()
            .filter_map(|obj| {
                if obj.id == frame.id {
                    return None;
                }
                if matches!(obj.kind, ObjectKind::Line | ObjectKind::Arrow) {
                    return None;
                }
                let center = Point::new(obj.x + (obj.width * 0.5), obj.y + (obj.height * 0.5));
                hit::point_in_rect(center, frame.x, frame.y, frame.width, frame.height, frame.rotation)
                    .then_some(obj.id)
            })
            .collect()
    }

    fn rotate_children_around_pivot(&mut self, child_ids: &[ObjectId], pivot: Point, delta_deg: f64) {
        for child_id in child_ids {
            if let Some(child) = self.doc.get(child_id).cloned() {
                let center = Point::new(child.x + (child.width * 0.5), child.y + (child.height * 0.5));
                let rotated_center = hit::rotate_point(center, pivot, delta_deg);
                let partial = PartialBoardObject {
                    x: Some(rotated_center.x - (child.width * 0.5)),
                    y: Some(rotated_center.y - (child.height * 0.5)),
                    rotation: Some(child.rotation + delta_deg),
                    ..Default::default()
                };
                self.doc.apply_partial(child_id, &partial);
            }
        }
    }

    fn create_default_object(&self, kind: ObjectKind, x: f64, y: f64, width: f64, height: f64) -> BoardObject {
        let props = if kind == ObjectKind::Text {
            serde_json::json!({ "text": "Text" })
        } else {
            serde_json::json!({
                "fill": "#D94B4B",
                "stroke": "#1F1A17",
                "stroke_width": 0,
            })
        };
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
            props,
            created_by: None,
            version: 1,
            group_id: None,
        }
    }

    fn next_z_index(&self) -> i64 {
        self.doc
            .sorted_objects()
            .last()
            .map_or(0, |obj| obj.z_index + 1)
    }

    fn viewport_center(&self) -> Point {
        Point::new(self.viewport_width * 0.5, self.viewport_height * 0.5)
    }

    fn screen_to_world(&self, screen_pt: Point) -> Point {
        self.camera
            .screen_to_world(screen_pt, self.viewport_center())
    }

    fn primary_selection(&self) -> Option<ObjectId> {
        self.ui.selected_ids.iter().copied().min()
    }

    fn duplicate_objects(&mut self, ids: &[ObjectId]) -> Vec<ObjectId> {
        let mut duplicated = Vec::new();
        for id in ids {
            if let Some(mut obj) = self.doc.get(id).cloned() {
                obj.id = uuid::Uuid::new_v4();
                obj.z_index = self.next_z_index();
                obj.version = 1;
                self.doc.insert(obj.clone());
                duplicated.push(obj.id);
            }
        }
        duplicated
    }

    fn update_marquee(&mut self, anchor: Point, current: Point) {
        let x = anchor.x.min(current.x);
        let y = anchor.y.min(current.y);
        let width = (current.x - anchor.x).abs();
        let height = (current.y - anchor.y).abs();
        self.ui.marquee = Some(SelectionRect { x, y, width, height });
    }

    fn select_by_marquee(&mut self, anchor: Point, current: Point) {
        let x = anchor.x.min(current.x);
        let y = anchor.y.min(current.y);
        let width = (current.x - anchor.x).abs();
        let height = (current.y - anchor.y).abs();

        if width <= f64::EPSILON || height <= f64::EPSILON {
            return;
        }

        self.ui.selected_ids.clear();
        for obj in self.doc.sorted_objects() {
            if self.rect_intersects_object(x, y, width, height, obj) {
                self.ui.selected_ids.insert(obj.id);
            }
        }
    }

    fn rect_intersects_object(&self, x: f64, y: f64, width: f64, height: f64, obj: &BoardObject) -> bool {
        let rect_right = x + width;
        let rect_bottom = y + height;

        let (obj_left, obj_top, obj_right, obj_bottom) = match obj.kind {
            ObjectKind::Line | ObjectKind::Arrow => {
                let a = hit::edge_endpoint_a_resolved(obj, &self.doc);
                let b = hit::edge_endpoint_b_resolved(obj, &self.doc);
                let Some(a) = a else { return false };
                let Some(b) = b else { return false };
                (a.x.min(b.x), a.y.min(b.y), a.x.max(b.x), a.y.max(b.y))
            }
            _ => {
                let handles = hit::resize_handle_positions(obj.x, obj.y, obj.width, obj.height, obj.rotation);
                let mut min_x = f64::INFINITY;
                let mut min_y = f64::INFINITY;
                let mut max_x = f64::NEG_INFINITY;
                let mut max_y = f64::NEG_INFINITY;
                for p in handles {
                    min_x = min_x.min(p.x);
                    min_y = min_y.min(p.y);
                    max_x = max_x.max(p.x);
                    max_y = max_y.max(p.y);
                }
                (min_x, min_y, max_x, max_y)
            }
        };

        obj_left <= rect_right && obj_right >= x && obj_top <= rect_bottom && obj_bottom >= y
    }
}

fn normalize_angle_delta(delta: f64) -> f64 {
    ((delta + 180.0).rem_euclid(360.0)) - 180.0
}

fn anchor_on_object_boundary(obj: &BoardObject, world_pt: Point) -> (f64, f64, Point) {
    if obj.width <= 0.0 || obj.height <= 0.0 {
        let center = Point::new(obj.x, obj.y);
        return (0.5, 0.5, center);
    }

    let local = hit::world_to_local(world_pt, obj.x, obj.y, obj.width, obj.height, obj.rotation);

    let (local_x, local_y) = if obj.kind == ObjectKind::Ellipse {
        nearest_ellipse_boundary_local(local, obj.width, obj.height)
    } else {
        nearest_rect_boundary_local(local, obj.width, obj.height)
    };

    let ux = (local_x / obj.width).clamp(0.0, 1.0);
    let uy = (local_y / obj.height).clamp(0.0, 1.0);
    let snapped = hit::attached_anchor_world_point(obj, ux, uy);
    (ux, uy, snapped)
}

fn nearest_rect_boundary_local(local: Point, width: f64, height: f64) -> (f64, f64) {
    let x = local.x.clamp(0.0, width);
    let y = local.y.clamp(0.0, height);

    let inside = local.x >= 0.0 && local.x <= width && local.y >= 0.0 && local.y <= height;
    if !inside {
        return (x, y);
    }

    let to_left = x;
    let to_right = width - x;
    let to_top = y;
    let to_bottom = height - y;
    let min = to_left.min(to_right).min(to_top).min(to_bottom);

    if (min - to_left).abs() < f64::EPSILON {
        (0.0, y)
    } else if (min - to_right).abs() < f64::EPSILON {
        (width, y)
    } else if (min - to_top).abs() < f64::EPSILON {
        (x, 0.0)
    } else {
        (x, height)
    }
}

fn nearest_ellipse_boundary_local(local: Point, width: f64, height: f64) -> (f64, f64) {
    let cx = width * 0.5;
    let cy = height * 0.5;
    if cx <= f64::EPSILON || cy <= f64::EPSILON {
        return nearest_rect_boundary_local(local, width, height);
    }

    let dx = local.x - cx;
    let dy = local.y - cy;
    let denom = (dx * dx) / (cx * cx) + (dy * dy) / (cy * cy);
    if denom <= 1e-9 {
        return (cx, 0.0);
    }

    let scale = 1.0 / denom.sqrt();
    (cx + dx * scale, cy + dy * scale)
}

/// The full canvas engine. Wraps `EngineCore` and owns the browser canvas element.
pub struct Engine {
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

    /// Hydrate the document from a server snapshot.
    pub fn load_snapshot(&mut self, objects: Vec<BoardObject>) {
        self.core.load_snapshot(objects);
    }

    /// Apply a server broadcast: object created.
    pub fn apply_create(&mut self, object: BoardObject) {
        self.core.apply_create(object);
    }

    /// Apply a server broadcast: object updated.
    pub fn apply_update(&mut self, id: &ObjectId, fields: &PartialBoardObject) {
        self.core.apply_update(id, fields);
    }

    /// Apply a server broadcast: object deleted.
    pub fn apply_delete(&mut self, id: &ObjectId) {
        self.core.apply_delete(id);
    }

    /// Set the active tool.
    pub fn set_tool(&mut self, tool: Tool) {
        self.core.set_tool(tool);
    }

    /// Commit text from the host editor back into the object's props.
    pub fn set_text(&mut self, id: &ObjectId, head: String, text: String, foot: String) -> Action {
        self.core.set_text(id, head, text, foot)
    }

    // --- Viewport ---

    /// Update viewport dimensions and device pixel ratio.
    pub fn set_viewport(&mut self, width_css: f64, height_css: f64, dpr: f64) {
        self.core.set_viewport(width_css, height_css, dpr);
    }

    /// Set canvas view rotation in degrees.
    pub fn set_view_rotation_deg(&mut self, deg: f64) {
        self.core.set_view_rotation_deg(deg);
    }

    /// Current canvas view rotation in degrees.
    #[must_use]
    pub fn view_rotation_deg(&self) -> f64 {
        self.core.view_rotation_deg()
    }

    // --- Input events (delegated) ---

    /// Handle a pointer-down event. Returns actions for the host.
    pub fn on_pointer_down(&mut self, screen_pt: Point, button: Button, modifiers: Modifiers) -> Vec<Action> {
        self.core.on_pointer_down(screen_pt, button, modifiers)
    }

    /// Handle a pointer-move event. Returns actions for the host.
    pub fn on_pointer_move(&mut self, screen_pt: Point, modifiers: Modifiers) -> Vec<Action> {
        self.core.on_pointer_move(screen_pt, modifiers)
    }

    /// Handle a pointer-up event. Returns actions for the host.
    pub fn on_pointer_up(&mut self, screen_pt: Point, button: Button, modifiers: Modifiers) -> Vec<Action> {
        self.core.on_pointer_up(screen_pt, button, modifiers)
    }

    /// Handle a wheel/scroll event. Returns actions for the host.
    pub fn on_wheel(&mut self, screen_pt: Point, delta: WheelDelta, modifiers: Modifiers) -> Vec<Action> {
        self.core.on_wheel(screen_pt, delta, modifiers)
    }

    /// Handle a key-down event. Returns actions for the host.
    pub fn on_key_down(&mut self, key: Key, modifiers: Modifiers) -> Vec<Action> {
        self.core.on_key_down(key, modifiers)
    }

    /// Handle a key-up event. No-op for v0.
    pub fn on_key_up(&mut self, key: Key, modifiers: Modifiers) -> Vec<Action> {
        self.core.on_key_up(key, modifiers)
    }

    // --- Render ---

    /// Draw the current state to the canvas.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the 2D context cannot be obtained or any `Canvas2D` call fails.
    pub fn render(&self) -> Result<(), wasm_bindgen::JsValue> {
        let ctx: CanvasRenderingContext2d = self
            .canvas
            .get_context("2d")?
            .ok_or_else(|| wasm_bindgen::JsValue::from_str("no 2d context"))?
            .dyn_into()?;

        // Resize backing store to match viewport × DPR.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let phys_w = (self.core.viewport_width * self.core.dpr) as u32;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let phys_h = (self.core.viewport_height * self.core.dpr) as u32;
        self.canvas.set_width(phys_w);
        self.canvas.set_height(phys_h);

        render::draw(
            &ctx,
            &self.core.doc,
            &self.core.camera,
            &self.core.ui,
            self.core.viewport_width,
            self.core.viewport_height,
            self.core.dpr,
        )
    }

    // --- Delegated queries ---

    /// The currently selected object, if any.
    #[must_use]
    pub fn selection(&self) -> Option<ObjectId> {
        self.core.selection()
    }

    /// All selected object ids.
    #[must_use]
    pub fn selections(&self) -> Vec<ObjectId> {
        self.core.selections()
    }

    /// The current camera state.
    #[must_use]
    pub fn camera(&self) -> Camera {
        self.core.camera()
    }

    /// Look up an object by ID.
    #[must_use]
    pub fn object(&self, id: &ObjectId) -> Option<&BoardObject> {
        self.core.object(id)
    }
}
