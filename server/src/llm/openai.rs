//! OpenAI-compatible API client.
//!
//! Ported from Prior's `kernel/src/llm/openai_client.rs`. Supports both
//! `/v1/chat/completions` and `/v1/responses` endpoints.

use serde::Serialize;
use serde_json::Value;
use std::time::Duration;

use super::types::{ChatResponse, Content, ContentBlock, LlmError, Message, Tool};

const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
const REQUEST_TIMEOUT_SECS: u64 = 120;
const CONNECT_TIMEOUT_SECS: u64 = 10;

#[derive(Debug, Clone, Copy)]
pub enum OpenAiApiMode {
    ChatCompletions,
    Responses,
}

pub struct OpenAiClient {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
    mode: OpenAiApiMode,
}

impl OpenAiClient {
    pub fn new(api_key: String, mode: Option<&str>, base_url: Option<&str>) -> Result<Self, LlmError> {
        let mode = match mode.unwrap_or("responses") {
            "responses" => OpenAiApiMode::Responses,
            "chat_completions" => OpenAiApiMode::ChatCompletions,
            other => {
                return Err(LlmError::ConfigParse(format!(
                    "unsupported openai_api mode '{other}' (expected 'responses' or 'chat_completions')"
                )));
            }
        };
        let base_url = base_url
            .unwrap_or(DEFAULT_OPENAI_BASE_URL)
            .trim_end_matches('/')
            .to_string();
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
            .build()
            .map_err(|e| LlmError::HttpClientBuild(e.to_string()))?;
        Ok(Self { http, api_key, base_url, mode })
    }

    pub async fn chat(
        &self,
        model: &str,
        max_tokens: u32,
        system: &str,
        messages: &[Message],
        tools: Option<&[Tool]>,
    ) -> Result<ChatResponse, LlmError> {
        match self.mode {
            OpenAiApiMode::ChatCompletions => {
                self.chat_completions(model, max_tokens, system, messages, tools)
                    .await
            }
            OpenAiApiMode::Responses => {
                self.responses(model, max_tokens, system, messages, tools)
                    .await
            }
        }
    }

    async fn chat_completions(
        &self,
        model: &str,
        max_tokens: u32,
        system: &str,
        messages: &[Message],
        tools: Option<&[Tool]>,
    ) -> Result<ChatResponse, LlmError> {
        let msgs = build_chat_completions_messages(system, messages);
        let tool_defs: Option<Vec<CcToolDef<'_>>> = tools.map(|t| t.iter().map(CcToolDef::from).collect());
        let body = CcRequest { model, max_tokens, messages: &msgs, tools: tool_defs.as_deref() };
        let text = self.send_json("/chat/completions", &body).await?;
        parse_chat_completions_response(&text)
    }

    async fn responses(
        &self,
        model: &str,
        max_tokens: u32,
        system: &str,
        messages: &[Message],
        tools: Option<&[Tool]>,
    ) -> Result<ChatResponse, LlmError> {
        let input = build_responses_input(messages);
        let tool_defs: Option<Vec<RespToolDef<'_>>> = tools.map(|t| t.iter().map(RespToolDef::from).collect());
        let body = RespRequest {
            model,
            max_output_tokens: max_tokens,
            instructions: system,
            input: &input,
            tools: tool_defs.as_deref(),
        };
        let text = self.send_json("/responses", &body).await?;
        parse_responses_response(&text)
    }

    async fn send_json(&self, path: &str, body: &impl Serialize) -> Result<String, LlmError> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.api_key)
            .json(body)
            .send()
            .await
            .map_err(|e| LlmError::ApiRequest(e.to_string()))?;

        let status = response.status().as_u16();
        let text = response
            .text()
            .await
            .map_err(|e| LlmError::ApiRequest(e.to_string()))?;
        if status != 200 {
            return Err(LlmError::ApiResponse { status, body: text });
        }
        Ok(text)
    }
}

// =============================================================================
// CHAT COMPLETIONS — wire types
// =============================================================================

#[derive(Serialize)]
struct CcRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    messages: &'a [CcMessage],
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<&'a [CcToolDef<'a>]>,
}

#[derive(Serialize)]
struct CcMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<CcToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Serialize)]
struct CcToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: &'static str,
    function: CcFunctionCall,
}

#[derive(Serialize)]
struct CcFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Serialize)]
struct CcToolDef<'a> {
    #[serde(rename = "type")]
    tool_type: &'static str,
    function: CcFunctionDef<'a>,
}

