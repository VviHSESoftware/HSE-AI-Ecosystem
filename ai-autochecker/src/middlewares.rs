use axum::{extract::{Request, State}, http::StatusCode, middleware::Next, response::Response};
use std::sync::Arc;
use crate::state::AppState;

pub async fn auth_guard(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req.headers().get("Authorization").and_then(|h| h.to_str().ok());
    if let Some(auth_str) = auth_header {
        if auth_str.starts_with("Bearer ") {
            let token = &auth_str[7..];
            if state.env.api_tokens.contains(token) {
                return Ok(next.run(req).await);
            }
        }
    }
    Err(StatusCode::UNAUTHORIZED)
}