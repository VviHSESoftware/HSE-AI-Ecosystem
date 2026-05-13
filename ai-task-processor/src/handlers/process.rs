use axum::{extract::State, Json};
use std::sync::Arc;
use uuid::Uuid;
use tracing::info;
use common_infra::AppError;
use crate::{
    schemas::{
        UniversalTaskRequest, SubmitResponse, ResultsRequest,
        ResultItem, SingleResponse
    },
    state::AppState,
    worker::UniversalTask,
    engine::run_managed_task,
};
use crate::schemas::CheckMode;
use crate::worker::{NormalTask, PreciseTask};

#[utoipa::path(
    post, path = "/v2/submit",
    request_body = UniversalTaskRequest,
    responses((status = 200, body = SubmitResponse)),
    security(("bearerAuth" = []))
)]
pub async fn submit(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UniversalTaskRequest>,
) -> Result<Json<SubmitResponse>, AppError> {
    let processor = crate::engine::get_processor(&req.payload);
    info!("Received async task: {}", processor.log_summary(&req.payload));

    let task_id = Uuid::new_v4();


    state.repo.create_task(
        task_id,
        req.payload.task_type(),
        &req.mode.to_string(),
        serde_json::to_value(&req.payload).map_err(|e| AppError::Internal(e.to_string()))?
    ).await?;

    let inner_payload = UniversalTask {
        task_id,
        req: req.clone()
    };

    match req.mode {
        CheckMode::Normal => {
            state.worker_utils
                .add_job(NormalTask(inner_payload), Default::default())
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }
        CheckMode::Precise => {
            state.worker_utils
                .add_job(PreciseTask(inner_payload), Default::default())
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }
    }


    Ok(Json(SubmitResponse { task_id }))
}

#[utoipa::path(
    post, path = "/v2/process",
    request_body = UniversalTaskRequest,
    responses((status = 200, body = SingleResponse)),
    security(("bearerAuth" = []))
)]
pub async fn process(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UniversalTaskRequest>,
) -> Result<Json<SingleResponse>, AppError> {
    let processor = crate::engine::get_processor(&req.payload);
    info!("Direct sync task: {}", processor.log_summary(&req.payload));

    let task_id = Uuid::new_v4();

    state.repo.create_task(
        task_id,
        req.payload.task_type(),
        &req.mode.to_string(),
        serde_json::to_value(&req.payload).map_err(|e| AppError::Internal(e.to_string()))?
    ).await?;

    match run_managed_task(&state, task_id, &req.payload, &req.mode).await {
        Ok(result) => Ok(Json(SingleResponse {
            task_result: Some(result),
            error: None,
        })),
        Err(e) => {
            Err(e.into())
        }
    }
}

#[utoipa::path(
    post, path = "/v2/results",
    request_body = ResultsRequest,
    responses((status = 200, body = Vec<ResultItem>)),
    security(("bearerAuth" = []))
)]
pub async fn get_results(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ResultsRequest>,
) -> Result<Json<Vec<ResultItem>>, AppError> {
    let results = state.repo.get_results_by_ids(&payload.task_ids[..]).await?;

    Ok(Json(results))
}