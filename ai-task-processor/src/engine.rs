use crate::schemas::{TaskPayload, AutocheckOutput, CheckMode, Submission};
use ai_gateway_client::{ChatMessage, LLMRequest};
use common_infra::AppError;
use serde_json::json;
use tracing::{error, info, warn};
use uuid::Uuid;
use crate::state::AppState;

#[derive(thiserror::Error, Debug)]
pub enum EngineError {
    #[error("AI Provider Throttling (429)")]
    RateLimit(String),
    #[error("AI Provider Infrastructure Error ({0})")]
    GatewayError(u16),
    #[error("Failed to parse AI response: {0}")]
    ParsingFailed(String),
    #[error("Internal System Error: {0}")]
    Internal(String),
}

impl From<EngineError> for AppError {
    fn from(err: EngineError) -> Self {
        match err {
            EngineError::RateLimit(_) => AppError::TooManyRequests("AI limit exceeded".into()),
            EngineError::GatewayError(code) => AppError::BadGateway(format!("AI Gateway returned {}", code)),
            EngineError::ParsingFailed(s) => AppError::Internal(format!("AI format invalid: {}", s)),
            EngineError::Internal(s) => AppError::Internal(s),
        }
    }
}

pub trait TaskProcessor: Send + Sync {
    fn build_prompt(&self, payload: &TaskPayload, system_prompt: &str) -> Result<Vec<ChatMessage>, EngineError>;
    fn parse_response(&self, raw: &str) -> Result<serde_json::Value, EngineError>;
    fn log_summary(&self, payload: &TaskPayload) -> String;
    fn response_format(&self) -> Option<serde_json::Value> {
        None
    }
}

pub struct AutocheckProcessor;

impl TaskProcessor for AutocheckProcessor {
    fn build_prompt(&self, payload: &TaskPayload, system_prompt: &str) -> Result<Vec<ChatMessage>, EngineError> {
        if let TaskPayload::Autocheck { task_description, submission, criteria, .. } = payload {
            let formatted_submission = match submission {
                Submission::Single(text) => text.clone(),
                Submission::Files(files) => {
                    let mut buf = String::new();
                    for file in files {
                        buf.push_str(&format!(
                            "--- File: {} ---\n{}\n\n",
                            file.filename, file.content
                        ));
                    }
                    buf
                }
            };

            let user_prompt = format!(
                "Task description:\n{}\nAssessment criteria:\n{}\nStudent's submission:\n{}",
                task_description, criteria, formatted_submission
            );
            Ok(vec![
                ChatMessage::system(system_prompt.to_string()),
                ChatMessage::user(user_prompt),
            ])
        } else {
            Err(EngineError::Internal("Wrong payload type".into()))
        }
    }

    fn response_format(&self) -> Option<serde_json::Value> {
        Some(json!({
            "type": "json_schema",
            "json_schema": {
                "name": "autocheck_output",
                "strict": true,
                "schema": {
                    "type": "object",
                    "properties": {
                        "feedback": {
                            "type": "string",
                            "description": "Развернутая обратная связь с баллами и пояснениями по каждому критерию"
                        },
                        "grade": {
                            "type": "number",
                            "description": "Итоговая суммарная оценка студента"
                        }
                    },
                    "required": ["feedback", "grade"],
                    "additionalProperties": false
                }
            }
        }))
    }

    fn parse_response(&self, response: &str) -> Result<serde_json::Value, EngineError> {
        let text = if let Some(idx) = response.rfind("</think>") {
            &response[idx + 8..]
        } else {
            response
        };

        let output: AutocheckOutput = serde_json::from_str(text.trim())
            .map_err(|e| EngineError::ParsingFailed(format!("JSON Error: {}. Raw: {}", e, text)))?;

        Ok(json!(output))
    }

    fn log_summary(&self, payload: &TaskPayload) -> String {
        if let TaskPayload::Autocheck { email, task_name, .. } = payload {
            format!("student <{}>, task <{}>", email, task_name)
        } else { "unknown autocheck".into() }
    }
}

pub struct QuizGenProcessor;

impl TaskProcessor for QuizGenProcessor {
    fn build_prompt(&self, payload: &TaskPayload, system_prompt: &str) -> Result<Vec<ChatMessage>, EngineError> {
        if let TaskPayload::QuizGen { submission, .. } = payload {
            let user_prompt = format!("Code for analysis:\n```\n{}\n```", submission);
            Ok(vec![
                ChatMessage::system(system_prompt.to_string()),
                ChatMessage::user(user_prompt),
            ])
        } else {
            Err(EngineError::Internal("Wrong payload type".into()))
        }
    }

