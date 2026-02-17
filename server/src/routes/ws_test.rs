
use super::*;
use crate::frame::Status;
use crate::llm::types::{ChatResponse, ContentBlock, LlmChat, LlmError, Message, Tool};
use crate::state::test_helpers;
use serde_json::json;
use std::sync::{Arc, Mutex};
use tokio::time::{Duration, timeout};

struct MockLlm {
    responses: Mutex<Vec<ChatResponse>>,
}

impl MockLlm {
    fn new(responses: Vec<ChatResponse>) -> Self {
        Self { responses: Mutex::new(responses) }
    }
}

#[async_trait::async_trait]
impl LlmChat for MockLlm {
    async fn chat(
        &self,
        _max_tokens: u32,
        _system: &str,
        _messages: &[Message],
        _tools: Option<&[Tool]>,
    ) -> Result<ChatResponse, LlmError> {
        let mut responses = self.responses.lock().expect("mock mutex should lock");
        if responses.is_empty() {
            Ok(ChatResponse {
                content: vec![ContentBlock::Text { text: "done".into() }],
                model: "mock".into(),
                stop_reason: "end_turn".into(),
                input_tokens: 0,
                output_tokens: 0,
            })
        } else {
            Ok(responses.remove(0))
        }
    }
}

fn ai_prompt_json(board_id: Uuid, prompt: &str) -> String {
    let mut data = Data::new();
    data.insert("prompt".into(), json!(prompt));
    let req = Frame::request("ai:prompt", data).with_board_id(board_id);
    serde_json::to_string(&req).expect("frame should serialize")
}

async fn recv_board_broadcast(rx: &mut mpsc::Receiver<Frame>) -> Frame {
    timeout(Duration::from_millis(500), rx.recv())
        .await
        .expect("broadcast receive timed out")
        .expect("broadcast channel closed unexpectedly")
}

async fn recv_board_broadcasts(rx: &mut mpsc::Receiver<Frame>, count: usize) -> Vec<Frame> {
    let mut frames = Vec::with_capacity(count);
    for _ in 0..count {
        frames.push(recv_board_broadcast(rx).await);
    }
    frames
}

async fn register_two_clients(
    state: &AppState,
    board_id: Uuid,
) -> (Uuid, mpsc::Sender<Frame>, mpsc::Receiver<Frame>, Uuid, mpsc::Receiver<Frame>) {
    let sender_client_id = Uuid::new_v4();
    let peer_client_id = Uuid::new_v4();

    let (sender_tx, sender_rx) = mpsc::channel(32);
    let (peer_tx, peer_rx) = mpsc::channel(32);

    let mut boards = state.boards.write().await;
    let board = boards
        .get_mut(&board_id)
        .expect("board should exist in memory");
    board.clients.insert(sender_client_id, sender_tx.clone());
    board.clients.insert(peer_client_id, peer_tx);

    (sender_client_id, sender_tx, sender_rx, peer_client_id, peer_rx)
}

