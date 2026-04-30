mod config;
mod error;
mod handlers;
mod metrics;
mod openapi;
mod schemas;
mod middlewares;
mod state;

use axum::{routing::{get, post}, Router, middleware};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use config::{AppEnv, DynamicConfig};
use state::GatewayService;

fn register_process_metrics() {
    #[cfg(target_os = "linux")]
    {
        let process_collector = prometheus::process_collector::ProcessCollector::for_self();
        prometheus::register(Box::new(process_collector)).ok();
    }
}

#[tokio::main]
async fn main() {
    register_process_metrics();

    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    let env = AppEnv::load();
    info!("Starting {} v{}", env.project_name, env.app_version);

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
        .route("/vlm", post(handlers::generation::vlm))
        .route("/embeddings", post(handlers::generation::embeddings))
        .route("/asr", post(handlers::audio::asr))
        .route("/admin/config/reload", post(handlers::admin::reload))
        .route("/admin/status/models", get(handlers::admin::status))
        .route_layer(middleware::from_fn_with_state(service.clone(), middlewares::auth_guard));

    let app = Router::new()
        .route("/health", get(handlers::system::health))
        .route("/metrics", get(handlers::system::metrics))
        .nest("/v1", auth_routes)
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", openapi::ApiDoc::openapi()))
        .with_state(service)
        .layer(axum::middleware::from_fn(middlewares::track_metrics));

    let listener = TcpListener::bind("0.0.0.0:8000").await.unwrap();
    info!("Listening on http://0.0.0.0:8000");
    info!("Swagger UI available at http://0.0.0.0:8000/docs");

    axum::serve(listener, app).await.unwrap();
}