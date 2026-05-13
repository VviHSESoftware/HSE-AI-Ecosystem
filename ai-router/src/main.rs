mod config;
mod handlers;
mod schemas;
mod state;
mod openapi;
mod engine;

use axum::{routing::post, Router, middleware};
use std::sync::Arc;
use axum::routing::get;
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use crate::state::RouterState;
use crate::config::AppEnv;

use common_infra::{
    init_tracing, register_process_metrics,
    track_metrics, common_auth_guard, system_handlers
};

pub const PROJECT_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
    init_tracing(PROJECT_NAME);
    let metrics_handle = register_process_metrics();

    let env = AppEnv::load();
    info!("Starting {} v{}", PROJECT_NAME, APP_VERSION);

    let service = Arc::new(RouterState::new(env));

    let app = Router::new()
        .route("/health", get(system_handlers::health))
        .route("/metrics", get(move || std::future::ready(metrics_handle.render())))
        .route("/v1/process", post(handlers::chat::process_request))
        .route_layer(middleware::from_fn_with_state(service.clone(), common_auth_guard))
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", openapi::ApiDoc::openapi()))
        .with_state(service)
        .layer(middleware::from_fn(track_metrics));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    info!("Chat Orchestrator listening on 0.0.0.0:8000");
    axum::serve(listener, app).await.unwrap();
}