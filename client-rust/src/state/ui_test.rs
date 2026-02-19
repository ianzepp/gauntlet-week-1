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
fn ui_state_default_tool_is_select() {
    let state = UiState::default();
    assert_eq!(state.active_tool, ToolType::Select);
}

#[test]
fn ui_state_default_left_panel_expanded() {
    let state = UiState::default();
    assert!(!state.left_panel_expanded);
    assert_eq!(state.left_tab, LeftTab::Tools);
}

#[test]
fn ui_state_default_right_panel_expanded() {
    let state = UiState::default();
    assert!(!state.right_panel_expanded);
    assert_eq!(state.right_tab, RightTab::Chat);
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
    assert_ne!(RightTab::Chat, RightTab::Boards);
    assert_ne!(RightTab::Ai, RightTab::Boards);
}
