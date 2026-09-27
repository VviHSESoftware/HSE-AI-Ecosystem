use axum::extract::{Json, Multipart, State};
use std::sync::Arc;
use crate::{
    schemas::*,
    state::{GatewayService, ImageEditFile, ImageEditForm},
};
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

#[utoipa::path(
    post,
    path = "/v1/images/edits",
    request_body(content = ImageEditUpload, content_type = "multipart/form-data"),
    responses((status = 200, description = "OpenAI-compatible image edit response")),
    security(("bearerAuth" = []))
)]
pub async fn image_edit(
    State(state): State<Arc<GatewayService>>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut form = ImageEditForm::default();

    while let Some(field) = multipart.next_field().await.map_err(|e| e.to_string())? {
        let name = field.name().unwrap_or_default().to_string();

        match name.as_str() {
            "image" | "mask" => {
                let file_name = field.file_name().unwrap_or_default().to_string();
                let bytes = field.bytes().await.map_err(|e| e.to_string())?.to_vec();
                form.files.push(ImageEditFile { field_name: name, file_name, bytes });
            }
            "prompt" => form.prompt = Some(field.text().await.map_err(|e| e.to_string())?),
            "n" => form.n = field.text().await.map_err(|e| e.to_string())?.parse().ok(),
            "response_format" => {
                form.response_format = Some(field.text().await.map_err(|e| e.to_string())?)
            }
            "size" => form.size = Some(field.text().await.map_err(|e| e.to_string())?),
            _ => {}
        }
    }

    if form.files.is_empty() {
        return Err(AppError::BadRequest("Missing 'image' field".into()));
    }

    let raw = state.edit_image(form).await?;
    Ok(Json(raw))
}
