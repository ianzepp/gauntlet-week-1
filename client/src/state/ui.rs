//! Local UI chrome state (tools, tabs, panel expansion).
//!
//! DESIGN
//! ======
//! Keeps transient presentation concerns out of domain state (`board`, `chat`)
//! so rendering controls can evolve independently of protocol data.

#[cfg(test)]
#[path = "ui_test.rs"]
mod ui_test;

/// Primary view mode for the board workspace.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ViewMode {
    /// Normal canvas editing mode.
    #[default]
    Canvas,
    /// Observability trace view — replaces the canvas area with the three-column trace UI.
    Trace,
}

/// UI state for panels, tabs, dark mode, and active tool.
///
/// In the full Leptos implementation, fields will be `RwSignal` types
/// provided via context. For now they are plain fields.
#[derive(Clone, Debug)]
pub struct UiState {
    pub dark_mode: bool,
    pub view_mode: ViewMode,
    pub active_tool: ToolType,
    pub home_viewport_seq: u64,
    pub zoom_override_seq: u64,
    pub zoom_override: Option<f64>,
    pub view_center_override_seq: u64,
    pub view_center_override: Option<(f64, f64)>,
    pub left_panel_expanded: bool,
    pub left_panel_width: f64,
    pub left_tab: LeftTab,
    pub right_panel_expanded: bool,
    pub right_panel_width: f64,
    pub right_tab: RightTab,
    pub ai_focus_seq: u64,
    pub object_text_dialog_seq: u64,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            dark_mode: false,
            view_mode: ViewMode::Canvas,
            active_tool: ToolType::Select,
            home_viewport_seq: 0,
            zoom_override_seq: 0,
            zoom_override: None,
            view_center_override_seq: 0,
            view_center_override: None,
            left_panel_expanded: false,
            left_panel_width: 160.0,
            left_tab: LeftTab::Tools,
            right_panel_expanded: false,
            right_panel_width: 320.0,
            right_tab: RightTab::Chat,
            ai_focus_seq: 0,
            object_text_dialog_seq: 0,
        }
    }
}

/// Available drawing/interaction tools.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToolType {
    #[default]
    Select,
    Hand,
    Sticky,
    Rectangle,
    Frame,
    Ellipse,
    Line,
    Connector,
    Text,
    Draw,
    Eraser,
}

/// Tabs available in the left panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LeftTab {
    #[default]
    Tools,
    Inspector,
}

/// Tabs available in the right panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RightTab {
    #[default]
    Chat,
    Ai,
    Trace,
    Users,
    Boards,
    Records,
}
