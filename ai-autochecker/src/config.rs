use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct AppEnv {
    pub project_name: String,
    pub app_version: String,
    pub log_level: String,
    pub api_tokens: HashSet<String>,
    pub database_url: String,
    pub ai_gateway_url: String,
    pub ai_gateway_token: String,
    pub system_prompt_path: String,
}

impl AppEnv {
    pub fn load() -> Self {
        dotenvy::dotenv().ok();
        let tokens_str = std::env::var("API_TOKENS").unwrap_or_default();
        let api_tokens: HashSet<String> = tokens_str.split(',').map(|s| s.to_string()).collect();

        Self {
            project_name: std::env::var("PROJECT_NAME").unwrap_or_else(|_| "AutoCheck Service".into()),
            app_version: std::env::var("APP_VERSION").unwrap_or_else(|_| "1.0.0".into()),
            log_level: std::env::var("LOG_LEVEL").unwrap_or_else(|_| "INFO".into()),
            api_tokens,
            database_url: std::env::var("DATABASE_URL").expect("DATABASE_URL is required"),
            ai_gateway_url: std::env::var("AI_GATEWAY_URL").expect("AI_GATEWAY_URL is required"),
            ai_gateway_token: std::env::var("AI_GATEWAY_TOKEN").expect("AI_GATEWAY_TOKEN is required"),
            system_prompt_path: std::env::var("CHECKER_SYSTEM_PROMPT_PATH")
                .unwrap_or_else(|_| "prompts/system_submission_checker.txt".into()),
        }
    }
}