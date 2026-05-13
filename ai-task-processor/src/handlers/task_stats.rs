use crate::{
    schemas::{SubmissionTextResponse, VerdictResponse},
    state::AppState
    ,
};
use axum::extract::Path;
use axum::{extract::State, Json};
use common_infra::AppError;
use std::sync::Arc;

#[utoipa::path(
    get, path = "/v2/tasks/{task_name}/students",
    responses((status = 200, body = Vec<String>)),
    security(("bearerAuth" = []))
)]
pub async fn get_submitted_students(
    State(state): State<Arc<AppState>>,
    Path(task_name): Path<String>,
) -> Result<Json<Vec<String>>, AppError> {
    let records = state.repo.get_students_by_task(&task_name, false).await?;
    Ok(Json(records))
}

#[utoipa::path(
    get, path = "/v2/tasks/{task_name}/students/failed",
    responses((status = 200, body = Vec<String>)),
    security(("bearerAuth" = []))
)]
pub async fn get_failed_students(
    State(state): State<Arc<AppState>>,
    Path(task_name): Path<String>,
) -> Result<Json<Vec<String>>, AppError> {
    let records = state.repo.get_students_by_task(&task_name, true).await?;

    Ok(Json(records))
}

#[utoipa::path(
    get, path = "/v2/tasks/{task_name}/students/{email}/submission",
    responses((status = 200, body = SubmissionTextResponse)),
    security(("bearerAuth" = []))
)]
pub async fn get_submission_text(
    State(state): State<Arc<AppState>>,
    Path((task_name, email)): Path<(String, String)>,
) -> Result<Json<SubmissionTextResponse>, AppError> {
    let result = state.repo.get_latest_submission(&task_name, &email).await?;

    match result {
        Some(res) => Ok(Json(res)),
        None => Err(AppError::NotFound("Submission not found".into())),
    }
}

#[utoipa::path(
    get, path = "/v2/tasks/{task_name}/students/{email}/verdict",
    responses((status = 200, body = VerdictResponse)),
    security(("bearerAuth" = []))
)]
pub async fn get_submission_verdict(
    State(state): State<Arc<AppState>>,
    Path((task_name, email)): Path<(String, String)>,
) -> Result<Json<VerdictResponse>, AppError> {
    let result = state.repo.get_latest_verdict(&task_name, &email).await?;

    match result {
        Some(res) => Ok(Json(res)),
        None => Err(AppError::NotFound("Verdict not found".into())),
    }
}