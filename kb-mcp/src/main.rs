mod config;
mod handlers;
mod kb_client;
mod middleware;
mod schemas;

use axum::{
    middleware as axum_middleware,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

use config::AppConfig;
use handlers::{handle_json_rpc, AppState};
use kb_client::KbClient;
use middleware::auth_and_extract_user;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let config = Arc::new(AppConfig::load());

    let kb_client = KbClient::new(
        config.kb_interface_url.clone(),
        config.kb_interface_token.clone(),
    );

    let state = Arc::new(AppState { kb_client });

    let mcp_router = Router::new()
        .route("/mcp", post(handle_json_rpc))
        .layer(axum_middleware::from_fn_with_state(
            config.clone(),
            auth_and_extract_user,
        ))
        .with_state(state);

    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .merge(mcp_router);

    let listener = TcpListener::bind("0.0.0.0:8000").await.unwrap();
    info!("HSE KB MCP Server started on http://0.0.0.0:8000");

    axum::serve(listener, app).await.unwrap();
}