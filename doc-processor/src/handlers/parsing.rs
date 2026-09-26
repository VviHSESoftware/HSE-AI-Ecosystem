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

    let supported = ["docx", "pptx", "doc", "ppt", "pdf", "png", "jpg", "jpeg", "ipynb"];
    if !supported.contains(&ext.as_str()) {
        return Err(AppError::BadRequest(format!("Extension .{} not supported", ext)));
    }

    if ext == "ipynb" {
        let result = state.parsing.process_ipynb(file_bytes).await?;
        return Ok(Json(result));
    }

    info!("Starting processing for file type: .{}", ext);

    let result = state.parsing.process_document(&ext, file_bytes).await?;
    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/v1/parse",
    request_body(content = ParseDocumentUpload, content_type = "multipart/form-data"),
    responses((status = 200, body = ParseFileResponse)),
    security(("bearerAuth" = []))
)]
pub async fn parse_file(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart
) -> Result<Json<ParseFileResponse>, AppError> {
    let mut file_bytes = Vec::new();
    let mut ext = String::new();

    while let Some(field) = multipart.next_field().await.map_err(|e| e.to_string())? {
        if field.name() == Some("file") {
            let filename = field.file_name().unwrap_or("file").to_string();

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

    let doc_types = ["docx", "pptx", "doc", "ppt", "pdf", "png", "jpg", "jpeg"];
    let video_types = ["mp4", "mkv", "avi", "mov", "webm"];
    let audio_types = ["mp3", "wav", "ogg", "m4a", "flac", "aac", "wma"];

    if ext == "ipynb" {
        info!("Universal parser: processing .ipynb file");
        let res = state.parsing.process_ipynb(file_bytes).await?;
        Ok(Json(ParseFileResponse::Document(res)))
    } else if doc_types.contains(&ext.as_str()) {
        info!("Universal parser: processing document file .{}", ext);
        let res = state.parsing.process_document(&ext, file_bytes).await?;
        Ok(Json(ParseFileResponse::Document(res)))
    } else if video_types.contains(&ext.as_str()) {
        info!("Universal parser: processing video file .{}", ext);
        let res = state.parsing.process_video_file(&ext, file_bytes).await?;
        Ok(Json(ParseFileResponse::Media(res)))
    } else if audio_types.contains(&ext.as_str()) {
        info!("Universal parser: processing audio file .{}", ext);
        let res = state.parsing.process_audio_file(&ext, file_bytes).await?;
        Ok(Json(ParseFileResponse::Media(res)))
    } else {
        Err(AppError::BadRequest(format!("Extension .{} not supported", ext)))
    }
}

#[utoipa::path(get, path = "/v1/getSupportedDocumentTypes", responses((status = 200, body = SupportedTypesResponse)), security(("bearerAuth" = [])))]
pub async fn get_supported_types() -> Result<Json<SupportedTypesResponse>, AppError> {
    Ok(Json(SupportedTypesResponse {
        types: vec!["docx", "pptx", "doc", "ppt", "pdf", "png", "jpg", "jpeg", "ipynb"].into_iter().map(String::from).collect(),
    }))
}

#[utoipa::path(
    post,
    path = "/v1/parseVideoFile",
    request_body(content = ParseVideoFileUpload, content_type = "multipart/form-data"),
    responses((status = 200, body = ParseVideoResponse)),
    security(("bearerAuth" = []))
)]
pub async fn parse_video_file(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart
) -> Result<Json<ParseVideoResponse>, AppError> {
    let mut file_bytes = Vec::new();
    let mut ext = String::new();

    while let Some(field) = multipart.next_field().await.map_err(|e| e.to_string())? {
        if field.name() == Some("file") {
            let filename = field.file_name().unwrap_or("video.mp4").to_string();

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

    let supported = ["mp4", "mkv", "avi", "mov", "webm"];
    if !supported.contains(&ext.as_str()) {
        return Err(AppError::BadRequest(format!("Extension .{} not supported", ext)));
    }

    info!("Starting processing for video file of type: .{}", ext);

    let result = state.parsing.process_video_file(&ext, file_bytes).await?;
    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/v1/parseAudioFile",
    request_body(content = ParseAudioFileUpload, content_type = "multipart/form-data"),
    responses((status = 200, body = ParseVideoResponse)),
    security(("bearerAuth" = []))
)]
pub async fn parse_audio_file(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart
) -> Result<Json<ParseVideoResponse>, AppError> {
    let mut file_bytes = Vec::new();
    let mut ext = String::new();

    while let Some(field) = multipart.next_field().await.map_err(|e| e.to_string())? {
        if field.name() == Some("file") {
            let filename = field.file_name().unwrap_or("audio.mp3").to_string();

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

    let supported = ["mp3", "wav", "ogg", "m4a", "flac", "aac", "wma"];
    if !supported.contains(&ext.as_str()) {
        return Err(AppError::BadRequest(format!("Extension .{} not supported", ext)));
    }

    info!("Starting processing for audio file of type: .{}", ext);

    let result = state.parsing.process_audio_file(&ext, file_bytes).await?;
    Ok(Json(result))
}