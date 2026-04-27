mod config;
mod error;
mod handlers;
mod metrics;
mod schemas;
mod middlewares;
mod state;
mod openapi;
mod engine;

use axum::{routing::post, Router, middleware};
use std::sync::Arc;
use axum::routing::get;
use tracing::info;
use tracing_subscriber::EnvFilter;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use crate::state::RouterState;
use crate::config::AppEnv;

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

    let service = Arc::new(RouterState::new(env));

    let app = Router::new()
        .route("/health", get(handlers::system::health))
        .route("/metrics", get(handlers::system::metrics))
        .route("/api/v1/chat", post(handlers::chat::process_request))
        .route_layer(middleware::from_fn_with_state(service.clone(), middlewares::auth_guard))
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", openapi::ApiDoc::openapi()))
        .with_state(service)
        .layer(middleware::from_fn(middlewares::track_metrics));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    tracing::info!("Chat Orchestrator listening on 0.0.0.0:8000");
    axum::serve(listener, app).await.unwrap();
}