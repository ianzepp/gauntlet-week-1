use super::*;

// =============================================================
// UiState defaults
// =============================================================

#[test]
fn ui_state_default_dark_mode_off() {
    let state = UiState::default();
    assert!(!state.dark_mode);
}

#[test]
fn ui_state_default_view_mode_is_canvas() {
    let state = UiState::default();
    assert_eq!(state.view_mode, ViewMode::Canvas);
}

#[test]
fn ui_state_default_tool_is_select() {
    let state = UiState::default();
    assert_eq!(state.active_tool, ToolType::Select);
    assert_eq!(state.home_viewport_seq, 0);
    assert_eq!(state.zoom_override_seq, 0);
    assert_eq!(state.zoom_override, None);
}

#[test]
fn ui_state_default_left_panel_expanded() {
    let state = UiState::default();
    assert!(!state.left_panel_expanded);
    assert_eq!(state.left_panel_width, 160.0);
    assert_eq!(state.left_tab, LeftTab::Tools);
}

#[test]
fn ui_state_default_right_panel_expanded() {
    let state = UiState::default();
    assert!(!state.right_panel_expanded);
    assert_eq!(state.right_panel_width, 320.0);
    assert_eq!(state.right_tab, RightTab::Chat);
    assert_eq!(state.ai_focus_seq, 0);
}

// =============================================================
// ViewMode
// =============================================================

#[test]
fn view_mode_default_is_canvas() {
    assert_eq!(ViewMode::default(), ViewMode::Canvas);
}

#[test]
fn view_mode_variants_are_distinct() {
    assert_ne!(ViewMode::Canvas, ViewMode::Trace);
}

// =============================================================
// ToolType
// =============================================================

#[test]
fn tool_type_default_is_select() {
    assert_eq!(ToolType::default(), ToolType::Select);
}

#[test]
fn tool_type_variants_are_distinct() {
    let variants = [
        ToolType::Select,
        ToolType::Sticky,
        ToolType::Rectangle,
        ToolType::Frame,
        ToolType::Ellipse,
        ToolType::Youtube,
        ToolType::Line,
        ToolType::Connector,
        ToolType::Text,
        ToolType::Draw,
        ToolType::Eraser,
    ];
    for (i, a) in variants.iter().enumerate() {
        for (j, b) in variants.iter().enumerate() {
            if i == j {
                assert_eq!(a, b);
            } else {
                assert_ne!(a, b);
            }
        }
    }
}

// =============================================================
// LeftTab
// =============================================================

#[test]
fn left_tab_default_is_tools() {
    assert_eq!(LeftTab::default(), LeftTab::Tools);
}

#[test]
fn left_tab_variants_are_distinct() {
    assert_ne!(LeftTab::Tools, LeftTab::Inspector);
}

// =============================================================
// RightTab
// =============================================================

#[test]
fn right_tab_default_is_chat() {
    assert_eq!(RightTab::default(), RightTab::Chat);
}

#[test]
fn right_tab_variants_are_distinct() {
    assert_ne!(RightTab::Chat, RightTab::Ai);
    assert_ne!(RightTab::Chat, RightTab::Trace);
    assert_ne!(RightTab::Chat, RightTab::Boards);
    assert_ne!(RightTab::Chat, RightTab::Records);
    assert_ne!(RightTab::Ai, RightTab::Trace);
    assert_ne!(RightTab::Ai, RightTab::Boards);
    assert_ne!(RightTab::Ai, RightTab::Records);
    assert_ne!(RightTab::Trace, RightTab::Boards);
    assert_ne!(RightTab::Trace, RightTab::Records);
    assert_ne!(RightTab::Boards, RightTab::Records);
}