    fn response_format(&self) -> Option<serde_json::Value> {
        Some(json!({
            "type": "json_schema",
            "json_schema": {
                "name": "quiz_gen_output",
                "strict": true,
                "schema": {
                    "type": "object",
                    "properties": {
                        "questions": {
                            "type": "array",
                            "description": "Список тестовых вопросов",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "question": {
                                        "type": "string",
                                        "description": "Текст вопроса"
                                    },
                                    "answers": {
                                        "type": "array",
                                        "description": "Варианты ответа (ровно 4)",
                                        "items": { "type": "string" }
                                    },
                                    "correct_answer": {
                                        "type": "integer",
                                        "description": "Индекс правильного ответа в массиве answers (0, 1, 2 или 3)"
                                    }
                                },
                                "required": ["question", "answers", "correct_answer"],
                                "additionalProperties": false
                            }
                        }
                    },
                    "required": ["questions"],
                    "additionalProperties": false
                }
            }
        }))
    }

    fn parse_response(&self, response: &str) -> Result<serde_json::Value, EngineError> {
        let text = if let Some(idx) = response.rfind("</think>") {
            &response[idx + 8..]
        } else {
            response
        };

        let parsed: serde_json::Value = serde_json::from_str(text.trim())
            .map_err(|e| EngineError::ParsingFailed(format!("JSON Error: {}. Raw: {}", e, text)))?;

        Ok(parsed)
    }

    fn log_summary(&self, payload: &TaskPayload) -> String {
        if let TaskPayload::QuizGen { email, task_name, .. } = payload {
            format!("student <{}>, task <{}>", email, task_name)
        } else { "unknown quizgen".into() }
    }
}

async fn do_ai_work(
    state: &AppState,
    processor: &dyn TaskProcessor,
    payload: &TaskPayload,
    mode: &CheckMode
) -> Result<serde_json::Value, EngineError> {
    let system_prompt = state.get_prompt(payload.task_type());

    let messages = processor.build_prompt(payload, system_prompt)?;
    let req = LLMRequest {
        messages,
        mode: mode.to_string(),
        temperature: Some(0.1),
        response_format: processor.response_format(),
        ..Default::default()
    };

    let res = state.ai_gateway.chat_completion(req).await
        .map_err(|e| EngineError::Internal(format!("Gateway error: {}", e)))?;

    processor.parse_response(&res.content)
}

pub async fn run_managed_task(
    state: &AppState,
    task_id: Uuid,
    payload: &TaskPayload,
    mode: &CheckMode
) -> Result<serde_json::Value, EngineError> {
    use tracing::Instrument;

    let start = std::time::Instant::now();
    let processor = get_processor(payload);
    let summary = processor.log_summary(payload);
    let task_type = payload.task_type();

    let span = tracing::info_span!("process_ai_task", %task_id, %summary, %task_type, mode = %mode);

    async move {
        state.repo.update_task_processing(task_id).await
            .map_err(|e| EngineError::Internal(format!("DB Error: {}", e)))?;

        info!("Task started");

        let result = do_ai_work(state, processor.as_ref(), payload, mode).await;
        let duration = start.elapsed().as_secs_f64();
        let mut status = "success";
        match &result {
            Ok(json_val) => {
                state.repo.update_task_completed(task_id, json_val.clone()).await
                    .map_err(|e| EngineError::Internal(format!("DB Update Error: {}", e)))?;
                info!("Task completed successfully");
            }
            Err(err) => {
                state.repo.update_task_error(task_id, &err.to_string()).await.ok();
                if err.is_retriable() {
                    warn!(reason = %err, "Task paused (retriable error), it remains in queue.");
                    status = "retry";
                } else {
                    error!(reason = %err, "Task failed");
                    status = "failure";
                }
            }
        }

        metrics::histogram!("task_duration_seconds", "status" => status, "mode" => mode.to_string(), "type" => task_type).record(duration);
        metrics::counter!("task_total", "status" => status, "mode" => mode.to_string(), "type" => task_type).increment(1);
        result
    }
    .instrument(span)
    .await
}

impl EngineError  {
    pub(crate) fn is_retriable(&self) -> bool {
        match self {
            Self::RateLimit(_) => true,
            Self::ParsingFailed(_) => true,
            Self::GatewayError(code) => *code >= 500,
            _ => false,
        }
    }
}

pub fn get_processor(payload: &TaskPayload) -> Box<dyn TaskProcessor> {
    match payload {
        TaskPayload::Autocheck { .. } => Box::new(AutocheckProcessor),
        TaskPayload::QuizGen { .. } => Box::new(QuizGenProcessor),
    }
}