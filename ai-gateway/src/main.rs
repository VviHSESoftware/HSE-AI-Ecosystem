mod config;
mod handlers;
mod metrics;
mod openapi;
mod schemas;
mod state;

use axum::{routing::{get, post}, Router, middleware};
use std::sync::Arc;
use axum::extract::DefaultBodyLimit;
use tokio::net::TcpListener;
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use config::{AppEnv, DynamicConfig};
use state::GatewayService;

pub const PROJECT_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

use common_infra::{
    init_tracing, register_process_metrics,
    track_metrics, common_auth_guard, system_handlers
};

#[tokio::main]
async fn main() {
    init_tracing(PROJECT_NAME);
    let metrics_handle = register_process_metrics();

    let env = AppEnv::load();
    info!("Starting {} v{}", PROJECT_NAME, APP_VERSION);

    let service = Arc::new(GatewayService::new(env));

    if let Ok(cfg) = DynamicConfig::load_from_file() {
        service.update_config(cfg).await;
        info!("Initial configuration loaded from config.json");
    } else {
        tracing::error!("CRITICAL: Could not load initial config.json");
    }

    let service_clone = service.clone();
    tokio::spawn(async move { service_clone.warmup_loop().await; });

    let auth_routes = Router::new()
        .route("/llm", post(handlers::generation::llm))
        .route("/chat/completions", post(handlers::openai::openai_chat))
        .route("/models", get(handlers::openai::list_models))
        .route("/vlm", post(handlers::generation::vlm))
        .route("/embeddings", post(handlers::generation::embeddings))
        .route("/asr", post(handlers::audio::asr))
        .route("/image", post(handlers::image::image))
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024))
        .route("/admin/config/reload", post(handlers::admin::reload))
        .route("/admin/status/models", get(handlers::admin::status))
        .route_layer(middleware::from_fn_with_state(service.clone(), common_auth_guard));

    let app = Router::new()
        .route("/health", get(system_handlers::health))
        .route("/metrics", get(move || std::future::ready(metrics_handle.render())))
        .nest("/v1", auth_routes)
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", openapi::ApiDoc::openapi()))
        .with_state(service)
        .layer(axum::middleware::from_fn(track_metrics));

    let listener = TcpListener::bind("0.0.0.0:8000").await.unwrap();
    info!("Listening on http://0.0.0.0:8000");
    info!("Swagger UI available at http://0.0.0.0:8000/docs");

    axum::serve(listener, app).await.unwrap();
}