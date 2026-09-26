use axum::{extract::{State, Multipart}, Json};
use std::sync::Arc;
use uuid::Uuid;
use tracing::info;
use common_infra::AppError;
use doc_processor_client::ParseFileResponse;
use crate::{
    schemas::{
        UniversalTaskRequest, SubmitResponse, ResultsRequest,
        ResultItem, SingleResponse
    },
    state::AppState,
    worker::UniversalTask,
    engine::run_managed_task,
};
use crate::schemas::{
    CheckMode, TaskPayload, FileSubmission,
    Submission, UploadMultipartForm
};
use crate::worker::{NormalTask, PreciseTask};

fn parse_form_value(s: String) -> serde_json::Value {
    if s == "true" {
        serde_json::Value::Bool(true)
    } else if s == "false" {
        serde_json::Value::Bool(false)
    } else if let Ok(n) = s.parse::<i64>() {
        serde_json::Value::Number(n.into())
    } else if let Ok(f) = s.parse::<f64>() {
        if let Some(num) = serde_json::Number::from_f64(f) {
            serde_json::Value::Number(num)
        } else {
            serde_json::Value::String(s)
        }
    } else {
        serde_json::Value::String(s)
    }
}

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
    post,
    path = "/v2/submit/multipart",
    request_body(
        content = inline(UploadMultipartForm),
        content_type = "multipart/form-data"
    ),
    responses(
        (status = 200, body = SubmitResponse),
        (status = 400, description = "Bad Request")
    ),
    security(("bearerAuth" = []))
)]
pub async fn submit_multipart(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<SubmitResponse>, AppError> {
    let mut fields_map = serde_json::Map::new();
    let mut files = Vec::new();
    let mut mode = CheckMode::Normal;

    const MAX_FILES: usize = 10;

    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        let name = field.name().unwrap_or_default().to_string();

        if name == "files" || name == "files[]" || name.starts_with("files[") {
            if files.len() >= MAX_FILES {
                return Err(AppError::BadRequest(format!("Too many files. Limit is {}", MAX_FILES)));
            }

            let filename = field.file_name().unwrap_or("unnamed").to_string();
            let bytes = field.bytes().await.map_err(|e| AppError::BadRequest(e.to_string()))?.to_vec();

            let content = match state.doc_processor.parse_file(&filename, bytes.clone()).await {
                Ok(ParseFileResponse::Document(doc)) => doc.text,
                Ok(ParseFileResponse::Media(media)) => media.text,
                Err(_) => {
                    String::from_utf8(bytes)
                        .map_err(|_| AppError::BadRequest(format!("File '{}' is binary and cannot be processed", filename)))?
                }
            };

            files.push(FileSubmission { filename, content });
        } else {
            match name.as_str() {
                "mode" => {
                    let mode_str = field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
                    if mode_str.to_lowercase() == "precise" {
                        mode = CheckMode::Precise;
                    }
                }
                field_name => {
                    let value_str = field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
                    fields_map.insert(field_name.to_string(), parse_form_value(value_str));
                }
            }
        }
    }

    let task_type = fields_map
        .get("type")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing required field: 'type'".into()))?;

    match task_type.as_str() {
        "autocheck" => {
            if !files.is_empty() {
                fields_map.insert(
                    "submission".to_string(),
                    serde_json::to_value(Submission::Files(files)).map_err(|e| AppError::Internal(e.to_string()))?
                );
            }
        }
        _ => {
            return Err(AppError::BadRequest(format!("Unsupported multipart task type: {}", task_type)));
        }
    }

    let payload: TaskPayload = serde_json::from_value(serde_json::Value::Object(fields_map))
        .map_err(|err| AppError::BadRequest(format!("Form validation failed for type '{}': {}", task_type, err)))?;

    let task_id = Uuid::new_v4();

    state.repo.create_task(
        task_id,
        payload.task_type(),
        &mode.to_string(),
        serde_json::to_value(&payload).map_err(|e| AppError::Internal(e.to_string()))?
    ).await?;

    let inner_payload = UniversalTask {
        task_id,
        req: UniversalTaskRequest {
            payload,
            mode: mode.clone(),
        }
    };

    match mode {
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