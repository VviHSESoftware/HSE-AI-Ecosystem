use axum::{extract::{State, Json}};
use std::sync::Arc;
use crate::{schemas::*, state::GatewayService};
use common_infra::AppError;

#[utoipa::path(post, path = "/v1/llm", request_body = LLMRequest, responses((status = 200, body = LLMResponse)), security(("bearerAuth" = [])))]
pub async fn llm(State(state): State<Arc<GatewayService>>, Json(req): Json<LLMRequest>) -> Result<Json<LLMResponse>, AppError> {
    let payload = serde_json::to_value(&req)?;
    let response = state.chat_completion(payload, &req.mode).await?;
    Ok(Json(response))
}

#[utoipa::path(post, path = "/v1/vlm", request_body = VLMRequest, responses((status = 200, body = LLMResponse)), security(("bearerAuth" = [])))]
pub async fn vlm(State(state): State<Arc<GatewayService>>, Json(req): Json<VLMRequest>) -> Result<Json<LLMResponse>, AppError> {
    let response = state.vlm_analyze(req).await?;
    Ok(Json(response))
}

#[utoipa::path(post, path = "/v1/embeddings", request_body = EmbeddingRequest, responses((status = 200, body = EmbeddingResponse)), security(("bearerAuth" = [])))]
pub async fn embeddings(State(state): State<Arc<GatewayService>>, Json(req): Json<EmbeddingRequest>) -> Result<Json<EmbeddingResponse>, AppError> {
    let response = state.create_embeddings(req.input).await?;
    Ok(Json(response))
}