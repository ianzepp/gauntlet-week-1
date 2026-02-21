use super::*;

fn msg(id: &str, role: &str, content: &str, timestamp: f64) -> AiMessage {
    AiMessage { id: id.to_owned(), role: role.to_owned(), content: content.to_owned(), timestamp, mutations: None }
}

#[test]
fn upsert_ai_user_message_updates_existing_user_message_content() {
    let mut ai =
        AiState { messages: vec![msg("m1", "user", "old", 0.0), msg("m2", "assistant", "reply", 10.0)], loading: true };

    upsert_ai_user_message(&mut ai, msg("m1", "user", "new", 42.0));

    assert_eq!(ai.messages.len(), 2);
    assert_eq!(ai.messages[0].content, "new");
    assert_eq!(ai.messages[0].timestamp, 42.0);
}

#[test]
fn upsert_ai_user_message_preserves_existing_nonzero_timestamp() {
    let mut ai = AiState { messages: vec![msg("m1", "user", "old", 7.0)], loading: false };

    upsert_ai_user_message(&mut ai, msg("m1", "user", "new", 99.0));

    assert_eq!(ai.messages[0].content, "new");
    assert_eq!(ai.messages[0].timestamp, 7.0);
}

#[test]
fn upsert_ai_user_message_appends_when_id_not_found() {
    let mut ai = AiState::default();
    upsert_ai_user_message(&mut ai, msg("m1", "user", "hello", 1.0));
    assert_eq!(ai.messages.len(), 1);
    assert_eq!(ai.messages[0].id, "m1");
}
