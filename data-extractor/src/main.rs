mod config;
mod moodle_client;
mod kb_client;
mod sync_job;

use axum::{routing::post, extract::State, Json, http::StatusCode, response::IntoResponse, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

#[derive(Deserialize)]
struct MoodleWebhookPayload {
    timestamp: i64,
    update_type: String,
    course_id: i32,
}

#[derive(Serialize)]
struct WebhookResponse {
    status: String,
}

async fn moodle_webhook(
    State(env): State<Arc<config::AppEnv>>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<MoodleWebhookPayload>,
) -> impl IntoResponse {
    let auth_header = headers.get("Authorization").and_then(|h| h.to_str().ok()).unwrap_or("");
    if auth_header != format!("Bearer {}", env.webhook_token) {
        return (StatusCode::UNAUTHORIZED, Json(WebhookResponse { status: "Unauthorized".into() }));
    }

    info!("Received webhook for course {} (type: {})", payload.course_id, payload.update_type);

    let env_clone = env.clone();
    tokio::spawn(async move {
        sync_job::process_webhook(env_clone, payload.course_id, payload.update_type).await;
    });

    (StatusCode::OK, Json(WebhookResponse { status: "Queued".into() }))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    let env = Arc::new(config::AppEnv::load());

    let app = Router::new()
        .route("/moodle-update", post(moodle_webhook))
        .with_state(env);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    info!("Moodle Extractor listening on http://0.0.0.0:8000");
    axum::serve(listener, app).await.unwrap();
}