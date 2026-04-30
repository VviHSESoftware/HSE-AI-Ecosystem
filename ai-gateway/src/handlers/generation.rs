use axum::{extract::{State, Json}};
use std::sync::Arc;
use crate::{schemas::*, state::GatewayService, error::AppError};

#[utoipa::path(post, path = "/v1/llm", request_body = LLMRequest, responses((status = 200, body = LLMResponse)), security(("bearerAuth" = [])))]
pub async fn llm(State(state): State<Arc<GatewayService>>, Json(req): Json<LLMRequest>) -> Result<Json<LLMResponse>, AppError> {
    let payload = serde_json::to_value(&req).unwrap();
    let raw = state.chat_completion(payload, &req.mode).await?;

    Ok(Json(LLMResponse {
        content: raw.pointer("/choices/0/message/content").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        model: raw["model"].as_str().unwrap_or("").to_string(),
        usage: raw["usage"].clone(),
    }))
}

#[utoipa::path(post, path = "/v1/vlm", request_body = VLMRequest, responses((status = 200, body = LLMResponse)), security(("bearerAuth" = [])))]
pub async fn vlm(State(state): State<Arc<GatewayService>>, Json(req): Json<VLMRequest>) -> Result<Json<LLMResponse>, AppError> {
    let raw = state.vlm_analyze(req).await?;
    Ok(Json(LLMResponse {
        content: raw.pointer("/choices/0/message/content").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        model: raw["model"].as_str().unwrap_or("").to_string(),
        usage: raw["usage"].clone(),
    }))
}

#[utoipa::path(post, path = "/v1/embeddings", request_body = EmbeddingRequest, responses((status = 200, body = EmbeddingResponse)), security(("bearerAuth" = [])))]
pub async fn embeddings(State(state): State<Arc<GatewayService>>, Json(req): Json<EmbeddingRequest>) -> Result<Json<EmbeddingResponse>, AppError> {
    let raw = state.create_embeddings(req.input).await?;

    let mut embeddings = vec![];
    if let Some(data) = raw["data"].as_array() {
        for item in data {
            if let Some(arr) = item["embedding"].as_array() {
                let vec_f32: Vec<f32> = arr.iter().filter_map(|v| v.as_f64())
                    .map(|v| v as f32)
                    .collect();
                embeddings.push(vec_f32);
            }
        }
    }

    Ok(Json(EmbeddingResponse { embeddings, model: raw["model"].as_str().unwrap_or("").to_string(), usage: raw["usage"].clone() }))
}