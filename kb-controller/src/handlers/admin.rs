use axum::extract::{Json, State};
use std::sync::Arc;
use common_infra::AppError;
use kb_common::schemas::{CreateSystemReq, CreateSystemRes};
use crate::state::KbControllerState;

#[utoipa::path(post, path = "/api/v1/admin/createSystem", request_body = CreateSystemReq, responses((status = 200, body = CreateSystemRes)), security(("bearerAuth" = [])))]
pub async fn create_system(
    State(state): State<Arc<KbControllerState>>,
    Json(req): Json<CreateSystemReq>
) -> Result<Json<CreateSystemRes>, AppError> {
    let res = state.admin_service.create_system(&req.name).await?;
    Ok(Json(res))
}