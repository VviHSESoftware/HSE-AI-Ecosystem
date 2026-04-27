use axum::response::IntoResponse;

#[utoipa::path(get, path = "/health")]
pub async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "status": "ok" }))
}

#[utoipa::path(get, path = "/metrics")]
pub async fn metrics() -> impl IntoResponse {
    let encoder = prometheus::TextEncoder::new();
    let mut buffer = vec![];
    prometheus::Encoder::encode(&encoder, &prometheus::gather(), &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}