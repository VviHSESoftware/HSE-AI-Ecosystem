mod config;
mod db;
mod error;
mod handlers;
mod metrics;
mod middlewares;
mod openapi;
mod schemas;
mod state;

use axum::{routing::{get, post}, Router, middleware};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use config::AppEnv;
use state::InterfaceState;

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

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&env.database_url)
        .await
        .expect("Failed to connect to Postgres");

    db::init_db(&pool).await;

    let service = Arc::new(InterfaceState::new(env, pool));

    let query_routes = Router::new()
        .route("/semanticQuery", post(handlers::query::semantic_query))
        .route("/timedVideoQuery", post(handlers::query::timed_video_query))
        .route("/documentPageQuery", post(handlers::query::document_page_query))
        .route("/fullContentQuery", post(handlers::query::full_content_query))
        .route("/availableStructure", post(handlers::query::available_structure))
        .route_layer(middleware::from_fn_with_state(service.clone(), middlewares::auth_guard));

    let app = Router::new()
        .route("/health", get(handlers::system::health))
        .route("/metrics", get(handlers::system::metrics))
        .nest("/api/v1", query_routes)
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", openapi::ApiDoc::openapi()))
        .with_state(service)
        .layer(middleware::from_fn(middlewares::track_metrics));

    let listener = TcpListener::bind("0.0.0.0:8000").await.unwrap();
    info!("Listening on http://0.0.0.0:8000");
    info!("Swagger UI available at http://0.0.0.0:8000/docs");

    axum::serve(listener, app).await.unwrap();
}