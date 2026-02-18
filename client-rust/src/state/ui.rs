#[cfg(test)]
#[path = "ui_test.rs"]
mod ui_test;

/// UI state for panels, tabs, dark mode, and active tool.
///
/// In the full Leptos implementation, fields will be `RwSignal` types
/// provided via context. For now they are plain fields.
#[derive(Clone, Debug)]
pub struct UiState {
    pub dark_mode: bool,
    pub active_tool: ToolType,
    pub left_panel_expanded: bool,
    pub left_tab: LeftTab,
    pub right_panel_expanded: bool,
    pub right_tab: RightTab,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            dark_mode: false,
            active_tool: ToolType::Select,
            left_panel_expanded: true,
            left_tab: LeftTab::Tools,
            right_panel_expanded: true,
            right_tab: RightTab::Chat,
        }
    }
}

/// Available drawing/interaction tools.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToolType {
    #[default]
    Select,
    Rect,
    Ellipse,
    Diamond,
    Star,
    Line,
    Arrow,
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
    Boards,
}
