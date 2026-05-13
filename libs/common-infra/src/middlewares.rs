use axum::{
    extract::{Request, MatchedPath},
    middleware::Next,
    response::Response,
    http::StatusCode,
};
use std::time::Instant;

pub async fn track_metrics(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();

    if path == "/metrics" || path == "/health" || path.starts_with("/docs") {
        return next.run(req).await;
    }

    let start = Instant::now();
    let method = req.method().to_string();

    let response = next.run(req).await;

    let matched_path = response
        .extensions()
        .get::<MatchedPath>()
        .map(|mp| mp.as_str())
        .unwrap_or("unknown");

    let latency = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    let labels = [
        ("method", method),
        ("path", matched_path.to_string()),
        ("status", status),
    ];

    metrics::counter!("http_requests_total", &labels).increment(1);
    metrics::histogram!("http_request_duration_seconds", &labels[..2]).record(latency);

    response
}

pub trait HasApiTokens {
    fn get_api_tokens(&self) -> &std::collections::HashSet<String>;
}

pub async fn common_auth_guard<S>(
    axum::extract::State(state): axum::extract::State<std::sync::Arc<S>>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode>
where S: HasApiTokens
{
    let auth_header = req.headers().get("Authorization").and_then(|h| h.to_str().ok());

    if let Some(auth_str) = auth_header {
        if let Some(token) = auth_str.strip_prefix("Bearer ") {
            if state.get_api_tokens().contains(token) {
                return Ok(next.run(req).await);
            }
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}