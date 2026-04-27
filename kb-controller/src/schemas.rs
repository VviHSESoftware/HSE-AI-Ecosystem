use serde::{Deserialize, Serialize};
use utoipa::ToSchema;


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