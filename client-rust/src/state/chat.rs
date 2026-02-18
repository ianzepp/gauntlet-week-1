#[cfg(test)]
#[path = "chat_test.rs"]
mod chat_test;

/// State for the board chat panel.
///
/// In the full Leptos implementation, fields will be `RwSignal` types
/// provided via context. For now they are plain fields.
#[derive(Clone, Debug, Default)]
pub struct ChatState {
    pub messages: Vec<ChatMessage>,
}

/// A single chat message.
#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub id: String,
    pub user_id: String,
    pub user_name: String,
    pub user_color: String,
    pub content: String,
    pub timestamp: f64,
}
