use super::*;

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
