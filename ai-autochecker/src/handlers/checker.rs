use axum::{extract::State, Json};
use std::sync::Arc;
use axum::extract::Path;
use fang::AsyncQueueable;
use uuid::Uuid;
use tracing::info;

use crate::{
    error::AppError,
    schemas::{AssignmentMessage, SubmitResponse, ResultsRequest, ResultItem, VerdictResponse, SubmissionTextResponse},
    state::AppState,
    worker::CheckJob,
};

#[utoipa::path(
    post, path = "/api/v2/generate",
    request_body = AssignmentMessage,
    responses((status = 200, body = SubmitResponse)),
    security(("bearerAuth" = []))
)]
pub async fn submit_check(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AssignmentMessage>,
) -> Result<Json<SubmitResponse>, AppError> {
    info!("Received submission for email: {} on task: {}", payload.email, payload.task_name);

    let task_id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO submission_results (task_id, task_name, email, mode, submission_text, status)
        VALUES ($1, $2, $3, $4, $5, 'pending')
        "#
    )
        .bind(task_id)
        .bind(&payload.task_name)
        .bind(&payload.email)
        .bind(payload.mode.to_string())
        .bind(&payload.submission)
        .execute(&state.db).await?;

    let job = CheckJob { task_id, payload };
    let mut queue = state.queue.clone();
    queue.insert_task(&job).await?;

    Ok(Json(SubmitResponse { task_id }))
}


#[utoipa::path(
    post, path = "/api/v2/results",
    request_body = ResultsRequest,
    responses((status = 200, body = Vec<ResultItem>)),
    security(("bearerAuth" = []))
)]
pub async fn get_results(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ResultsRequest>,
) -> Result<Json<Vec<ResultItem>>, AppError> {
    let results = sqlx::query_as::<_, ResultItem>(
        "SELECT task_id, status, grade, feedback, error FROM submission_results WHERE task_id = ANY($1)"
    )
        .bind(&payload.task_ids[..])
        .fetch_all(&state.db)
        .await?;

    Ok(Json(results))
}

#[utoipa::path(
    get, path = "/api/v2/tasks/{task_name}/students",
    responses((status = 200, body = Vec<String>)),
    security(("bearerAuth" = []))
)]
pub async fn get_submitted_students(
    State(state): State<Arc<AppState>>,
    Path(task_name): Path<String>,
) -> Result<Json<Vec<String>>, AppError> {
    let records = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT email FROM submission_results WHERE task_name = $1"
    )
        .bind(task_name)
        .fetch_all(&state.db).await?;

    Ok(Json(records))
}

#[utoipa::path(
    get, path = "/api/v2/tasks/{task_name}/students/failed",
    responses((status = 200, body = Vec<String>)),
    security(("bearerAuth" = []))
)]
pub async fn get_failed_students(
    State(state): State<Arc<AppState>>,
    Path(task_name): Path<String>,
) -> Result<Json<Vec<String>>, AppError> {
    let records = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT email FROM submission_results WHERE task_name = $1 AND status = 'error'"
    )
        .bind(task_name)
        .fetch_all(&state.db).await?;

    Ok(Json(records))
}

#[utoipa::path(
    get, path = "/api/v2/tasks/{task_name}/students/{email}/submission",
    responses((status = 200, body = SubmissionTextResponse)),
    security(("bearerAuth" = []))
)]
pub async fn get_submission_text(
    State(state): State<Arc<AppState>>,
    Path((task_name, email)): Path<(String, String)>,
) -> Result<Json<SubmissionTextResponse>, AppError> {
    let result = sqlx::query_as::<_, SubmissionTextResponse>(
        "SELECT submission_text FROM submission_results WHERE task_name = $1 AND email = $2 ORDER BY created_at DESC LIMIT 1"
    )
        .bind(task_name)
        .bind(email)
        .fetch_optional(&state.db).await?;

    match result {
        Some(res) => Ok(Json(res)),
        None => Err(AppError::not_found("Submission not found")),
    }
}

#[utoipa::path(
    get, path = "/api/v2/tasks/{task_name}/students/{email}/verdict",
    responses((status = 200, body = VerdictResponse)),
    security(("bearerAuth" = []))
)]
pub async fn get_submission_verdict(
    State(state): State<Arc<AppState>>,
    Path((task_name, email)): Path<(String, String)>,
) -> Result<Json<VerdictResponse>, AppError> {
    let result = sqlx::query_as::<_, VerdictResponse>(
        "SELECT status, grade, feedback, error, mode, created_at FROM submission_results WHERE task_name = $1 AND email = $2 ORDER BY created_at DESC LIMIT 1"
    )
        .bind(task_name)
        .bind(email)
        .fetch_optional(&state.db).await?;

    match result {
        Some(res) => Ok(Json(res)),
        None => Err(AppError::not_found("Verdict not found")),
    }
}