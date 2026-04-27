use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct AppEnv {
    pub project_name: String,
    pub app_version: String,
    pub api_tokens: HashSet<String>,

    pub gateway_base_url: String,
    pub gateway_api_token: String,
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
            project_name: std::env::var("PROJECT_NAME").unwrap_or_else(|_| "HSE Document Processor".into()),
            app_version: std::env::var("APP_VERSION").unwrap_or_else(|_| "1.0.0".into()),
            api_tokens,
            gateway_base_url: std::env::var("GATEWAY_API_URL").unwrap_or_else(|_| "http://localhost:8000".into()),
            gateway_api_token: std::env::var("GATEWAY_API_KEY").unwrap_or_default(),
            vlm_batch_size: std::env::var("VLM_BATCH_SIZE").unwrap_or_else(|_| "10".into()).parse().unwrap_or(10),
            dufs_url: std::env::var("DUFS_URL").unwrap_or_else(|_| "http://dufs:5000".into()),
            dufs_user: std::env::var("DUFS_USER").unwrap_or_default(),
            dufs_pass: std::env::var("DUFS_PASS").unwrap_or_default(),
        }
    }
}