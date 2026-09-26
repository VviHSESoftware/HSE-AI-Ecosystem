use axum::extract::{Json, State};
use std::sync::Arc;
use crate::{schemas::*, state::GatewayService};
use common_infra::AppError;

#[utoipa::path(
    post,
    path = "/v1/image",
    request_body = ImageRequest,
    responses((status = 200, body = ImageResponse)),
    security(("bearerAuth" = []))
)]
pub async fn image(
    State(state): State<Arc<GatewayService>>,
    Json(req): Json<ImageRequest>,
) -> Result<Json<ImageResponse>, AppError> {
    let response = state.generate_image(req).await?;
    Ok(Json(response))
}
