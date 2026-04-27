use axum::{routing::{get, post}, Router, Json};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::sleep;
use std::env;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let delay_ms = env::var("LLM_DELAY_MS").unwrap_or_else(|_| "500".to_string()).parse::<u64>().unwrap();
    let model_id = env::var("MODEL_ID").unwrap_or_else(|_| "mock-model".to_string());

    let app = Router::new()
        .route("/v1/models", get(move || async move {
            Json(json!({
                "object": "list",
                "data": [{ "id": model_id, "object": "model", "created": 1686935002, "owned_by": "mock" }, { "id": "mock-embedding-model", "object": "model", "created": 1686935002, "owned_by": "mock" }]
            }))
        }))
        .route("/v1/chat/completions", post(move |Json(payload): Json<Value>| async move {
            sleep(Duration::from_millis(delay_ms)).await;

            Json(json!({
                "id": "chatcmpl-mock",
                "object": "chat.completion",
                "created": 1677652288,
                "model": payload.get("model").unwrap_or(&json!("unknown")),
                "choices": [{
                    "index": 0,
                    "message": { "role": "assistant", "content": "This is a mock response from Rust provider." },
                    "finish_reason": "stop"
                }],
                "usage": { "prompt_tokens": 10, "completion_tokens": 20, "total_tokens": 30 }
            }))
        }))
        .route("/v1/embeddings", post(|| async {
            sleep(Duration::from_millis(100)).await;
            
            let embedding: Vec<f32> = vec![0.0; 4096];

            Json(json!({
                "object": "list",
                "data": [{ "object": "embedding", "index": 0, "embedding": embedding }],
                "model": "mock-embedding-model",
                "usage": { "prompt_tokens": 5, "total_tokens": 5 }
            }))
        }));

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("Mock AI Provider listening on {}", addr);
    axum::serve(listener, app).await.unwrap();
}