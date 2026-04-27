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

use config::AppEnv;
use state::DocumentService;

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

    let service = Arc::new(DocumentService::new(env));

    let auth_routes = Router::new()
        .route("/chunkText", post(handlers::chunking::chunk_text))
        .route("/chunkPartitionedText", post(handlers::chunking::chunk_partitioned))
        .route("/parseVideo", post(handlers::parsing::parse_video))
        .route("/parseDocument", post(handlers::parsing::parse_document))
        .route("/getSupportedDocumentTypes", get(handlers::parsing::get_supported_types))
        .route_layer(middleware::from_fn_with_state(service.clone(), middlewares::auth_guard));

    let app = Router::new()
        .route("/health", get(handlers::system::health))
        .route("/metrics", get(handlers::system::metrics))
        .nest("/api/v1", auth_routes)
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", openapi::ApiDoc::openapi()))
        .with_state(service)
        .layer(axum::middleware::from_fn(middlewares::track_metrics));

    let listener = TcpListener::bind("0.0.0.0:8000").await.unwrap();
    info!("Listening on http://0.0.0.0:8000");
    info!("Swagger UI available at http://0.0.0.0:8000/docs");

    axum::serve(listener, app).await.unwrap();
}