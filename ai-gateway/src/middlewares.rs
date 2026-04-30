use axum::{extract::{Request, State, MatchedPath}, http::StatusCode, middleware::Next, response::Response};
use std::sync::Arc;
use crate::state::GatewayService;
use std::time::Instant;
use crate::metrics::{HTTP_REQUESTS_TOTAL, HTTP_REQUEST_DURATION};

pub async fn auth_guard(
    State(state): State<Arc<GatewayService>>,
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

pub async fn track_metrics(req: Request<axum::body::Body>, next: Next) -> Response {
    let path = req.uri().path().to_string();
    if path == "/metrics" || path == "/health" || path.starts_with("/docs") || path.starts_with("/api-docs") {
        return next.run(req).await;
    }

    let start = Instant::now();

    let method = req.method().clone().to_string();

    let response = next.run(req).await;

    let path = response.extensions()
        .get::<MatchedPath>()
        .map(|mp| mp.as_str())
        .unwrap_or("unknown");

    let latency = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    HTTP_REQUESTS_TOTAL.with_label_values(&[&method, &path, &status]).inc();
    HTTP_REQUEST_DURATION.with_label_values(&[&method, &path]).observe(latency);

    response
}