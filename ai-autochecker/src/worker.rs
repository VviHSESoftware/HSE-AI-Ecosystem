use fang::asynk::async_queue::AsyncQueueable;
use fang::AsyncRunnable;
use fang::FangError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use tracing::{info, error};

use crate::schemas::{AssignmentMessage, CheckMode};
use crate::state::GLOBAL_STATE;
use serde_json::json;
use tracing::log::warn;

#[derive(thiserror::Error, Debug)]
pub enum CheckError {
    #[error("AI Provider Throttling (429)")]
    RateLimit(String),
    #[error("AI Provider Infrastructure Error ({0})")]
    GatewayError(u16),
    #[error("Submission too large: {0} tokens")]
    ContextExceeded(usize),
    #[error("Failed to parse AI response: {0}")]
    ParsingFailed(String),
    #[error("Internal System Error: {0}")]
    Internal(String),
}

impl CheckError {
    fn is_retriable(&self) -> bool {
        match self {
            Self::RateLimit(_) => true,
            Self::GatewayError(code) => *code >= 500,
            _ => false,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "fang::serde")]
pub struct CheckJob {
    pub task_id: Uuid,
    pub payload: AssignmentMessage,
}

#[typetag::serde]
#[fang::async_trait]
impl AsyncRunnable for CheckJob {
    async fn run(&self, _queue: &mut dyn AsyncQueueable) -> Result<(), FangError> {
        let start = std::time::Instant::now();
        let state = GLOBAL_STATE.get().expect("Global state not initialized");

        let email = &self.payload.email;
        let task_id = self.task_id;
        let mode_label = self.payload.mode.to_string();

        let _span = tracing::info_span!("check_submission", %task_id, %email, mode=%mode_label).entered();

        info!("Starting evaluation for student <{}>, task <{}>, id <{}>", email, self.payload.task_name, task_id);

        if let Err(e) = sqlx::query("UPDATE submission_results SET status = 'processing' WHERE task_id = $1")
            .bind(task_id)
            .execute(&state.db).await
        {
            error!("Failed to update status to processing: {}", e);
            return Ok(());
        }

        let result = run_ai_check(&self.payload, state).await;

        let duration = start.elapsed().as_secs_f64();

        match result {
            Ok((grade, feedback)) => {
                info!("Task ID: {} completed successfully. Grade: {}", task_id, grade);
                if let Err(e) = sqlx::query("UPDATE submission_results SET status = 'completed', grade = $1, feedback = $2 WHERE task_id = $3")
                    .bind(grade).bind(feedback).bind(task_id)
                    .execute(&state.db).await
                {
                    error!("Failed to save completed result: {}", e);
                }

                metrics::histogram!("submission_check_duration_seconds", "status" => "success", "mode" => mode_label.clone()).record(duration);
                metrics::counter!("submission_checks_total", "status" => "success", "mode" => mode_label).increment(1);
                Ok(())
            }
            Err(err) => {
                let error_detail = err.to_string();
                let retriable = err.is_retriable();

                if retriable {
                    warn!(
                        status = "transient_failure", reason = %error_detail, retry = true,
                        "Check for <{}> paused. Task remains in queue.", email
                    );

                    metrics::histogram!("submission_check_duration_seconds", "status" => "retry", "mode" => mode_label.clone()).record(duration);
                    metrics::counter!("submission_checks_total", "status" => "retry", "mode" => mode_label).increment(1);

                    Err(FangError {
                        description: format!("Retryable failure for {}: {}", email, error_detail),
                    })
                } else {
                    error!(
                        status = "permanent_failure", reason = %error_detail, retry = false,
                        "Check for <{}> FAILED. No further retries.", email
                    );

                    metrics::histogram!("submission_check_duration_seconds", "status" => "failed", "mode" => mode_label.clone()).record(duration);
                    metrics::counter!("submission_checks_total", "status" => "failed_permanent", "mode" => mode_label).increment(1);

                    if let Err(e) = sqlx::query("UPDATE submission_results SET status = 'error', error = $1 WHERE task_id = $2")
                        .bind(&error_detail).bind(task_id)
                        .execute(&state.db).await
                    {
                        error!("Failed to update status to error: {}", e);
                    }

                    Ok(())
                }
            }
        }
    }

    fn task_type(&self) -> String {
        match self.payload.mode {
            CheckMode::Normal => "autocheck_normal".to_string(),
            CheckMode::Precise => "autocheck_precise".to_string(),
        }
    }
}

async fn run_ai_check(payload: &AssignmentMessage, state: &crate::state::AppState) -> Result<(f64, String), CheckError> {
    let estimated_tokens = (payload.submission.len() + payload.task_description.len()) / 4;

    if estimated_tokens > 100_000 {
        return Err(CheckError::ContextExceeded(estimated_tokens));
    }

    let user_prompt = format!(
        "Task description:\n{}\nAssessment criteria:\n{}\nStudent's submission:\n{}",
        payload.task_description, payload.criteria, payload.submission
    );

    let req_body = json!({
        "mode": payload.mode.to_string(),
        "temperature": 0.1,
        "messages":[
            {"role": "system", "content": state.system_prompt},
            {"role": "user", "content": user_prompt}
        ]
    });

    let res = state.client.post(&format!("{}/llm", state.env.ai_gateway_url))
        .header("Authorization", format!("Bearer {}", state.env.ai_gateway_token))
        .json(&req_body)
        .send()
        .await
        .map_err(|e| CheckError::Internal(format!("Network/Connection error: {}", e)))?;

    let status = res.status();

    metrics::counter!("ai_gateway_responses_total", "status_code" => status.as_u16().to_string(), "mode" => payload.mode.to_string()).increment(1);

    if !status.is_success() {
        return match status.as_u16() {
            429 => Err(CheckError::RateLimit("AI Provider is throttled".into())),
            500..=599 => Err(CheckError::GatewayError(status.as_u16())),
            _ => Err(CheckError::Internal(format!("Gateway returned error status: {}", status))),
        };
    }

    let res_json: serde_json::Value = res.json().await
        .map_err(|e| CheckError::Internal(format!("Failed to decode JSON: {}", e)))?;

    let content = res_json.pointer("/content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CheckError::ParsingFailed("Missing 'content' field in AI response".into()))?;

    parse_llm_response(content)
}

fn parse_llm_response(response: &str) -> Result<(f64, String), CheckError> {
    let re = regex::Regex::new(r"(?s)<think>.*?</think>").unwrap();
    let cleaned = re.replace_all(response, "").to_string();
    let cleaned = cleaned.trim();

    let lines: Vec<&str> = cleaned.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();

    if lines.is_empty() {
        return Err(CheckError::ParsingFailed("AI returned an empty response".into()));
    }

    let last_line = lines.last().unwrap();

    let re_num = regex::Regex::new(r"-?\d+\.?\d*").unwrap();

    let grade = re_num.find(last_line)
        .ok_or_else(|| CheckError::ParsingFailed(format!("No grade found in last line: '{}'", last_line)))?
        .as_str()
        .parse::<f64>()
        .map_err(|_| CheckError::ParsingFailed("Failed to parse grade as number".into()))?;

    let feedback = lines[..lines.len() - 1].join("\n");
    Ok((grade, feedback))
}