#[derive(Serialize)]
struct CcFunctionDef<'a> {
    name: &'a str,
    description: &'a str,
    parameters: &'a Value,
}

impl<'a> From<&'a Tool> for CcToolDef<'a> {
    fn from(tool: &'a Tool) -> Self {
        Self {
            tool_type: "function",
            function: CcFunctionDef {
                name: &tool.name,
                description: &tool.description,
                parameters: &tool.input_schema,
            },
        }
    }
}

fn build_chat_completions_messages(system: &str, messages: &[Message]) -> Vec<CcMessage> {
    let mut out = Vec::new();
    if !system.trim().is_empty() {
        out.push(CcMessage {
            role: "system".to_string(),
            content: Some(system.to_string()),
            tool_calls: None,
            tool_call_id: None,
        });
    }
    for message in messages {
        match &message.content {
            Content::Text(text) => {
                out.push(CcMessage {
                    role: message.role.clone(),
                    content: Some(text.clone()),
                    tool_calls: None,
                    tool_call_id: None,
                });
            }
            Content::Blocks(blocks) => {
                let mut text = String::new();
                let mut tool_calls = Vec::new();
                let mut tool_results = Vec::new();
                for block in blocks {
                    match block {
                        ContentBlock::Text { text: t } => text.push_str(t),
                        ContentBlock::ToolUse { id, name, input } => {
                            tool_calls.push(CcToolCall {
                                id: id.clone(),
                                call_type: "function",
                                function: CcFunctionCall {
                                    name: name.clone(),
                                    arguments: serde_json::to_string(input).unwrap_or_else(|_| "{}".to_string()),
                                },
                            });
                        }
                        ContentBlock::ToolResult { tool_use_id, content, is_error: _ } => {
                            tool_results.push(CcMessage {
                                role: "tool".to_string(),
                                content: Some(content.clone()),
                                tool_calls: None,
                                tool_call_id: Some(tool_use_id.clone()),
                            });
                        }
                        ContentBlock::Thinking { .. } | ContentBlock::Unknown => {}
                    }
                }
                if !text.is_empty() || !tool_calls.is_empty() {
                    out.push(CcMessage {
                        role: message.role.clone(),
                        content: if text.is_empty() { None } else { Some(text) },
                        tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls) },
                        tool_call_id: None,
                    });
                }
                out.extend(tool_results);
            }
        }
    }
    out
}

// =============================================================================
// RESPONSES — wire types
// =============================================================================

#[derive(Serialize)]
struct RespRequest<'a> {
    model: &'a str,
    max_output_tokens: u32,
    instructions: &'a str,
    input: &'a [RespInputItem],
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<&'a [RespToolDef<'a>]>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum RespInputItem {
    #[serde(rename = "message")]
    Message {
        role: String,
        content: Vec<RespTextContent>,
    },

    #[serde(rename = "function_call")]
    FunctionCall {
        call_id: String,
        name: String,
        arguments: String,
    },

    #[serde(rename = "function_call_output")]
    FunctionCallOutput { call_id: String, output: String },
}

#[derive(Serialize)]
struct RespTextContent {
    #[serde(rename = "type")]
    content_type: &'static str,
    text: String,
}

impl RespTextContent {
    fn input_text(text: String) -> Self {
        Self { content_type: "input_text", text }
    }
}

#[derive(Serialize)]
struct RespToolDef<'a> {
    #[serde(rename = "type")]
    tool_type: &'static str,
    name: &'a str,
    description: &'a str,
    parameters: &'a Value,
}

impl<'a> From<&'a Tool> for RespToolDef<'a> {
    fn from(tool: &'a Tool) -> Self {
        Self { tool_type: "function", name: &tool.name, description: &tool.description, parameters: &tool.input_schema }
    }
}

