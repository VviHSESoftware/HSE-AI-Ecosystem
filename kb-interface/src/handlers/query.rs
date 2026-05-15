use axum::{extract::State, Json};
use std::sync::Arc;
use crate::state::InterfaceState;
use kb_common::schemas::*;
use common_infra::AppError;

#[utoipa::path(
    post,
    path = "/v1/semanticQuery",
    request_body = SemanticQueryReq,
    responses((status = 200, body = QueryRes)),
    security(("bearerAuth" = []))
)]
pub async fn semantic_query(State(state): State<Arc<InterfaceState>>, Json(req): Json<SemanticQueryReq>) -> Result<Json<QueryRes>, AppError> {
    let res = state.query_service.semantic_query(req).await?;
    Ok(Json(res))
}

#[utoipa::path(
    post,
    path = "/v1/timedVideoQuery",
    request_body = TimedVideoQueryReq,
    responses((status = 200, body = QueryRes)),
    security(("bearerAuth" = []))
)]
pub async fn timed_video_query(State(state): State<Arc<InterfaceState>>, Json(req): Json<TimedVideoQueryReq>) -> Result<Json<QueryRes>, AppError> {
    let res = state.query_service.timed_video_query(req).await?;
    Ok(Json(res))
}

#[utoipa::path(
    post,
    path = "/v1/documentPageQuery",
    request_body = DocumentPageQueryReq,
    responses((status = 200, body = QueryRes)),
    security(("bearerAuth" = []))
)]
pub async fn document_page_query(State(state): State<Arc<InterfaceState>>, Json(req): Json<DocumentPageQueryReq>) -> Result<Json<QueryRes>, AppError> {
    let res = state.query_service.document_page_query(req).await?;
    Ok(Json(res))
}

#[utoipa::path(
    post,
    path = "/v1/fullContentQuery",
    request_body = FullContentQueryReq,
    responses((status = 200, body = QueryRes)),
    security(("bearerAuth" = []))
)]
pub async fn full_content_query(State(state): State<Arc<InterfaceState>>, Json(req): Json<FullContentQueryReq>) -> Result<Json<QueryRes>, AppError> {
    let res = state.query_service.full_content_query(req).await?;
    Ok(Json(res))
}

#[utoipa::path(
    post,
    path = "/v1/availableStructure",
    request_body = AvailableStructureReq,
    responses((status = 200, body = StructureRes)),
    security(("bearerAuth" = []))
)]
pub async fn available_structure(State(state): State<Arc<InterfaceState>>, Json(req): Json<AvailableStructureReq>) -> Result<Json<StructureRes>, AppError> {
    let res = state.query_service.available_structure(req).await?;
    Ok(Json(res))
}