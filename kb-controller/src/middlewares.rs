use crate::state::KbControllerState;
use axum::{extract::{Request, State}, http::StatusCode, middleware::Next, response::Response};
use sha2::{Digest, Sha256};
use std::sync::Arc;

#[derive(Clone)]
pub struct SystemContext {
    pub system_id: i32,
}

pub async fn system_auth_guard(
    State(state): State<Arc<KbControllerState>>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req.headers().get("Authorization").and_then(|h| h.to_str().ok());

    if let Some(auth_str) = auth_header {
        if auth_str.starts_with("Bearer ") {
            let token = &auth_str[7..];

            let mut hasher = Sha256::new();
            hasher.update(token.as_bytes());
            let hash = hex::encode(hasher.finalize());

            let row = sqlx::query("SELECT module_id FROM module_system WHERE token_hash = $1")
                .bind(hash)
                .fetch_optional(&state.db)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            if let Some(r) = row {
                let system_id: i32 = sqlx::Row::get(&r, "module_id");
                req.extensions_mut().insert(SystemContext { system_id });
                return Ok(next.run(req).await);
            }
        }
    }
    Err(StatusCode::UNAUTHORIZED)
}