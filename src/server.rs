use std::sync::Arc;

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use tower_http::trace::TraceLayer;

use crate::provider::ProviderResponse;
use crate::router::Router as ModelRouter;

pub type AppState = Arc<ModelRouter>;

/// Build the Axum router with all routes.
pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/models", get(models))
        .route("/health", get(health))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn chat_completions(
    State(router): State<AppState>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let model = body
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("gpt-4o");
    let stream = body.get("stream").and_then(|s| s.as_bool()).unwrap_or(false);

    let provider = router.route_model(model);
    let response = provider.chat_completion(&body, stream).await;

    match response {
        ProviderResponse::Json(v) => Json(v),
        ProviderResponse::Stream(_) => {
            // TODO: proper SSE streaming response
            Json(json!({"error": {"message": "streaming not yet supported via JSON endpoint"}}))
        }
    }
}

async fn models(State(router): State<AppState>) -> Json<Value> {
    let model_list: Vec<Value> = router
        .all_models()
        .into_iter()
        .map(|id| json!({"id": id, "object": "model"}))
        .collect();

    Json(json!({
        "object": "list",
        "data": model_list
    }))
}

async fn health() -> Json<Value> {
    Json(json!({"status": "ok"}))
}
