use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum CheckMode {
    #[default]
    Normal,
    Precise,
}

impl std::fmt::Display for CheckMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckMode::Normal => write!(f, "normal"),
            CheckMode::Precise => write!(f, "precise"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskPayload {
    Autocheck {
        task_description: String,
        task_name: String,
        submission: String,
        criteria: String,
        email: String,
    },
    QuizGen {
        task_description: String,
        task_name: String,
        submission: String,
        email: String,
    }
}

impl TaskPayload {
    pub fn task_type(&self) -> &'static str {
        match self {
            TaskPayload::Autocheck { .. } => "autocheck",
            TaskPayload::QuizGen { .. } => "quiz_gen",
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct UniversalTaskRequest {
    pub payload: TaskPayload,
    #[serde(default)]
    pub mode: CheckMode,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AutocheckOutput {
    pub grade: f64,
    pub feedback: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct QuizGenOutput {
    pub questions: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SubmitResponse {
    pub task_id: Uuid,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ResultsRequest {
    pub task_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
pub struct ResultItem {
    pub task_id: Uuid,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
pub struct VerdictResponse {
    pub status: String,
    pub task_result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub mode: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
pub struct SubmissionTextResponse {
    pub submission_text: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SingleResponse {
    pub task_result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}