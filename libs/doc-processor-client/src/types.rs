use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ChunkTextRequest {
    pub text: String,
    #[cfg_attr(feature = "openapi", schema(example = 1000))]
    pub max_chunk_size: usize,
    #[cfg_attr(feature = "openapi", schema(example = 200))]
    pub overlap: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct PlainChunk {
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ChunkTextResponse {
    pub chunks: Vec<PlainChunk>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct PartitionPart {
    pub text: String,
    pub start: f64,
    pub end: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ChunkPartitionedTextRequest {
    pub parts: Vec<PartitionPart>,
    #[cfg_attr(feature = "openapi", schema(example = 1000))]
    pub max_chunk_size: usize,
    #[cfg_attr(feature = "openapi", schema(example = 200))]
    pub overlap: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct PartitionedChunk {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ChunkPartitionedTextResponse {
    pub chunks: Vec<PartitionedChunk>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ParseVideoRequest {
    pub video_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ParseVideoResponse {
    pub text: String,
    pub parts: Vec<PartitionPart>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ParseDocumentUpload {
    #[cfg_attr(feature = "openapi", schema(format = Binary))]
    pub file: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DocumentPagePart {
    pub text: String,
    pub page_number: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ParseDocumentResponse {
    pub text: String,
    pub page_count: usize,
    pub parts: Vec<DocumentPagePart>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SupportedTypesResponse {
    pub types: Vec<String>,
}