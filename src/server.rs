use std::sync::Arc;
use std::time::Instant;

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use tower_http::trace::TraceLayer;

use crate::logging::{LogEntry, Logger};
use crate::provider::ProviderResponse;
use crate::router::Router as ModelRouter;

pub struct AppStateInner {
    pub router: ModelRouter,
    pub logger: Logger,
}

pub type AppState = Arc<AppStateInner>;

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
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let model = body
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("gpt-4o")
        .to_string();
    let stream = body.get("stream").and_then(|s| s.as_bool()).unwrap_or(false);

    let start = Instant::now();
    let provider = state.router.route_model(&model);
    let response = provider.chat_completion(&body, stream).await;
    let latency_ms = start.elapsed().as_millis() as i64;

    let (result, status, tokens_in, tokens_out) = match response {
        ProviderResponse::Json(ref v) => {
            let status = if v.get("error").is_some() { "error" } else { "ok" };
            let usage = v.get("usage");
            let t_in = usage.and_then(|u| u.get("prompt_tokens")).and_then(|t| t.as_i64()).unwrap_or(0);
            let t_out = usage.and_then(|u| u.get("completion_tokens")).and_then(|t| t.as_i64()).unwrap_or(0);
            (v.clone(), status.to_string(), t_in, t_out)
        }
        ProviderResponse::Stream(_) => {
            (json!({"error": {"message": "streaming not yet supported via JSON endpoint"}}), "error".to_string(), 0, 0)
        }
    };

    state.logger.log_request(LogEntry {
        timestamp: chrono_now(),
        source: "api".to_string(),
        model,
        tokens_in,
        tokens_out,
        latency_ms,
        status,
    });

    Json(result)
}

async fn models(State(state): State<AppState>) -> Json<Value> {
    let model_list: Vec<Value> = state
        .router
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

fn chrono_now() -> String {
    // Simple ISO-8601 timestamp without chrono dep
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{secs}")
}
