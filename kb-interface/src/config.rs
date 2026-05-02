use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct AppEnv {
    pub api_tokens: HashSet<String>,

    pub database_url: String,

    pub gateway_url: String,
    pub gateway_token: String,

    pub qdrant_url: String,
    pub qdrant_api_key: String,
}

impl AppEnv {
    pub fn load() -> Self {
        dotenvy::dotenv().ok();
        let tokens_str = std::env::var("API_TOKENS").unwrap_or_default();
        let api_tokens: HashSet<String> = tokens_str.split(',').map(|s| s.to_string()).collect();

        Self {
            api_tokens,
            database_url: std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/kb_db".into()),
            gateway_url: std::env::var("GATEWAY_URL").unwrap_or_else(|_| "http://localhost:8000".into()),
            gateway_token: std::env::var("GATEWAY_TOKEN").unwrap_or_default(),
            qdrant_url: std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://qdrant:6333".into()),
            qdrant_api_key: std::env::var("QDRANT_API_KEY").unwrap_or_default(),
        }
    }
}