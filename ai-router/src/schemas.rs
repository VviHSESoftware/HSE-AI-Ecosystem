use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
pub use ai_gateway_client::{ChatMessage, LLMRequest};
#[derive(Debug, Deserialize, ToSchema)]
pub struct RouterRequest {
    pub messages: Vec<ChatMessage>,
    pub user_email: Option<String>,
    #[serde(default = "default_true")]
    pub integrate_links_in_text: bool,
    #[serde(default = "default_true")]
    pub use_markdown: bool,
    #[serde(default = "default_false")]
    pub voice_mode: bool,
}

fn default_true() -> bool { true }
fn default_false() -> bool { false }

#[derive(Debug, Serialize, ToSchema)]
pub struct SourceMaterial {
    pub title: String,
    pub url: String,
    pub r#type: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RouterResponse {
    pub content: String,
    pub sources: Vec<SourceMaterial>,
    pub module_used: String,
}

#[derive(Debug, Serialize)]
pub struct KbAvailableStructureReq {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct KbQueryChunk {
    pub desc: String,
    pub text: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct KbQueryRes {
    #[serde(default)]
    pub chunks: Vec<KbQueryChunk>,
}