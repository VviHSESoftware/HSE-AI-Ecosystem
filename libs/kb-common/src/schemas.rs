use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;

// --- kb-interface schemas ---

#[derive(Debug, Deserialize, ToSchema)]
pub struct SemanticQueryReq {
    #[schema(example = "student@university.edu")]
    pub email: String,
    #[schema(example = json!([10, 20]))]
    pub module_ids: Vec<i32>,
    #[schema(example = "Как работают LLM модели?")]
    pub semantic_query: String,
    pub keywords: Vec<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TimedVideoQueryReq {
    pub email: String,
    pub module_id: i32,
    pub second_start: f64,
    pub second_end: f64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DocumentPageQueryReq {
    pub email: String,
    pub module_id: i32,
    pub page: i32,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct FullContentQueryReq {
    pub email: String,
    pub module_ids: Vec<i32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct QueryChunk {
    pub desc: String,
    pub text: String,
    pub url: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct QueryRes {
    pub chunks: Vec<QueryChunk>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AvailableStructureReq {
    pub email: String,
    pub module_ids: Option<Vec<i32>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ModuleNode {
    pub id: i32,
    pub name: String,
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_length: Option<f64>,
    #[schema(no_recursion)]
    pub sub_modules: Vec<ModuleNode>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StructureRes {
    pub modules: Vec<ModuleNode>,
}

// --- kb-controller schemas ---

#[derive(Debug, Deserialize, ToSchema)]
pub struct AddModuleBaseReq {
    pub name: String,
    pub url: String,
    pub external_id: String,
    pub external_type: String,
    pub parent_module_id: i32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ModuleIdRes {
    pub module_id: i32,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSystemReq {
    pub name: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateSystemRes {
    pub system_id: i32,
    pub system_token: String,
    pub note: String
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AddModuleAccessReq {
    pub module_id: i32,
    pub user_mails: Vec<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SetVideoTypeReq {
    pub module_id: i32,
    pub video_url: String,
}

#[derive(utoipa::ToSchema)]
pub struct SetDocumentTypeUpload {
    pub module_id: i32,
    pub filename: String,
    #[schema(format = Binary)]
    pub file: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SetTextTypeReq {
    pub module_id: i32,
    pub text: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct GetModuleIdReq {
    pub external_id: String,
    pub external_type: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct InvalidateModuleReq {
    pub external_id: String,
    pub external_type: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct GetSystemIdReq {
    pub system_name: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SupportedFileTypesRes {
    pub types: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HashResText {
    pub chunks: Vec<TextChunkRes>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextChunkRes {
    pub embedding: Vec<f32>,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HashResVideo {
    pub text: String,
    pub parts: Vec<VideoPartRes>,
    pub chunks: Vec<VideoChunkRes>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoPartRes {
    pub text: String,
    pub start: f32,
    pub end: f32,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoChunkRes {
    pub embedding: Vec<f32>,
    pub start: f32,
    pub end: f32,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HashResDocument {
    pub text: String,
    pub page_count: i32,
    pub parts: Vec<DocPartRes>,
    pub chunks: Vec<DocChunkRes>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocPartRes {
    pub text: String,
    pub page_number: i32,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocChunkRes {
    pub embedding: Vec<f32>,
    pub text: String,
    pub page_numbers: Vec<i32>,
}