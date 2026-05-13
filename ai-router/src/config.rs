use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct AppEnv {
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
            api_tokens,
            ai_gateway_url: std::env::var("AI_GATEWAY_URL").expect("AI_GATEWAY_URL is required"),
            ai_gateway_token: std::env::var("AI_GATEWAY_TOKEN").expect("AI_GATEWAY_TOKEN is required"),
            kb_interface_url: std::env::var("KB_INTERFACE_URL").expect("KB_INTERFACE_URL is required"),
            kb_interface_token: std::env::var("KB_INTERFACE_TOKEN").expect("KB_INTERFACE_TOKEN is required"),
            router_model_mode: std::env::var("ROUTER_MODEL_MODE").unwrap_or_else(|_| "fast".into()),
            generation_model_mode: std::env::var("GENERATION_MODEL_MODE").unwrap_or_else(|_| "normal".into()),
        }
    }
}