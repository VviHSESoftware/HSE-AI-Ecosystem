mod config;
mod error;
mod handlers;
mod middlewares;
mod openapi;
mod schemas;
mod state;
mod worker;

use axum::{routing::{post, get}, Router, middleware};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use fang::asynk::async_queue::AsyncQueue;
use fang::asynk::async_worker_pool::AsyncWorkerPool;
use metrics_exporter_prometheus::PrometheusBuilder;
use config::AppEnv;
use state::{AppState, GLOBAL_STATE};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt, Layer};

#[tokio::main]
async fn main() {
    let env = AppEnv::load();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_filter(EnvFilter::builder().parse(&env.log_level).unwrap_or_else(|_| EnvFilter::new("info"))),
        )
        .init();

    info!("Starting {} v{}", env.project_name, env.app_version);

    let builder = PrometheusBuilder::new();
    let handle = builder
        .install_recorder()
        .expect("Failed to install Prometheus recorder");

    let pool = PgPoolOptions::new()
        .max_connections(30)
        .connect(&env.database_url)
        .await
        .expect("Failed to connect to Postgres via SQLx");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS submission_results (
            task_id UUID PRIMARY KEY,
            task_name VARCHAR(255) NOT NULL,
            email VARCHAR(255) NOT NULL,
            mode VARCHAR(50) NOT NULL,
            submission_text TEXT NOT NULL,
            status VARCHAR(50) NOT NULL,
            grade FLOAT,
            feedback TEXT,
            error TEXT,
            created_at TIMESTAMPTZ DEFAULT NOW(),
            updated_at TIMESTAMPTZ DEFAULT NOW()
        );
        CREATE INDEX IF NOT EXISTS idx_submission_task_name ON submission_results(task_name);
        CREATE INDEX IF NOT EXISTS idx_submission_email ON submission_results(email);
        "#
    ).execute(&pool).await.expect("Failed to create submission_results table");

    let queue = AsyncQueue::builder()
        .uri(&env.database_url)
        .max_pool_size(15u32)
        .build();

    let pool_for_metrics = pool.clone();
    tokio::spawn(async move {
        info!("Starting queue depth monitoring task...");
        loop {
            let res = sqlx::query!("SELECT task_type, state, count(*) as count FROM fang_tasks GROUP BY task_type, state")
                .fetch_all(&pool_for_metrics)
                .await;

            if let Ok(rows) = res {
                for row in rows {
                    let state_label = row.state.unwrap_or_else(|| "unknown".to_string());
                    let mode_label = row.task_type.unwrap_or_else(|| "unknown".to_string());
                    metrics::gauge!(
                        "fang_queue_depth",
                        "state" => state_label,
                        "mode" => mode_label
                    ).set(row.count.unwrap_or(0) as f64);
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
        }
    });

    let state = Arc::new(AppState::new(env, pool, queue.clone()));
    GLOBAL_STATE.set(state.clone()).unwrap_or_else(|_| panic!("Failed to set GLOBAL_STATE"));

    let api_routes = Router::new()
        .route("/generate", post(handlers::checker::submit_check))
        .route("/results", post(handlers::checker::get_results))
        .route("/tasks/:task_name/students", get(handlers::checker::get_submitted_students))
        .route("/tasks/:task_name/students/failed", get(handlers::checker::get_failed_students))
        .route("/tasks/:task_name/students/:email/submission", get(handlers::checker::get_submission_text))
        .route("/tasks/:task_name/students/:email/verdict", get(handlers::checker::get_submission_verdict))
        .route_layer(middleware::from_fn_with_state(state.clone(), middlewares::auth_guard));

    let app = Router::new()
        .nest("/api/v2", api_routes)
        .route("/metrics", get(move || std::future::ready(handle.render())))
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", openapi::ApiDoc::openapi()))
        .with_state(state.clone());

    let mut normal_worker_pool = AsyncWorkerPool::builder()
        .number_of_workers(40u32)
        .queue(queue.clone())
        .task_type("autocheck_normal")
        .build();

    let mut precise_worker_pool = AsyncWorkerPool::builder()
        .number_of_workers(20u32)
        .queue(queue.clone())
        .task_type("autocheck_precise")
        .build();

    let normal_task = tokio::spawn(async move {
        info!("Starting Normal worker pool...");
        normal_worker_pool.start().await;
    });

    let precise_task = tokio::spawn(async move {
        info!("Starting Precise worker pool...");
        precise_worker_pool.start().await;
    });

    let server = tokio::spawn(async move {
        let listener = TcpListener::bind("0.0.0.0:8000").await.unwrap();
        info!("API is running on http://0.0.0.0:8000");
        info!("Docs at http://0.0.0.0:8000/docs");
        axum::serve(listener, app).await.unwrap();
    });

    let _ = tokio::join!(normal_task, precise_task, server);
}