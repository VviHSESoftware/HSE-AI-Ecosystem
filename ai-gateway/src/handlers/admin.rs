use axum::{extract::State, Json};
use std::sync::Arc;
use serde_json::{json, Value};
use crate::{state::GatewayService, config::DynamicConfig, error::AppError};

#[utoipa::path(post, path = "/v1/admin/config/reload", responses((status = 200, description = "Success")), security(("bearerAuth" = [])))]
pub async fn reload(State(state): State<Arc<GatewayService>>) -> Result<Json<Value>, AppError> {
    let cfg = DynamicConfig::load_from_file().map_err(|e| e.to_string())?;
    state.update_config(cfg).await;
    Ok(Json(json!({"status": "success", "message": "Config reloaded"})))
}

#[utoipa::path(get, path = "/v1/admin/status/models", responses((status = 200, description = "Gateway status")), security(("bearerAuth" = [])))]
pub async fn status(State(state): State<Arc<GatewayService>>) -> Json<Value> {
    Json(state.get_status().await)
}