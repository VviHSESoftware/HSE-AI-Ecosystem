use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct AppEnv {
    pub api_tokens: HashSet<String>,
    pub fake_users: HashMap<String, Vec<i32>>,

    pub database_url: String,

    pub ai_gateway_url: String,
    pub ai_gateway_token: String,

    pub qdrant_url: String,
    pub qdrant_api_key: String,
}

impl AppEnv {
    pub fn load() -> Self {
        dotenvy::dotenv().ok();
        let tokens_str = std::env::var("API_TOKENS").unwrap_or_default();
        let api_tokens: HashSet<String> = tokens_str.split(',').map(|s| s.to_string()).collect();

        let fake_users_str = std::env::var("FAKEUSERS").unwrap_or_default();
        let mut fake_users = HashMap::new();

        for entry in fake_users_str.split(';') {
            let entry = entry.trim();
            if entry.is_empty() {
                continue;
            }

            if let Some((user, ids_str)) = entry.split_once(':') {
                let ids: Vec<i32> = ids_str
                    .split(',')
                    .filter_map(|s| s.trim().parse::<i32>().ok())
                    .collect();

                fake_users.insert(user.trim().to_string(), ids);
            }
        }

        Self {
            api_tokens,
            fake_users,
            database_url: std::env::var("DATABASE_URL").expect("DATABASE_URL is required"),
            ai_gateway_url: std::env::var("AI_GATEWAY_URL").expect("AI_GATEWAY_URL is required"),
            ai_gateway_token: std::env::var("AI_GATEWAY_TOKEN").expect("AI_GATEWAY_TOKEN is required"),
            qdrant_url: std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://qdrant:6333".into()),
            qdrant_api_key: std::env::var("QDRANT_API_KEY").unwrap_or_default(),
        }
    }
}