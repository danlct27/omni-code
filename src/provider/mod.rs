pub mod anthropic;
pub mod openai;

use async_trait::async_trait;
use bytes::Bytes;
use futures_core::Stream;
use serde_json::Value;
use std::pin::Pin;

pub type BoxStream = Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>;

pub enum ProviderResponse {
    Json(Value),
    Stream(BoxStream),
}

#[async_trait]
pub trait Provider: Send + Sync {
    async fn chat_completion(&self, request: &Value, stream: bool) -> ProviderResponse;
    fn models(&self) -> Vec<String>;
}
