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

        // Copy model
        if let Some(model) = request.get("model") {
            obj.insert("model".to_string(), model.clone());
        }

        // Handle system prompt: Anthropic has top-level "system", OpenAI uses a system message
        let mut messages: Vec<Value> = Vec::new();
        if let Some(system) = request.get("system").and_then(|s| s.as_str()) {
            messages.push(json!({"role": "system", "content": system}));
        }

        // Translate messages
        if let Some(msgs) = request.get("messages").and_then(|m| m.as_array()) {
            for msg in msgs {
                let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("user");
                // Anthropic content can be string or array of content blocks
                let content = match msg.get("content") {
                    Some(Value::String(s)) => Value::String(s.clone()),
                    Some(Value::Array(blocks)) => {
                        // Extract text from content blocks
                        let text: String = blocks
                            .iter()
                            .filter_map(|b| {
                                if b.get("type").and_then(|t| t.as_str()) == Some("text") {
                                    b.get("text").and_then(|t| t.as_str()).map(|s| s.to_string())
                                } else {
                                    None // TODO: handle tool_use blocks
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("");
                        Value::String(text)
                    }
                    _ => Value::String(String::new()),
                };
                messages.push(json!({"role": role, "content": content}));
            }
        }

        obj.insert("messages".to_string(), Value::Array(messages));

        // Copy max_tokens → max_tokens
        if let Some(max) = request.get("max_tokens") {
            obj.insert("max_tokens".to_string(), max.clone());
        }

        // Copy temperature
        if let Some(temp) = request.get("temperature") {
            obj.insert("temperature".to_string(), temp.clone());
        }

        openai_req
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    async fn chat_completion(&self, request: &Value, stream: bool) -> ProviderResponse {
        let translated = Self::translate_request(request);
        self.backend.chat_completion(&translated, stream).await
    }

    fn models(&self) -> Vec<String> {
        self.backend.models()
    }
}
