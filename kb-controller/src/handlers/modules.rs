use axum::{extract::{State, Json, Multipart}, http::StatusCode, Extension};
use std::sync::Arc;
use serde_json::{json, Value};
use sqlx::Row;
use crate::{schemas::*, state::KbControllerState, error::AppError};
use crate::middlewares::SystemContext;

#[utoipa::path(post, path = "/api/v1/addModuleBase", request_body = AddModuleBaseReq, responses((status = 200, body = ModuleIdRes)), security(("bearerAuth" = [])))]
pub async fn add_module_base(State(state): State<Arc<KbControllerState>>, Extension(ctx): Extension<SystemContext>, Json(req): Json<AddModuleBaseReq>) -> Result<Json<ModuleIdRes>, AppError> {
    state.check_access(ctx.system_id, req.parent_module_id).await?;

    let p_row = sqlx::query("SELECT path FROM modules WHERE id = $1").bind(req.parent_module_id)
        .fetch_one(&state.db).await.map_err(|_| AppError(StatusCode::NOT_FOUND, "Parent module not found".into()))?;

    let row = sqlx::query("INSERT INTO modules (name, type, url, external_id, external_type, parent_id) VALUES ($1, 'Base', $2, $3, $4, $5) RETURNING id")
        .bind(req.name).bind(req.url).bind(req.external_id).bind(req.external_type).bind(req.parent_module_id)
        .fetch_one(&state.db).await?;

    let path = format!("{}/{}", p_row.get::<Option<String>, _>("path").unwrap_or_default(), row.get::<i32, _>("id"));
    sqlx::query("UPDATE modules SET path = $1 WHERE id = $2").bind(path).bind(row.get::<i32, _>("id")).execute(&state.db).await?;

    Ok(Json(ModuleIdRes { module_id: row.get::<i32, _>("id") }))
}

