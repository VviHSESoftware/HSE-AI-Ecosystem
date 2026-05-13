use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct AppEnv {
    pub api_tokens: HashSet<String>,

    pub ai_gateway_url: String,
    pub ai_gateway_token: String,
    pub vlm_batch_size: usize,

    pub dufs_url: String,
    pub dufs_user: String,
    pub dufs_pass: String,
}

impl AppEnv {
    pub fn load() -> Self {
        dotenvy::dotenv().ok();
        let tokens_str = std::env::var("API_TOKENS").unwrap_or_default();
        let api_tokens: HashSet<String> = tokens_str.split(',').map(|s| s.to_string()).collect();

        Self {
            api_tokens,
            ai_gateway_url: std::env::var("AI_GATEWAY_URL").expect("AI_GATEWAY_URL is required"),
            ai_gateway_token: std::env::var("AI_GATEWAY_TOKEN").expect("AI_GATEWAY_TOKEN is required"),
            vlm_batch_size: std::env::var("VLM_BATCH_SIZE").unwrap_or_else(|_| "10".into()).parse().unwrap_or(10),
            dufs_url: std::env::var("DUFS_URL").unwrap_or_else(|_| "http://dufs:5000".into()),
            dufs_user: std::env::var("DUFS_USER").unwrap_or_default(),
            dufs_pass: std::env::var("DUFS_PASS").unwrap_or_default(),
        }
    }
}