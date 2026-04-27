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
pub struct AssignmentMessage {
    #[schema(example = "Write a Python function to sort a list.")]
    pub task_description: String,
    #[schema(example = "assignment_1_sort")]
    pub task_name: String,
    #[schema(example = "def sort_list(x): return sorted(x)")]
    pub submission: String,
    #[schema(example = "Correctness (100)")]
    pub criteria: String,
    #[schema(example = "student@hse.ru")]
    pub email: String,
    #[schema(example = "normal")]
    #[serde(default)]
    pub mode: CheckMode,
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
    pub status: String, // "processing", "completed", "error"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grade: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feedback: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
pub struct VerdictResponse {
    pub status: String,
    pub grade: Option<f64>,
    pub feedback: Option<String>,
    pub error: Option<String>,
    pub mode: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
pub struct SubmissionTextResponse {
    pub submission_text: String,
}