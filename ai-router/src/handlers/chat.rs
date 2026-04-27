use axum::{extract::State, Json};
use std::sync::Arc;
use crate::{schemas::*, state::RouterState, engine, error::AppError};

#[utoipa::path(
    post,
    path = "/api/v1/chat",
    request_body = RouterRequest,
    responses(
        (status = 200, description = "Успешная обработка запроса", body = RouterResponse),
        (status = 401, description = "Неверный токен авторизации"),
        (status = 502, description = "Ошибка внешнего сервиса (Gateway/KB)")
    ),
    security(("bearerAuth" = []))
)]
pub async fn process_request(
    State(state): State<Arc<RouterState>>,
    Json(req): Json<RouterRequest>
) -> Result<Json<RouterResponse>, AppError> {
    let response = engine::process_pipeline(&state, req).await?;
    Ok(Json(response))
}