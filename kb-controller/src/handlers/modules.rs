use axum::{extract::{State, Json, Multipart}, Extension};
use std::sync::Arc;
use serde_json::{json, Value};
use crate::state::KbControllerState;
use crate::middlewares::SystemContext;
use kb_common::schemas::*;
use common_infra::AppError;

#[utoipa::path(post, path = "/api/v1/addModuleBase", request_body = AddModuleBaseReq, responses((status = 200, body = ModuleIdRes)), security(("bearerAuth" = [])))]
pub async fn add_module_base(State(state): State<Arc<KbControllerState>>, Extension(ctx): Extension<SystemContext>, Json(req): Json<AddModuleBaseReq>) -> Result<Json<ModuleIdRes>, AppError> {
    let module_id = state.module_service.add_module_base(ctx.system_id, req).await?;
    Ok(Json(ModuleIdRes { module_id }))
}

#[utoipa::path(post, path = "/api/v1/addModuleAccess", request_body = AddModuleAccessReq, responses((status = 200, description = "Success")), security(("bearerAuth" = [])))]
pub async fn add_module_access(State(state): State<Arc<KbControllerState>>, Extension(ctx): Extension<SystemContext>, Json(req): Json<AddModuleAccessReq>) -> Result<Json<Value>, AppError> {
    state.module_service.add_module_access(ctx.system_id, req.module_id, &req.user_mails).await?;
    Ok(Json(json!({ "status": "success" })))
}

#[utoipa::path(post, path = "/api/v1/setVideoType", request_body = SetVideoTypeReq, responses((status = 200, description = "Success")), security(("bearerAuth" = [])))]
pub async fn set_video_type(State(state): State<Arc<KbControllerState>>, Extension(ctx): Extension<SystemContext>, Json(req): Json<SetVideoTypeReq>) -> Result<Json<Value>, AppError> {
    state.module_service.set_video_type(ctx.system_id, req.module_id, req.video_url).await?;
    Ok(Json(json!({"status": "success", "message": "Video module processed and saved"})))
}

#[utoipa::path(post, path = "/api/v1/setTextType", request_body = SetTextTypeReq, responses((status = 200, description = "Success")), security(("bearerAuth" = [])))]
pub async fn set_text_type(State(state): State<Arc<KbControllerState>>, Extension(ctx): Extension<SystemContext>, Json(req): Json<SetTextTypeReq>) -> Result<Json<Value>, AppError> {
    state.module_service.set_text_type(ctx.system_id, req.module_id, req.text).await?;
    Ok(Json(json!({"status": "success", "message": "Text module processed and saved"})))
}

#[utoipa::path(post, path = "/api/v1/setDocumentType", request_body(content = SetDocumentTypeUpload, content_type = "multipart/form-data"), responses((status = 200, description = "Success")), security(("bearerAuth" = [])))]
pub async fn set_document_type(State(state): State<Arc<KbControllerState>>, Extension(ctx): Extension<SystemContext>, mut multipart: Multipart) -> Result<Json<Value>, AppError> {
    let mut file_bytes = Vec::new();
    let mut filename = String::new();
    let mut module_id = 0;

    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        match field.name() {
            Some("file") => {
                filename = field.file_name().unwrap_or("doc").to_string();
                file_bytes = field.bytes().await.map_err(|e| AppError::BadRequest(e.to_string()))?.to_vec();
            },
            Some("module_id") => {
                let txt = field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
                module_id = txt.parse().unwrap_or(0);
            },
            _ => {}
        }
    }

    if module_id == 0 || file_bytes.is_empty() { return Err(AppError::BadRequest("Missing fields".into())); }

    state.module_service.set_document_type(ctx.system_id, module_id, filename, file_bytes).await?;
    Ok(Json(json!({"status": "success", "message": "Document module processed and saved"})))
}

#[utoipa::path(post, path = "/api/v1/getModuleId", request_body = GetModuleIdReq, responses((status = 200, body = ModuleIdRes)), security(("bearerAuth" = [])))]
pub async fn get_module_id(State(state): State<Arc<KbControllerState>>, Extension(ctx): Extension<SystemContext>, Json(req): Json<GetModuleIdReq>) -> Result<Json<ModuleIdRes>, AppError> {
    let id = state.module_service.get_module_id_secure(ctx.system_id, &req.external_id, &req.external_type).await?;
    Ok(Json(ModuleIdRes { module_id: id }))
}

#[utoipa::path(post,path = "/api/v1/getSystemId",request_body = GetSystemIdReq,responses((status = 200, body = ModuleIdRes),(status = 404, description = "Module not found")),security(("bearerAuth" = [])))]
pub async fn get_system_id(State(_state): State<Arc<KbControllerState>>, Extension(ctx): Extension<SystemContext>, Json(_req): Json<GetSystemIdReq>) -> Result<Json<ModuleIdRes>, AppError> {
    Ok(Json(ModuleIdRes { module_id: ctx.system_id }))
}

#[utoipa::path(post, path = "/api/v1/invalidateModule", request_body = InvalidateModuleReq, responses((status = 200, description = "Success")), security(("bearerAuth" = [])))]
pub async fn invalidate_module(State(state): State<Arc<KbControllerState>>, Extension(ctx): Extension<SystemContext>, Json(req): Json<InvalidateModuleReq>) -> Result<Json<Value>, AppError> {
    state.module_service.invalidate_module(ctx.system_id, &req.external_id, &req.external_type).await?;
    Ok(Json(json!({ "status": "invalidated" })))
}

#[utoipa::path(get, path = "/api/v1/getSupportedFileTypes", responses((status = 200, body = SupportedFileTypesRes)), security(("bearerAuth" = [])))]
pub async fn get_supported_file_types(State(state): State<Arc<KbControllerState>>) -> Result<Json<SupportedFileTypesRes>, AppError> {
    let data = state.module_service.get_supported_file_types().await?;
    Ok(Json(data))
}