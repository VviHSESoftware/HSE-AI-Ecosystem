use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};
use common_infra::AppError;
use crate::state::GatewayService;

#[utoipa::path(
    post,
    path = "/v1/chat/completions",
    responses((status = 200, description = "OpenAI Chat Completion")),
    security(("bearerAuth" = []))
)]
pub async fn openai_chat(
    State(state): State<Arc<GatewayService>>,
    Json(payload): Json<Value>
) -> Result<Json<Value>, AppError> {
    let target = payload.get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("normal")
        .to_string();

    let response = state.chat_completion(payload, &target).await?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let openai_response = json!({
        "id": format!("chatcmpl-{}", uuid::Uuid::new_v4().simple()),
        "object": "chat.completion",
        "created": now,
        "model": response.model,
        "choices": [
            {
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": response.content
                },
                "finish_reason": "stop"
            }
        ],
        "usage": response.usage
    });

    Ok(Json(openai_response))
}

#[utoipa::path(get, path = "/v1/models", responses((status = 200, description = "OpenAI model list")), security(("bearerAuth" = [])))]
pub async fn list_models(State(state): State<Arc<GatewayService>>, ) -> Result<Json<Value>, AppError> {
    let guard = state.state.read().await;
    let state_ref = guard.as_ref().ok_or_else(|| AppError::Internal("Config not loaded".into()))?;

    let mut data = vec![];

    for (mode_name, target_model) in &state_ref.config.routing {
        data.push(json!({
            "id": mode_name,
            "object": "model",
            "owned_by": "gateway-routing",
            "root": target_model
        }));
    }

    for model in &state_ref.config.models {
        data.push(json!({
            "id": model.name,
            "object": "model",
            "owned_by": model.provider_id,
            "root": model.remote_model_id
        }));
    }

    Ok(Json(json!({
        "object": "list",
        "data": data
    })))
}