#[tokio::test]
async fn ai_prompt_create_sticky_broadcasts_mutation_and_replies_with_text() {
    let llm: Arc<dyn LlmChat> = Arc::new(MockLlm::new(vec![
        ChatResponse {
            content: vec![ContentBlock::ToolUse {
                id: "tool_1".into(),
                name: "createStickyNote".into(),
                input: json!({ "text": "hello from ai", "x": 220, "y": 180, "color": "#FFEB3B" }),
            }],
            model: "mock".into(),
            stop_reason: "tool_use".into(),
            input_tokens: 25,
            output_tokens: 30,
        },
        ChatResponse {
            content: vec![ContentBlock::Text { text: "Created a sticky.".into() }],
            model: "mock".into(),
            stop_reason: "end_turn".into(),
            input_tokens: 10,
            output_tokens: 6,
        },
    ]));
    let state = test_helpers::test_app_state_with_llm(llm);
    let board_id = test_helpers::seed_board(&state).await;

    let (sender_client_id, sender_tx, mut sender_rx, _peer_client_id, mut peer_rx) =
        register_two_clients(&state, board_id).await;
    let user_id = Uuid::new_v4();
    let mut current_board = Some(board_id);

    let sender_frames = process_inbound_text(
        &state,
        &mut current_board,
        sender_client_id,
        user_id,
        &sender_tx,
        &ai_prompt_json(board_id, "create a sticky"),
    )
    .await;

    assert_eq!(sender_frames.len(), 1);
    let reply = &sender_frames[0];
    assert_eq!(reply.syscall, "ai:prompt");
    assert_eq!(reply.status, Status::Done);
    assert_eq!(reply.data.get("mutations").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(reply.data.get("text").and_then(|v| v.as_str()), Some("Created a sticky."));

    let sender_broadcast = recv_board_broadcast(&mut sender_rx).await;
    let peer_broadcast = recv_board_broadcast(&mut peer_rx).await;
    assert_eq!(sender_broadcast.syscall, "object:create");
    assert_eq!(peer_broadcast.syscall, "object:create");
    assert_eq!(sender_broadcast.data.get("kind").and_then(|v| v.as_str()), Some("sticky_note"));
    assert_eq!(peer_broadcast.data.get("kind").and_then(|v| v.as_str()), Some("sticky_note"));

    let boards = state.boards.read().await;
    let board = boards.get(&board_id).expect("board should be present");
    assert_eq!(board.objects.len(), 1);
    let created = board
        .objects
        .values()
        .next()
        .expect("created sticky should exist");
    assert_eq!(created.kind, "sticky_note");
    assert_eq!(created.props.get("text").and_then(|v| v.as_str()), Some("hello from ai"));
}

#[tokio::test]
async fn ai_prompt_resize_sticky_broadcasts_update_and_replies_with_text() {
    let mut sticky = test_helpers::dummy_object();
    sticky.version = 0;
    sticky.width = Some(120.0);
    sticky.height = Some(90.0);
    sticky.props = json!({ "text": "sticky 4", "color": "#FFEB3B" });
    let target_id = sticky.id;

    let llm: Arc<dyn LlmChat> = Arc::new(MockLlm::new(vec![
        ChatResponse {
            content: vec![ContentBlock::ToolUse {
                id: "tool_1".into(),
                name: "resizeObject".into(),
                input: json!({ "objectId": target_id, "width": 420, "height": 260 }),
            }],
            model: "mock".into(),
            stop_reason: "tool_use".into(),
            input_tokens: 20,
            output_tokens: 28,
        },
        ChatResponse {
            content: vec![ContentBlock::Text { text: "Resized sticky 4.".into() }],
            model: "mock".into(),
            stop_reason: "end_turn".into(),
            input_tokens: 8,
            output_tokens: 5,
        },
    ]));
    let state = test_helpers::test_app_state_with_llm(llm);
    let board_id = test_helpers::seed_board_with_objects(&state, vec![sticky]).await;

    let (sender_client_id, sender_tx, mut sender_rx, _peer_client_id, mut peer_rx) =
        register_two_clients(&state, board_id).await;
    let user_id = Uuid::new_v4();
    let mut current_board = Some(board_id);

    let sender_frames = process_inbound_text(
        &state,
        &mut current_board,
        sender_client_id,
        user_id,
        &sender_tx,
        &ai_prompt_json(board_id, "resize sticky 4"),
    )
    .await;

    assert_eq!(sender_frames.len(), 1);
    let reply = &sender_frames[0];
    assert_eq!(reply.syscall, "ai:prompt");
    assert_eq!(reply.status, Status::Done);
    assert_eq!(reply.data.get("mutations").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(reply.data.get("text").and_then(|v| v.as_str()), Some("Resized sticky 4."));

    let sender_broadcast = recv_board_broadcast(&mut sender_rx).await;
    let peer_broadcast = recv_board_broadcast(&mut peer_rx).await;
    assert_eq!(sender_broadcast.syscall, "object:update");
    assert_eq!(peer_broadcast.syscall, "object:update");
    let target_id_str = target_id.to_string();
    assert_eq!(
        sender_broadcast.data.get("id").and_then(|v| v.as_str()),
        Some(target_id_str.as_str())
    );
    assert_eq!(
        peer_broadcast.data.get("id").and_then(|v| v.as_str()),
        Some(target_id_str.as_str())
    );
    assert_eq!(sender_broadcast.data.get("width").and_then(|v| v.as_f64()), Some(420.0));
    assert_eq!(sender_broadcast.data.get("height").and_then(|v| v.as_f64()), Some(260.0));

    let boards = state.boards.read().await;
    let board = boards.get(&board_id).expect("board should exist");
    let updated = board
        .objects
        .get(&target_id)
        .expect("updated sticky should remain on board");
    assert_eq!(updated.width, Some(420.0));
    assert_eq!(updated.height, Some(260.0));
    assert_eq!(updated.version, 1);
}

#[tokio::test]
async fn ai_prompt_multi_tool_single_turn_broadcasts_all_mutations_and_replies_with_text() {
    let llm: Arc<dyn LlmChat> = Arc::new(MockLlm::new(vec![
        ChatResponse {
            content: vec![
                ContentBlock::ToolUse {
                    id: "tool_1".into(),
                    name: "createStickyNote".into(),
                    input: json!({ "text": "first sticky", "x": 120, "y": 140, "color": "#FFEB3B" }),
                },
                ContentBlock::ToolUse {
                    id: "tool_2".into(),
                    name: "createStickyNote".into(),
                    input: json!({ "text": "second sticky", "x": 360, "y": 140, "color": "#8BC34A" }),
                },
            ],
            model: "mock".into(),
            stop_reason: "tool_use".into(),
            input_tokens: 30,
            output_tokens: 40,
        },
        ChatResponse {
            content: vec![ContentBlock::Text { text: "Added two stickies.".into() }],
            model: "mock".into(),
            stop_reason: "end_turn".into(),
            input_tokens: 10,
            output_tokens: 8,
        },
    ]));
    let state = test_helpers::test_app_state_with_llm(llm);
    let board_id = test_helpers::seed_board(&state).await;

    let (sender_client_id, sender_tx, mut sender_rx, _peer_client_id, mut peer_rx) =
        register_two_clients(&state, board_id).await;
    let user_id = Uuid::new_v4();
    let mut current_board = Some(board_id);

    let sender_frames = process_inbound_text(
        &state,
        &mut current_board,
        sender_client_id,
        user_id,
        &sender_tx,
        &ai_prompt_json(board_id, "create two stickies"),
    )
    .await;

    assert_eq!(sender_frames.len(), 1);
    let reply = &sender_frames[0];
    assert_eq!(reply.syscall, "ai:prompt");
    assert_eq!(reply.status, Status::Done);
    assert_eq!(reply.data.get("mutations").and_then(|v| v.as_u64()), Some(2));
    assert_eq!(reply.data.get("text").and_then(|v| v.as_str()), Some("Added two stickies."));

    let sender_broadcasts = recv_board_broadcasts(&mut sender_rx, 2).await;
    let peer_broadcasts = recv_board_broadcasts(&mut peer_rx, 2).await;
    assert!(
        sender_broadcasts
            .iter()
            .all(|f| f.syscall == "object:create")
    );
    assert!(peer_broadcasts.iter().all(|f| f.syscall == "object:create"));
    assert!(
        sender_broadcasts
            .iter()
            .all(|f| { f.data.get("kind").and_then(|v| v.as_str()) == Some("sticky_note") })
    );
    assert!(
        peer_broadcasts
            .iter()
            .all(|f| { f.data.get("kind").and_then(|v| v.as_str()) == Some("sticky_note") })
    );

    let boards = state.boards.read().await;
    let board = boards.get(&board_id).expect("board should exist");
    assert_eq!(board.objects.len(), 2);
    let texts: Vec<&str> = board
        .objects
        .values()
        .filter_map(|o| o.props.get("text").and_then(|v| v.as_str()))
        .collect();
    assert!(texts.contains(&"first sticky"));
    assert!(texts.contains(&"second sticky"));
}

#[tokio::test]
async fn ai_prompt_sequence_multi_tool_text_then_multi_tool_text() {
    let llm: Arc<dyn LlmChat> = Arc::new(MockLlm::new(vec![
        ChatResponse {
            content: vec![
                ContentBlock::ToolUse {
                    id: "tool_1".into(),
                    name: "createStickyNote".into(),
                    input: json!({ "text": "batch 1 - a", "x": 100, "y": 120, "color": "#FFEB3B" }),
                },
                ContentBlock::ToolUse {
                    id: "tool_2".into(),
                    name: "createStickyNote".into(),
                    input: json!({ "text": "batch 1 - b", "x": 300, "y": 120, "color": "#FFC107" }),
                },
            ],
            model: "mock".into(),
            stop_reason: "tool_use".into(),
            input_tokens: 28,
            output_tokens: 32,
        },
        ChatResponse {
            content: vec![ContentBlock::Text { text: "First batch complete.".into() }],
            model: "mock".into(),
            stop_reason: "end_turn".into(),
            input_tokens: 9,
            output_tokens: 7,
        },
        ChatResponse {
            content: vec![
                ContentBlock::ToolUse {
                    id: "tool_3".into(),
                    name: "createFrame".into(),
                    input: json!({ "title": "Batch 2 Frame", "x": 80, "y": 280, "width": 500, "height": 260 }),
                },
                ContentBlock::ToolUse {
                    id: "tool_4".into(),
                    name: "createStickyNote".into(),
                    input: json!({ "text": "batch 2 - note", "x": 140, "y": 340, "color": "#4CAF50" }),
                },
            ],
            model: "mock".into(),
            stop_reason: "tool_use".into(),
            input_tokens: 30,
            output_tokens: 34,
        },
        ChatResponse {
            content: vec![ContentBlock::Text { text: "Second batch complete.".into() }],
            model: "mock".into(),
            stop_reason: "end_turn".into(),
            input_tokens: 9,
            output_tokens: 7,
        },
    ]));
    let state = test_helpers::test_app_state_with_llm(llm);
    let board_id = test_helpers::seed_board(&state).await;

    let (sender_client_id, sender_tx, mut sender_rx, _peer_client_id, mut peer_rx) =
        register_two_clients(&state, board_id).await;
    let user_id = Uuid::new_v4();
    let mut current_board = Some(board_id);

    let first_reply = process_inbound_text(
        &state,
        &mut current_board,
        sender_client_id,
        user_id,
        &sender_tx,
        &ai_prompt_json(board_id, "do first batch"),
    )
    .await;

    assert_eq!(first_reply.len(), 1);
    let first = &first_reply[0];
    assert_eq!(first.status, Status::Done);
    assert_eq!(first.data.get("mutations").and_then(|v| v.as_u64()), Some(2));
    assert_eq!(first.data.get("text").and_then(|v| v.as_str()), Some("First batch complete."));
    let first_sender_broadcasts = recv_board_broadcasts(&mut sender_rx, 2).await;
    let first_peer_broadcasts = recv_board_broadcasts(&mut peer_rx, 2).await;
    assert!(
        first_sender_broadcasts
            .iter()
            .all(|f| f.syscall == "object:create")
    );
    assert!(
        first_peer_broadcasts
            .iter()
            .all(|f| f.syscall == "object:create")
    );

    let second_reply = process_inbound_text(
        &state,
        &mut current_board,
        sender_client_id,
        user_id,
        &sender_tx,
        &ai_prompt_json(board_id, "do second batch"),
    )
    .await;

    assert_eq!(second_reply.len(), 1);
    let second = &second_reply[0];
    assert_eq!(second.status, Status::Done);
    assert_eq!(second.data.get("mutations").and_then(|v| v.as_u64()), Some(2));
    assert_eq!(second.data.get("text").and_then(|v| v.as_str()), Some("Second batch complete."));
    let second_sender_broadcasts = recv_board_broadcasts(&mut sender_rx, 2).await;
    let second_peer_broadcasts = recv_board_broadcasts(&mut peer_rx, 2).await;
    assert!(
        second_sender_broadcasts
            .iter()
            .all(|f| f.syscall == "object:create")
    );
    assert!(
        second_peer_broadcasts
            .iter()
            .all(|f| f.syscall == "object:create")
    );
    assert!(
        second_sender_broadcasts
            .iter()
            .any(|f| { f.data.get("kind").and_then(|v| v.as_str()) == Some("frame") })
    );
    assert!(
        second_sender_broadcasts
            .iter()
            .any(|f| { f.data.get("kind").and_then(|v| v.as_str()) == Some("sticky_note") })
    );

    let boards = state.boards.read().await;
    let board = boards.get(&board_id).expect("board should exist");
    assert_eq!(board.objects.len(), 4);
}
