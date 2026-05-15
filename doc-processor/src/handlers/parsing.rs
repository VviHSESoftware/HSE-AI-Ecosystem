use axum::{extract::{State, Json, Multipart}, http::StatusCode};
use std::sync::Arc;
use tracing::info;
use crate::{schemas::*, state::AppState};
use common_infra::AppError;

#[utoipa::path(post, path = "/v1/parseVideo", request_body = ParseVideoRequest, responses((status = 200, body = ParseVideoResponse)), security(("bearerAuth" = [])))]
pub async fn parse_video(State(state): State<Arc<AppState>>, Json(req): Json<ParseVideoRequest>) -> Result<Json<ParseVideoResponse>, AppError> {
    let result = state.parsing.parse_video(&req.video_url).await?;
    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/v1/parseDocument",
    request_body(content = ParseDocumentUpload, content_type = "multipart/form-data"),
    responses((status = 200, body = ParseDocumentResponse)),
    security(("bearerAuth" = []))
)]
pub async fn parse_document(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart
) -> Result<Json<ParseDocumentResponse>, AppError> {
    let mut file_bytes = Vec::new();
    let mut ext = String::new();

    while let Some(field) = multipart.next_field().await.map_err(|e| e.to_string())? {
        if field.name() == Some("file") {
            let filename = field.file_name().unwrap_or("file.txt").to_string();

            ext = std::path::Path::new(&filename)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();

            file_bytes = field.bytes().await.map_err(|e| e.to_string())?.to_vec();
        }
    }

    if file_bytes.is_empty() {
        return Err(AppError::BadRequest("No file provided".into()));
    }
    if ext.is_empty() {
        return Err(AppError::BadRequest("File extension missing".into()));
    }

    let supported = ["docx", "pptx", "doc", "ppt", "pdf", "png", "jpg", "jpeg"];
    if !supported.contains(&ext.as_str()) {
        return Err(AppError::BadRequest(format!("Extension .{} not supported", ext)));
    }

    info!("Starting processing for file type: .{}", ext);

    let result = state.parsing.process_document(&ext, file_bytes).await?;
    Ok(Json(result))
}

#[utoipa::path(get, path = "/v1/getSupportedDocumentTypes", responses((status = 200, body = SupportedTypesResponse)), security(("bearerAuth" = [])))]
pub async fn get_supported_types() -> Result<Json<SupportedTypesResponse>, AppError> {
    Ok(Json(SupportedTypesResponse {
        types: vec!["docx", "pptx", "doc", "ppt", "pdf", "png", "jpg", "jpeg"].into_iter().map(String::from).collect(),
    }))
}