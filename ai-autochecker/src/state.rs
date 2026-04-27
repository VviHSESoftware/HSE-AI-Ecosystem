use crate::config::AppEnv;
use fang::asynk::async_queue::AsyncQueue;
use reqwest::Client;
use sqlx::PgPool;
use std::sync::{Arc, OnceLock};
use tracing::warn;

pub static GLOBAL_STATE: OnceLock<Arc<AppState>> = OnceLock::new();

pub struct AppState {
    pub env: AppEnv,
    pub db: PgPool,
    pub client: Client,
    pub queue: AsyncQueue,
    pub system_prompt: String,
}

impl AppState {
    pub fn new(env: AppEnv, db: PgPool, queue: AsyncQueue) -> Self {
        let system_prompt = std::fs::read_to_string(&env.system_prompt_path).unwrap_or_else(|err| {
            warn!("Failed to read system prompt from {}: {}. Using default fallback.", env.system_prompt_path, err);
            "Представь себя в роли опытного преподавателя. Твоя цель — выставить балл и дать обратную связь. В ПОСЛЕДНЕЙ СТРОКЕ твоего сообщения должно быть только ОДНО ЧИСЛО — итоговый балл студента.".to_string()
        });
        
        Self {
            env,
            db,
            client: Client::builder().timeout(std::time::Duration::from_secs(600)).build().unwrap(),
            queue,
            system_prompt,
        }
    }
}