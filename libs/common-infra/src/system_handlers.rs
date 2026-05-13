use axum::extract::State;
use axum::response::IntoResponse;
use serde_json::{json, Value};
use axum::Json;
use metrics_exporter_prometheus::PrometheusHandle;

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
pub async fn metrics(State(handle): State<PrometheusHandle>) -> impl IntoResponse {
    handle.render()
}