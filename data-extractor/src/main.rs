mod config;
mod moodle_client;
mod kb_client;
mod sync_job;

use axum::{routing::post, extract::State, Json, http::StatusCode, response::IntoResponse, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use utoipa::{
    OpenApi, ToSchema,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};
use utoipa_swagger_ui::SwaggerUi;

pub const PROJECT_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Deserialize, ToSchema)]
pub struct MoodleWebhookPayload {
    #[schema(example = 1725400000)]
    pub timestamp: i64,
    #[schema(example = "course_updated")]
    pub update_type: String,
    #[schema(example = 42)]
    pub course_id: i32,
}

#[derive(Serialize, ToSchema)]
pub struct WebhookResponse {
    #[schema(example = "Queued")]
    pub status: String,
}

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearerAuth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("Token")
                        .build(),
                ),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "data-extractor",
        version = "1.0.0",
        description = "Moodle Data Extractor Webhook Service"
    ),
    paths(
        moodle_webhook
    ),
    components(
        schemas(MoodleWebhookPayload, WebhookResponse)
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

#[utoipa::path(
    post,
    path = "/moodle-update",
    request_body = MoodleWebhookPayload,
    responses(
        (status = 200, description = "Webhook accepted and task queued", body = WebhookResponse),
        (status = 401, description = "Unauthorized", body = WebhookResponse)
    ),
    security(("bearerAuth" = []))
)]
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
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .with_state(env);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    info!("Moodle Extractor listening on http://0.0.0.0:8000");
    info!("Swagger UI available at http://0.0.0.0:8000/docs");
    axum::serve(listener, app).await.unwrap();
}