use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct AppEnv {
    pub project_name: String,
    pub app_version: String,
    pub api_tokens: HashSet<String>,

    pub ai_gateway_url: String,
    pub ai_gateway_token: String,

    pub kb_interface_url: String,
    pub kb_interface_token: String,

    pub router_model_mode: String,
    pub generation_model_mode: String,
}

impl AppEnv {
    pub fn load() -> Self {
        dotenvy::dotenv().ok();
        let tokens_str = std::env::var("API_TOKENS").unwrap_or_default();
        let api_tokens: HashSet<String> = tokens_str.split(',').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect();

        Self {
            project_name: std::env::var("PROJECT_NAME").unwrap_or_else(|_| "HSE AI Router".into()),
            app_version: std::env::var("APP_VERSION").unwrap_or_else(|_| "1.0.0".into()),
            api_tokens,
            ai_gateway_url: std::env::var("AI_GATEWAY_URL").unwrap_or_else(|_| "http://ai-gateway:8000/v1".into()),
            ai_gateway_token: std::env::var("AI_GATEWAY_TOKEN").unwrap_or_default(),
            kb_interface_url: std::env::var("KB_INTERFACE_URL").unwrap_or_else(|_| "http://kb-interface:8003/api/v1".into()),
            kb_interface_token: std::env::var("KB_INTERFACE_TOKEN").unwrap_or_default(),
            router_model_mode: std::env::var("ROUTER_MODEL_MODE").unwrap_or_else(|_| "fast".into()),
            generation_model_mode: std::env::var("GENERATION_MODEL_MODE").unwrap_or_else(|_| "normal".into()),
        }
    }
}