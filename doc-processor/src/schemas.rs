use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct ChunkTextRequest {
    pub text: String,
    #[schema(example = 1000)]
    pub max_chunk_size: usize,
    #[schema(example = 0.1)]
    pub overlap: f32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChunkTextResponse {
    pub chunks: Vec<PlainChunk>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PlainChunk {
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PartitionPart {
    pub text: String,
    pub start: f64,
    pub end: f64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ChunkPartitionedTextRequest {
    pub parts: Vec<PartitionPart>,
    #[schema(example = 1000)]
    pub max_chunk_size: usize,
    #[schema(example = 0.1)]
    pub overlap: f32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PartitionedChunk {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChunkPartitionedTextResponse {
    pub chunks: Vec<PartitionedChunk>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ParseVideoRequest {
    pub video_url: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ParseVideoResponse {
    pub text: String,
    pub parts: Vec<PartitionPart>,
}

#[derive(utoipa::ToSchema)]
pub struct ParseDocumentUpload {
    #[schema(format = Binary)]
    pub file: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DocumentPagePart {
    pub text: String,
    pub page_number: usize,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ParseDocumentResponse {
    pub text: String,
    pub page_count: usize,
    pub parts: Vec<DocumentPagePart>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SupportedTypesResponse {
    pub types: Vec<String>,
}