use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

use super::{BoxStream, Provider, ProviderResponse};

pub struct OpenAiProvider {
    pub endpoint: String,
    pub api_key: String,
    pub model_list: Vec<String>,
    client: Client,
}

impl OpenAiProvider {
    pub fn new(endpoint: String, api_key: String, model_list: Vec<String>) -> Self {
        Self {
            endpoint,
            api_key,
            model_list,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    async fn chat_completion(&self, request: &Value, stream: bool) -> ProviderResponse {
        let url = format!("{}/v1/chat/completions", self.endpoint.trim_end_matches('/'));

        let mut req = request.clone();
        if let Some(obj) = req.as_object_mut() {
            obj.insert("stream".to_string(), Value::Bool(stream));
        }

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await;

        match resp {
            Ok(r) if stream => {
                let byte_stream: BoxStream = Box::pin(r.bytes_stream());
                ProviderResponse::Stream(byte_stream)
            }
            Ok(r) => {
                let body = r.json::<Value>().await.unwrap_or_default();
                ProviderResponse::Json(body)
            }
            Err(e) => ProviderResponse::Json(serde_json::json!({
                "error": {"message": e.to_string(), "type": "proxy_error"}
            })),
        }
    }

    fn models(&self) -> Vec<String> {
        self.model_list.clone()
    }
}