fn build_responses_input(messages: &[Message]) -> Vec<RespInputItem> {
    let mut out = Vec::new();
    for message in messages {
        match &message.content {
            Content::Text(text) => {
                out.push(RespInputItem::Message {
                    role: message.role.clone(),
                    content: vec![RespTextContent::input_text(text.clone())],
                });
            }
            Content::Blocks(blocks) => {
                let mut text = String::new();
                for block in blocks {
                    match block {
                        ContentBlock::Text { text: t } => text.push_str(t),
                        ContentBlock::ToolUse { id, name, input } => {
                            out.push(RespInputItem::FunctionCall {
                                call_id: id.clone(),
                                name: name.clone(),
                                arguments: serde_json::to_string(input).unwrap_or_else(|_| "{}".to_string()),
                            });
                        }
                        ContentBlock::ToolResult { tool_use_id, content, is_error: _ } => {
                            out.push(RespInputItem::FunctionCallOutput {
                                call_id: tool_use_id.clone(),
                                output: content.clone(),
                            });
                        }
                        ContentBlock::Thinking { .. } | ContentBlock::Unknown => {}
                    }
                }
                if !text.is_empty() {
                    out.push(RespInputItem::Message {
                        role: message.role.clone(),
                        content: vec![RespTextContent::input_text(text)],
                    });
                }
            }
        }
    }
    out
}

// =============================================================================
// RESPONSE PARSING
// =============================================================================

