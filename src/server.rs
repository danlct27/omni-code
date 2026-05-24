use std::sync::Arc;
use std::time::Instant;

use axum::{
    body::Body,
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
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
        .layer(axum::extract::DefaultBodyLimit::max(50 * 1024 * 1024))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn chat_completions(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Response {
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

    match response {
        ProviderResponse::Json(v) => {
            let status = if v.get("error").is_some() { "error" } else { "ok" };
            let usage = v.get("usage");
            let t_in = usage.and_then(|u| u.get("prompt_tokens")).and_then(|t| t.as_i64()).unwrap_or(0);
            let t_out = usage.and_then(|u| u.get("completion_tokens")).and_then(|t| t.as_i64()).unwrap_or(0);

            state.logger.log_request(LogEntry {
                timestamp: chrono_now(),
                source: "api".to_string(),
                model,
                tokens_in: t_in,
                tokens_out: t_out,
                latency_ms,
                status: status.to_string(),
            });

            Json(v).into_response()
        }
        ProviderResponse::Stream(byte_stream) => {
            state.logger.log_request(LogEntry {
                timestamp: chrono_now(),
                source: "api".to_string(),
                model,
                tokens_in: 0,
                tokens_out: 0,
                latency_ms,
                status: "stream".to_string(),
            });

            // Forward the raw SSE byte stream from the upstream provider
            let body = Body::from_stream(byte_stream);
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/event-stream")
                .header(header::CACHE_CONTROL, "no-cache")
                .body(body)
                .unwrap()
                .into_response()
        }
    }
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
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{secs}")
}
