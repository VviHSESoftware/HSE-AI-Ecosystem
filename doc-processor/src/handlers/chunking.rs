use axum::{extract::{State, Json}};
use std::sync::Arc;
use crate::{schemas::*, state::DocumentService, error::AppError};

#[utoipa::path(post, path = "/api/v1/chunkText", request_body = ChunkTextRequest, responses((status = 200, body = ChunkTextResponse)), security(("bearerAuth" = [])))]
pub async fn chunk_text(State(state): State<Arc<DocumentService>>, Json(req): Json<ChunkTextRequest>) -> Result<Json<ChunkTextResponse>, AppError> {
    let chunks = state.chunk_text(req);

    Ok(Json(ChunkTextResponse { chunks }))
}

#[utoipa::path(post, path = "/api/v1/chunkPartitionedText", request_body = ChunkPartitionedTextRequest, responses((status = 200, body = ChunkPartitionedTextResponse)), security(("bearerAuth" = [])))]
pub async fn chunk_partitioned(State(state): State<Arc<DocumentService>>, Json(req): Json<ChunkPartitionedTextRequest>) -> Result<Json<ChunkPartitionedTextResponse>, AppError> {
    let chunks = state.chunk_partitioned_text(req);
    Ok(Json(ChunkPartitionedTextResponse { chunks }))
}