pub(crate) fn parse_chat_completions_response(json_text: &str) -> Result<ChatResponse, LlmError> {
    let root: Value = serde_json::from_str(json_text).map_err(|e| LlmError::ApiParse(e.to_string()))?;
    let model = root
        .get("model")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_default();
    let prompt_tokens = root
        .get("usage")
        .and_then(|u| u.get("prompt_tokens"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let completion_tokens = root
        .get("usage")
        .and_then(|u| u.get("completion_tokens"))
        .and_then(Value::as_u64)
        .unwrap_or(0);

    let Some(choice) = root
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|arr| arr.first())
    else {
        return Err(LlmError::ApiParse("chat_completions: missing choices[0]".to_string()));
    };
    let finish_reason = choice
        .get("finish_reason")
        .and_then(Value::as_str)
        .unwrap_or("stop");
    let message = choice.get("message").cloned().unwrap_or(Value::Null);

    let mut content = Vec::new();
    if let Some(text) = message.get("content").and_then(Value::as_str) {
        if !text.is_empty() {
            content.push(ContentBlock::Text { text: text.to_string() });
        }
    }

    if let Some(tool_calls) = message.get("tool_calls").and_then(Value::as_array) {
        for call in tool_calls {
            let Some(name) = call
                .get("function")
                .and_then(|f| f.get("name"))
                .and_then(Value::as_str)
            else {
                continue;
            };
            let Some(id) = call.get("id").and_then(Value::as_str).map(str::to_owned) else {
                return Err(LlmError::ApiParse("chat_completions: tool call missing id".to_string()));
            };
            let args = call
                .get("function")
                .and_then(|f| f.get("arguments"))
                .and_then(Value::as_str)
                .unwrap_or("{}");
            let input =
                serde_json::from_str::<Value>(args).unwrap_or_else(|_| Value::Object(serde_json::Map::default()));
            content.push(ContentBlock::ToolUse { id, name: name.to_string(), input });
        }
    }

    let stop_reason = if content
        .iter()
        .any(|block| matches!(block, ContentBlock::ToolUse { .. }))
    {
        "tool_use".to_string()
    } else if finish_reason == "length" {
        "max_tokens".to_string()
    } else {
        "end_turn".to_string()
    };

    Ok(ChatResponse { content, model, stop_reason, input_tokens: prompt_tokens, output_tokens: completion_tokens })
}

pub(crate) fn parse_responses_response(json_text: &str) -> Result<ChatResponse, LlmError> {
    let root: Value = serde_json::from_str(json_text).map_err(|e| LlmError::ApiParse(e.to_string()))?;
    let model = root
        .get("model")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_default();
    let input_tokens = root
        .get("usage")
        .and_then(|u| u.get("input_tokens"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let output_tokens = root
        .get("usage")
        .and_then(|u| u.get("output_tokens"))
        .and_then(Value::as_u64)
        .unwrap_or(0);

    let mut content = Vec::new();
    if let Some(items) = root.get("output").and_then(Value::as_array) {
        for item in items {
            match item.get("type").and_then(Value::as_str) {
                Some("message") => {
                    let Some(parts) = item.get("content").and_then(Value::as_array) else {
                        continue;
                    };
                    for part in parts {
                        let kind = part.get("type").and_then(Value::as_str);
                        let text = part
                            .get("text")
                            .or_else(|| part.get("output_text"))
                            .and_then(Value::as_str)
                            .unwrap_or("");
                        if matches!(kind, Some("output_text" | "text")) && !text.is_empty() {
                            content.push(ContentBlock::Text { text: text.to_string() });
                        }
                    }
                }
                Some("function_call") => {
                    let Some(id) = item
                        .get("call_id")
                        .or_else(|| item.get("id"))
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                    else {
                        return Err(LlmError::ApiParse("responses: function_call missing call_id".to_string()));
                    };
                    let Some(name) = item.get("name").and_then(Value::as_str) else {
                        continue;
                    };
                    let args = item
                        .get("arguments")
                        .and_then(Value::as_str)
                        .unwrap_or("{}");
                    let input = serde_json::from_str::<Value>(args)
                        .unwrap_or_else(|_| Value::Object(serde_json::Map::default()));
                    content.push(ContentBlock::ToolUse { id, name: name.to_string(), input });
                }
                _ => {}
            }
        }
    } else if let Some(output_text) = root.get("output_text").and_then(Value::as_str) {
        if !output_text.is_empty() {
            content.push(ContentBlock::Text { text: output_text.to_string() });
        }
    }

    let stop_reason = if content
        .iter()
        .any(|block| matches!(block, ContentBlock::ToolUse { .. }))
    {
        "tool_use".to_string()
    } else if root
        .get("incomplete_details")
        .and_then(|d| d.get("reason"))
        .and_then(Value::as_str)
        == Some("max_output_tokens")
    {
        "max_tokens".to_string()
    } else {
        "end_turn".to_string()
    };

    Ok(ChatResponse { content, model, stop_reason, input_tokens, output_tokens })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== chat completions =====

    #[test]
    fn cc_parse_text_response() {
        let json = serde_json::json!({
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "Hello!" },
                "finish_reason": "stop"
            }],
            "usage": { "prompt_tokens": 10, "completion_tokens": 5 }
        })
        .to_string();
        let resp = parse_chat_completions_response(&json).unwrap();
        assert_eq!(resp.content.len(), 1);
        assert!(matches!(&resp.content[0], ContentBlock::Text { text } if text == "Hello!"));
        assert_eq!(resp.stop_reason, "end_turn");
        assert_eq!(resp.input_tokens, 10);
    }

    #[test]
    fn cc_parse_tool_call() {
        let json = serde_json::json!({
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": { "name": "create_objects", "arguments": "{\"objects\":[]}" }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": { "prompt_tokens": 20, "completion_tokens": 10 }
        })
        .to_string();
        let resp = parse_chat_completions_response(&json).unwrap();
        assert_eq!(resp.stop_reason, "tool_use");
        assert!(matches!(&resp.content[0], ContentBlock::ToolUse { name, .. } if name == "create_objects"));
    }

    #[test]
    fn cc_parse_missing_choices() {
        let json = serde_json::json!({ "model": "gpt-4o", "choices": [] }).to_string();
        assert!(parse_chat_completions_response(&json).is_err());
    }

    // ===== responses API =====

    #[test]
    fn resp_parse_text_response() {
        let json = serde_json::json!({
            "model": "gpt-4o",
            "output": [{
                "type": "message",
                "content": [{ "type": "output_text", "text": "Done!" }]
            }],
            "usage": { "input_tokens": 15, "output_tokens": 8 }
        })
        .to_string();
        let resp = parse_responses_response(&json).unwrap();
        assert_eq!(resp.content.len(), 1);
        assert!(matches!(&resp.content[0], ContentBlock::Text { text } if text == "Done!"));
        assert_eq!(resp.stop_reason, "end_turn");
    }

    #[test]
    fn resp_parse_function_call() {
        let json = serde_json::json!({
            "model": "gpt-4o",
            "output": [{
                "type": "function_call",
                "call_id": "fc_1",
                "name": "move_objects",
                "arguments": "{\"moves\":[]}"
            }],
            "usage": { "input_tokens": 10, "output_tokens": 5 }
        })
        .to_string();
        let resp = parse_responses_response(&json).unwrap();
        assert_eq!(resp.stop_reason, "tool_use");
        assert!(
            matches!(&resp.content[0], ContentBlock::ToolUse { id, name, .. } if id == "fc_1" && name == "move_objects")
        );
    }

    #[test]
    fn resp_parse_output_text_fallback() {
        let json = serde_json::json!({
            "model": "gpt-4o",
            "output_text": "Fallback text",
            "usage": { "input_tokens": 5, "output_tokens": 3 }
        })
        .to_string();
        let resp = parse_responses_response(&json).unwrap();
        assert_eq!(resp.content.len(), 1);
        assert!(matches!(&resp.content[0], ContentBlock::Text { text } if text == "Fallback text"));
    }
}
