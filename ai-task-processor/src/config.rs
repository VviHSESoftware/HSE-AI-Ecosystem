use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct AppEnv {
    pub api_tokens: HashSet<String>,
    pub database_url: String,
    pub ai_gateway_url: String,
    pub ai_gateway_token: String,
    pub prompts_dir: String,
}

impl AppEnv {
    pub fn load() -> Self {
        dotenvy::dotenv().ok();
        let tokens_str = std::env::var("API_TOKENS").unwrap_or_default();
        let api_tokens: HashSet<String> = tokens_str.split(',').map(|s| s.to_string()).collect();

        Self {
            api_tokens,
            database_url: std::env::var("DATABASE_URL").expect("DATABASE_URL is required"),
            ai_gateway_url: std::env::var("AI_GATEWAY_URL").expect("AI_GATEWAY_URL is required"),
            ai_gateway_token: std::env::var("AI_GATEWAY_TOKEN").expect("AI_GATEWAY_TOKEN is required"),
            prompts_dir: std::env::var("PROMPTS_DIR").unwrap_or_else(|_| "./prompts".into()),
        }
    }
}