#[utoipa::path(post, path = "/api/v1/addModuleAccess", request_body = AddModuleAccessReq, responses((status = 200, description = "Success")), security(("bearerAuth" = [])))]
pub async fn add_module_access(State(state): State<Arc<KbControllerState>>, Extension(ctx): Extension<SystemContext>, Json(req): Json<AddModuleAccessReq>) -> Result<Json<Value>, AppError> {
    state.check_access(ctx.system_id, req.module_id).await?;

    let mut tx = state.db.begin().await.map_err(|e| e.to_string())?;

    sqlx::query("DELETE FROM module_access WHERE module_id = $1")
        .bind(req.module_id)
        .execute(&mut *tx)
        .await?;

    for email in req.user_mails {
        let u_row = sqlx::query("INSERT INTO users (email) VALUES ($1) ON CONFLICT (email) DO UPDATE SET email=$1 RETURNING id")
            .bind(email)
            .fetch_one(&mut *tx)
            .await?;

        let user_id: i32 = u_row.get("id");

        sqlx::query("INSERT INTO module_access (module_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(req.module_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(Json(json!({ "status": "success" })))
}

#[utoipa::path(post, path = "/api/v1/setVideoType", request_body = SetVideoTypeReq, responses((status = 200, description = "Success")), security(("bearerAuth" = [])))]
pub async fn set_video_type(State(state): State<Arc<KbControllerState>>, Extension(ctx): Extension<SystemContext>, Json(req): Json<SetVideoTypeReq>) -> Result<Json<Value>, AppError> {
    state.check_access(ctx.system_id, req.module_id).await?;

    let result = state.process_video(&req.video_url).await?;
    state.persist_video_module(req.module_id, result).await?;
    Ok(Json(json!({"status": "success", "message": "Video module processed and saved"})))
}

#[utoipa::path(post, path = "/api/v1/setTextType", request_body = SetTextTypeReq, responses((status = 200, description = "Success")), security(("bearerAuth" = [])))]
pub async fn set_text_type(State(state): State<Arc<KbControllerState>>, Extension(ctx): Extension<SystemContext>, Json(req): Json<SetTextTypeReq>) -> Result<Json<Value>, AppError> {
    state.check_access(ctx.system_id, req.module_id).await?;

    let result = state.process_text(&req.text).await?;
    state.persist_text_module(req.module_id, req.text, result).await?;
    Ok(Json(json!({"status": "success", "message": "Text module processed and saved"})))
}

#[utoipa::path(post, path = "/api/v1/setDocumentType", request_body(content = SetDocumentTypeUpload, content_type = "multipart/form-data"), responses((status = 200, description = "Success")), security(("bearerAuth" = [])))]
pub async fn set_document_type(State(state): State<Arc<KbControllerState>>, Extension(ctx): Extension<SystemContext>, mut multipart: Multipart) -> Result<Json<Value>, AppError> {
    let mut file_bytes = Vec::new();
    let mut filename = String::new();
    let mut module_id = 0;

    while let Some(field) = multipart.next_field().await.map_err(|e| e.to_string())? {
        match field.name() {
            Some("file") => {
                filename = field.file_name().unwrap_or("doc").to_string();
                file_bytes = field.bytes().await.map_err(|e| e.to_string())?.to_vec();
            },
            Some("module_id") => {
                let txt = field.text().await.map_err(|e| e.to_string())?;
                module_id = txt.parse().unwrap_or(0);
            },
            _ => {}
        }
    }

    if module_id == 0 || file_bytes.is_empty() { return Err(AppError(StatusCode::BAD_REQUEST, "Missing fields".into())); }

    state.check_access(ctx.system_id, module_id).await?;

    let result = state.process_document(&filename, file_bytes).await?;
    state.persist_file_module(module_id, filename, result).await?;

    Ok(Json(json!({"status": "success", "message": "Document module processed and saved"})))
}

#[utoipa::path(post, path = "/api/v1/getModuleId", request_body = GetModuleIdReq, responses((status = 200, body = ModuleIdRes)), security(("bearerAuth" = [])))]
pub async fn get_module_id(State(state): State<Arc<KbControllerState>>, Extension(ctx): Extension<SystemContext>, Json(req): Json<GetModuleIdReq>) -> Result<Json<ModuleIdRes>, AppError> {
    let id = state.get_module_id_secure(ctx.system_id, &req.external_id.to_string(), &req.external_type).await?;
    Ok(Json(ModuleIdRes { module_id: id }))
}

#[utoipa::path(post,path = "/api/v1/getSystemId",request_body = GetSystemIdReq,responses((status = 200, body = ModuleIdRes),(status = 404, description = "Module not found")),security(("bearerAuth" = [])))]
pub async fn get_system_id(State(_state): State<Arc<KbControllerState>>, Extension(ctx): Extension<SystemContext>, Json(_req): Json<GetSystemIdReq>) -> Result<Json<ModuleIdRes>, AppError> {
    Ok(Json(ModuleIdRes { module_id: ctx.system_id }))
}

#[utoipa::path(post, path = "/api/v1/invalidateModule", request_body = InvalidateModuleReq, responses((status = 200, description = "Success")), security(("bearerAuth" = [])))]
pub async fn invalidate_module(State(state): State<Arc<KbControllerState>>, Extension(ctx): Extension<SystemContext>, Json(req): Json<InvalidateModuleReq>) -> Result<Json<Value>, AppError> {
    let id = state.get_module_id_secure(ctx.system_id, &req.external_id.to_string(), &req.external_type).await?;

    state.delete_subtree_from_qdrant(id).await?;

    sqlx::query("DELETE FROM modules WHERE id = $1").bind(id).execute(&state.db).await?;
    Ok(Json(json!({ "status": "invalidated" })))
}

#[utoipa::path(get, path = "/api/v1/getSupportedFileTypes", responses((status = 200, body = SupportedFileTypesRes)), security(("bearerAuth" = [])))]
pub async fn get_supported_file_types(State(state): State<Arc<KbControllerState>>) -> Result<Json<SupportedFileTypesRes>, AppError> {
    let res = state.client.get(format!("{}/api/v1/getSupportedDocumentTypes", state.env.doc_processor_url))
        .header("Authorization", format!("Bearer {}", state.env.doc_processor_token))
        .send().await.map_err(|e| e.to_string())?;

    let data: SupportedFileTypesRes = res.json().await.map_err(|e| e.to_string())?;
    Ok(Json(data))
}