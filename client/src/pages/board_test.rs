use super::*;
use crate::net::types::FrameStatus;
use crate::pages::board_prompt::assistant_preview_and_has_more;

#[test]
fn reset_board_for_route_change_preserves_client_identity() {
    let mut board = BoardState {
        board_id: Some("b-old".to_owned()),
        board_name: Some("Board Old".to_owned()),
        self_client_id: Some("client-1".to_owned()),
        follow_client_id: Some("client-2".to_owned()),
        jump_to_client_id: Some("client-3".to_owned()),
        ..BoardState::default()
    };

    reset_board_for_route_change(&mut board, Some("b-new".to_owned()));

    assert_eq!(board.board_id.as_deref(), Some("b-new"));
    assert_eq!(board.self_client_id.as_deref(), Some("client-1"));
    assert!(board.follow_client_id.is_none());
    assert!(board.jump_to_client_id.is_none());
    assert!(board.presence.is_empty());
    assert!(board.objects.is_empty());
}

#[test]
fn build_board_membership_frame_sets_protocol_fields() {
    let frame = build_board_membership_frame("board:part", "b-1".to_owned());
    assert_eq!(frame.syscall, "board:part");
    assert_eq!(frame.status, FrameStatus::Request);
    assert_eq!(frame.board_id.as_deref(), Some("b-1"));
    assert_eq!(frame.data, serde_json::json!({}));
}

#[test]
fn assistant_preview_shows_up_to_three_plain_paragraphs_without_more() {
    let text = "Para one.\n\nPara two.\n\nPara three.";
    let (preview, has_more) = assistant_preview_and_has_more(text);
    assert_eq!(preview, text);
    assert!(!has_more);
}

#[test]
fn assistant_preview_flags_more_for_plain_fourth_paragraph() {
    let text = "Para one.\n\nPara two.\n\nPara three.\n\nPara four.";
    let (preview, has_more) = assistant_preview_and_has_more(text);
    assert_eq!(preview, "Para one.\n\nPara two.\n\nPara three.");
    assert!(has_more);
}

#[test]
fn assistant_preview_flags_more_when_intro_paragraph_ends_with_colon() {
    let text = "Here is the plan:\n\n- first\n- second";
    let (preview, has_more) = assistant_preview_and_has_more(text);
    assert_eq!(preview, "Here is the plan:");
    assert!(has_more);
}

#[test]
fn assistant_preview_flags_more_when_list_starts() {
    let text = "Summary paragraph.\n\n1. First item\n2. Second item";
    let (preview, has_more) = assistant_preview_and_has_more(text);
    assert_eq!(preview, "Summary paragraph.");
    assert!(has_more);
}
