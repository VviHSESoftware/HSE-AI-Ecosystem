use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use crate::config::AppConfig;

tokio::task_local! {
    pub static CURRENT_USER_EMAIL: String;
}

pub async fn auth_and_extract_user(
    State(config): State<Arc<AppConfig>>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok());

    let expected_auth = format!("Bearer {}", config.mcp_bearer_token);

    match auth_header {
        Some(token) if token == expected_auth => {}
        _ => {
            tracing::warn!("Unauthorized attempt or invalid MCP Bearer token");
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    let email = req
        .headers()
        .get("X-User-Email")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "anonymous".to_string());

    Ok(CURRENT_USER_EMAIL.scope(email, next.run(req)).await)
}