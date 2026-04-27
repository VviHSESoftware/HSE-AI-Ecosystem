use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;

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
    pub sub_modules: Vec<ModuleNode>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StructureRes {
    pub modules: Vec<ModuleNode>,
}