use async_trait::async_trait;
use serde_json::{json, Value};

use super::{Provider, ProviderResponse};
use crate::provider::openai::OpenAiProvider;

/// Translates Anthropic API format to OpenAI format, then delegates to an OpenAI-compatible backend.
pub struct AnthropicProvider {
    backend: OpenAiProvider,
}

impl AnthropicProvider {
    pub fn new(endpoint: String, api_key: String, model_list: Vec<String>) -> Self {
        Self {
            backend: OpenAiProvider::new(endpoint, api_key, model_list),
        }
    }

    /// Convert Anthropic messages format to OpenAI messages format.
    fn translate_request(request: &Value) -> Value {
        let mut openai_req = json!({});
        let obj = openai_req.as_object_mut().unwrap();

        if let Some(model) = request.get("model") {
            obj.insert("model".to_string(), model.clone());
        }

        let mut messages: Vec<Value> = Vec::new();
        if let Some(system) = request.get("system").and_then(|s| s.as_str()) {
            messages.push(json!({"role": "system", "content": system}));
        }

        if let Some(msgs) = request.get("messages").and_then(|m| m.as_array()) {
            for msg in msgs {
                let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("user");
                match msg.get("content") {
                    Some(Value::String(s)) => {
                        messages.push(json!({"role": role, "content": s}));
                    }
                    Some(Value::Array(blocks)) => {
                        let mut text_parts: Vec<String> = Vec::new();
                        let mut tool_calls: Vec<Value> = Vec::new();

                        for block in blocks {
                            match block.get("type").and_then(|t| t.as_str()) {
                                Some("text") => {
                                    if let Some(t) = block.get("text").and_then(|t| t.as_str()) {
                                        text_parts.push(t.to_string());
                                    }
                                }
                                Some("tool_use") => {
                                    let id = block.get("id").and_then(|v| v.as_str()).unwrap_or("");
                                    let name = block.get("name").and_then(|v| v.as_str()).unwrap_or("");
                                    let input = block.get("input").unwrap_or(&Value::Null);
                                    tool_calls.push(json!({
                                        "id": id,
                                        "type": "function",
                                        "function": {
                                            "name": name,
                                            "arguments": serde_json::to_string(input).unwrap_or_default()
                                        }
                                    }));
                                }
                                Some("tool_result") => {
                                    // Anthropic tool_result → OpenAI tool message
                                    let tool_call_id = block.get("tool_use_id").and_then(|v| v.as_str()).unwrap_or("");
                                    let content = block.get("content").and_then(|v| v.as_str()).unwrap_or("");
                                    messages.push(json!({
                                        "role": "tool",
                                        "tool_call_id": tool_call_id,
                                        "content": content
                                    }));
                                }
                                _ => {}
                            }
                        }

                        if !tool_calls.is_empty() {
                            let mut m = json!({"role": role});
                            if !text_parts.is_empty() {
                                m["content"] = Value::String(text_parts.join(""));
                            }
                            m["tool_calls"] = Value::Array(tool_calls);
                            messages.push(m);
                        } else if !text_parts.is_empty() {
                            messages.push(json!({"role": role, "content": text_parts.join("")}));
                        }
                    }
                    _ => {
                        messages.push(json!({"role": role, "content": ""}));
                    }
                }
            }
        }

        obj.insert("messages".to_string(), Value::Array(messages));

        if let Some(max) = request.get("max_tokens") {
            obj.insert("max_tokens".to_string(), max.clone());
        }
        if let Some(temp) = request.get("temperature") {
            obj.insert("temperature".to_string(), temp.clone());
        }
        // Forward tools if present
        if let Some(tools) = request.get("tools") {
            obj.insert("tools".to_string(), tools.clone());
        }

        openai_req
    }

    /// Convert OpenAI response tool_calls back to Anthropic tool_use content blocks.
    fn translate_response(response: &Value) -> Value {
        let mut resp = response.clone();
        if let Some(choices) = resp.get_mut("choices").and_then(|c| c.as_array_mut()) {
            for choice in choices {
                if let Some(msg) = choice.get_mut("message") {
                    if let Some(tool_calls) = msg.get("tool_calls").and_then(|t| t.as_array()) {
                        let mut content_blocks: Vec<Value> = Vec::new();

                        // Add text content if present
                        if let Some(text) = msg.get("content").and_then(|c| c.as_str()) {
                            if !text.is_empty() {
                                content_blocks.push(json!({"type": "text", "text": text}));
                            }
                        }

                        // Convert tool_calls → tool_use blocks
                        for tc in tool_calls {
                            let id = tc.get("id").and_then(|v| v.as_str()).unwrap_or("");
                            let func = tc.get("function").unwrap_or(&Value::Null);
                            let name = func.get("name").and_then(|v| v.as_str()).unwrap_or("");
                            let args_str = func.get("arguments").and_then(|v| v.as_str()).unwrap_or("{}");
                            let input: Value = serde_json::from_str(args_str).unwrap_or(Value::Null);
                            content_blocks.push(json!({
                                "type": "tool_use",
                                "id": id,
                                "name": name,
                                "input": input
                            }));
                        }

                        // Store as anthropic-style content blocks in a custom field
                        if let Some(obj) = msg.as_object_mut() {
                            obj.insert("anthropic_content".to_string(), Value::Array(content_blocks));
                        }
                    }
                }
            }
        }
        resp
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    async fn chat_completion(&self, request: &Value, stream: bool) -> ProviderResponse {
        let translated = Self::translate_request(request);
        let response = self.backend.chat_completion(&translated, stream).await;
        match response {
            ProviderResponse::Json(v) => ProviderResponse::Json(Self::translate_response(&v)),
            other => other,
        }
    }

    fn models(&self) -> Vec<String> {
        self.backend.models()
    }
}
