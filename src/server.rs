use axum::{routing::{get, post}, Json, Router};
use serde_json::{json, Value};
use tower_http::trace::TraceLayer;

/// Build the Axum router with all routes.
pub fn app() -> Router {
    Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/models", get(models))
        .route("/health", get(health))
        .layer(TraceLayer::new_for_http())
}

async fn chat_completions() -> Json<Value> {
    Json(json!({
        "id": "chatcmpl-mock",
        "object": "chat.completion",
        "created": 1234567890u64,
        "model": "mock",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "OmniCode proxy is working!"},
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
    }))
}

async fn models() -> Json<Value> {
    Json(json!({
        "object": "list",
        "data": [
            {"id": "gpt-4o", "object": "model"},
            {"id": "claude-sonnet", "object": "model"}
        ]
    }))
}

async fn health() -> Json<Value> {
    Json(json!({"status": "ok"}))
}
