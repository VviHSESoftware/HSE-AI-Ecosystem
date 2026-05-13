use crate::state::GatewayService;
use ai_gateway_client::AudioUpload;
use axum::extract::{Multipart, State};
use common_infra::AppError;
use std::sync::Arc;

#[utoipa::path(post, path = "/v1/asr", request_body(content = AudioUpload,content_type = "multipart/form-data"), responses((status = 200, description = "Verbose JSON response")), security(("bearerAuth" = [])))]
pub async fn asr(State(state): State<Arc<GatewayService>>, mut multipart: Multipart) -> Result<axum::Json<serde_json::Value>, AppError> {
    while let Some(field) = multipart.next_field().await.map_err(|e| e.to_string())? {
        if field.name() == Some("file") {
            let file_name = field.file_name().unwrap_or("audio.mp3").to_string();
            let data = field.bytes().await.map_err(|e| e.to_string())?.to_vec();

            let raw = state.transcribe_audio(file_name, data).await?;
            return Ok(axum::Json(raw));
        }
    }
    Err(AppError::BadRequest("Missing file field".into()))
}