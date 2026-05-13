mod config;
mod handlers;
mod services;
mod middlewares;
mod openapi;
mod schemas;
mod state;

use axum::{routing::{get, post}, Router, middleware};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use config::AppEnv;
use state::KbControllerState;

use common_infra::{
    init_tracing, register_process_metrics,
    track_metrics, common_auth_guard, system_handlers
};
use kb_common::repository::KbRepository;

pub const PROJECT_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
    init_tracing(PROJECT_NAME);
    let metrics_handle = register_process_metrics();

    let env = AppEnv::load();
    info!("Starting {} v{}", PROJECT_NAME, APP_VERSION);

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&env.database_url)
        .await
        .expect("Failed to connect to Postgres");

    let repo = KbRepository::new(pool);
    repo.init_schema().await.expect("Failed to initialize database schema");
    info!("Database schema initialized successfully");

    let service = Arc::new(KbControllerState::new(env, repo).await);

    let admin_routes = Router::new()
        .route("/createSystem", post(handlers::admin::create_system))
        .route_layer(middleware::from_fn_with_state(service.clone(), common_auth_guard));

    let system_routes  = Router::new()
        .route("/addModuleBase", post(handlers::modules::add_module_base))
        .route("/addModuleAccess", post(handlers::modules::add_module_access))
        .route("/setVideoType", post(handlers::modules::set_video_type))
        .route("/setDocumentType", post(handlers::modules::set_document_type))
        .route("/setTextType", post(handlers::modules::set_text_type))
        .route("/getModuleId", post(handlers::modules::get_module_id))
        .route("/getSystemId", post(handlers::modules::get_system_id))
        .route("/invalidateModule", post(handlers::modules::invalidate_module))
        .route("/getSupportedFileTypes", get(handlers::modules::get_supported_file_types))
        .route_layer(middleware::from_fn_with_state(service.clone(), middlewares::system_auth_guard));

    let app = Router::new()
        .route("/health", get(system_handlers::health))
        .route("/metrics", get(move || std::future::ready(metrics_handle.render())))
        .nest("/v1/admin", admin_routes)
        .nest("/v1", system_routes)
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", openapi::ApiDoc::openapi()))
        .with_state(service)
        .layer(middleware::from_fn(track_metrics));

    let listener = TcpListener::bind("0.0.0.0:8000").await.unwrap();
    info!("Listening on http://0.0.0.0:8000");
    info!("Swagger UI available at http://0.0.0.0:8000/docs");

    axum::serve(listener, app).await.unwrap();
}