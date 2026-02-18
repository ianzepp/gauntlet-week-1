#[cfg(test)]
#[path = "ai_test.rs"]
mod ai_test;

/// State for the AI assistant panel.
///
/// In the full Leptos implementation, fields will be `RwSignal` types
/// provided via context. For now they are plain fields.
#[derive(Clone, Debug, Default)]
pub struct AiState {
    pub messages: Vec<AiMessage>,
    pub loading: bool,
}

/// A single AI conversation message.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AiMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub timestamp: f64,
    #[serde(default)]
    pub mutations: Option<i64>,
}
