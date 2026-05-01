use axum::response::IntoResponse;
use serde_json::{json, Value};
use axum::Json;

#[cfg_attr(feature = "openapi", utoipa::path(
    get,
    path = "/health",
    responses((status = 200, description = "Service is healthy"))
))]
pub async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

#[cfg_attr(feature = "openapi", utoipa::path(
    get,
    path = "/metrics",
    responses((status = 200, description = "Prometheus metrics"))
))]
pub async fn metrics() -> impl IntoResponse {
    let encoder = prometheus::TextEncoder::new();
    let mut buffer = vec![];
    prometheus::Encoder::encode(&encoder, &prometheus::gather(), &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}