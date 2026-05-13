mod config;
mod handlers;
mod openapi;
mod schemas;
mod state;
mod worker;
pub mod repository;
pub mod engine;
pub mod metrics;

use axum::{routing::{post, get}, Router, middleware};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use graphile_worker::WorkerOptions;
use tokio::net::TcpListener;
use tracing::{error, info};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use config::AppEnv;
use state::{AppState};
use common_infra::{
    init_tracing, register_process_metrics,
    common_auth_guard, system_handlers
};
use crate::repository::AutocheckRepository;
use crate::worker::{NormalTask, PreciseTask};

pub const PROJECT_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
    init_tracing(PROJECT_NAME);

    let env = AppEnv::load();

    info!("Starting {} v{}", PROJECT_NAME, APP_VERSION);

    let metrics_handle = register_process_metrics();

    let pool = PgPoolOptions::new()
        .max_connections(30)
        .connect(&env.database_url)
        .await
        .expect("Failed to connect to Postgres via SQLx");

    let repo = Arc::new(AutocheckRepository::new(pool.clone()));

    if let Err(e) = repo.run_migrations().await {
        error!("Failed to run migrations: {}", e);
    }

    metrics::spawn_metrics_collector(repo.clone());

    let state = Arc::new(AppState::new(env, pool.clone()));

    let api_routes = Router::new()
        .route("/submit", post(handlers::process::submit))
        .route("/results", post(handlers::process::get_results))
        .route("/process", post(handlers::process::process))
        .route("/tasks/{task_name}/students", get(handlers::task_stats::get_submitted_students))
        .route("/tasks/{task_name}/students/failed", get(handlers::task_stats::get_failed_students))
        .route("/tasks/{task_name}/students/{email}/submission", get(handlers::task_stats::get_submission_text))
        .route("/tasks/{task_name}/students/{email}/verdict", get(handlers::task_stats::get_submission_verdict))
        .route_layer(middleware::from_fn_with_state(state.clone(), common_auth_guard));

    let app = Router::new()
        .nest("/v2", api_routes)
        .route("/health", get(system_handlers::health))
        .route("/metrics", get(move || std::future::ready(metrics_handle.render())))
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", openapi::ApiDoc::openapi()))
        .with_state(state.clone());

    let normal_runner = WorkerOptions::default()
        .concurrency(40)
        .pg_pool(pool.clone())
        .add_extension(state.clone())
        .define_job::<NormalTask>()
        .init()
        .await.unwrap_or_else(|_| panic!("Failed to create normal worker pool settings"));

    let precise_runner = WorkerOptions::default()
        .concurrency(20)
        .pg_pool(pool.clone())
        .add_extension(state.clone())
        .define_job::<PreciseTask>()
        .init()
        .await.unwrap_or_else(|_| panic!("Failed to create precise worker pool settings"));

    let normal_task = tokio::spawn(async move {
        info!("Starting Normal worker pool...");
        if let Err(e) = normal_runner.run().await {
            error!("Normal worker pool crashed: {}", e);
        }
    });

    let precise_task = tokio::spawn(async move {
        info!("Starting Precise worker pool...");
        if let Err(e) = precise_runner.run().await {
            error!("Precise worker pool crashed: {}", e);
        }
    });

    let server = tokio::spawn(async move {
        let listener = TcpListener::bind("0.0.0.0:8000").await.unwrap();
        info!("API is running on http://0.0.0.0:8000");
        info!("Docs at http://0.0.0.0:8000/docs");
        axum::serve(listener, app).await.unwrap();
    });

    let _ = tokio::join!(normal_task, precise_task, server);
}