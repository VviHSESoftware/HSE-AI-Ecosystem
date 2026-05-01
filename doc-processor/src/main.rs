mod config;
mod handlers;
mod openapi;
mod schemas;
mod state;

use axum::{routing::{get, post}, Router, middleware};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use config::AppEnv;
use state::DocumentService;

use common_infra::{
    init_tracing, register_process_metrics,
    track_metrics, common_auth_guard, system_handlers
};

#[tokio::main]
async fn main() {
    init_tracing("HSE Doc Processor");
    register_process_metrics();
    
    let env = AppEnv::load();
    info!("Starting {} v{}", env.project_name, env.app_version);

    let service = Arc::new(DocumentService::new(env));

    let auth_routes = Router::new()
        .route("/chunkText", post(handlers::chunking::chunk_text))
        .route("/chunkPartitionedText", post(handlers::chunking::chunk_partitioned))
        .route("/parseVideo", post(handlers::parsing::parse_video))
        .route("/parseDocument", post(handlers::parsing::parse_document))
        .route("/getSupportedDocumentTypes", get(handlers::parsing::get_supported_types))
        .route_layer(middleware::from_fn_with_state(service.clone(), common_auth_guard));

    let app = Router::new()
        .route("/health", get(system_handlers::health))
        .route("/metrics", get(system_handlers::metrics))
        .nest("/api/v1", auth_routes)
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", openapi::ApiDoc::openapi()))
        .with_state(service)
        .layer(axum::middleware::from_fn(track_metrics));

    let listener = TcpListener::bind("0.0.0.0:8000").await.unwrap();
    info!("Listening on http://0.0.0.0:8000");
    info!("Swagger UI available at http://0.0.0.0:8000/docs");

    axum::serve(listener, app).await.unwrap();
}