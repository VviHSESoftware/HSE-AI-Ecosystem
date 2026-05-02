use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use axum::http::HeaderValue;

#[derive(Debug, Clone)]
pub struct AppEnv {
    pub api_tokens: HashSet<String>,
    pub proxy_url: Option<String>,
}

impl AppEnv {
    pub fn load() -> Self {
        dotenvy::dotenv().ok();
        let tokens_str = std::env::var("API_TOKENS").unwrap_or_default();
        let api_tokens: HashSet<String> = tokens_str.split(',').map(|s| s.to_string()).collect();

        Self {
            api_tokens,
            proxy_url: std::env::var("PROXY_URL").ok().filter(|s| !s.is_empty()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProviderConfig {
    pub id: String,
    pub base_url: String,
    pub api_key: String,
    #[serde(default)]
    pub use_proxy: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ModelSettings {
    pub name: String,
    pub provider_id: String,
    pub remote_model_id: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    pub max_tokens: Option<i32>,
    #[serde(default)]
    pub extra_payload: serde_json::Value,
}

fn default_temperature() -> f32 { 0.7 }

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DynamicConfig {
    pub providers: Vec<ProviderConfig>,
    pub models: Vec<ModelSettings>,
    pub routing: HashMap<String, String>,
}

impl DynamicConfig {
    pub fn load_from_file() -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string("config.json")?;
        let config: DynamicConfig = serde_json::from_str(&content)?;

        config.validate()?;

        Ok(config)
    }

    pub fn validate(&self) -> Result<(), String> {
        let provider_ids: HashSet<_> = self.providers.iter().map(|p| &p.id).collect();

        if self.models.is_empty() {
            return Err("Config error: 'models' list cannot be empty".into());
        }

        for provider in &self.providers {
            if HeaderValue::from_str(&format!("Bearer {}", provider.api_key)).is_err() {
                return Err(format!("Provider '{}' has invalid API key (contains illegal characters)", provider.id));
            }
        }

        let mut model_names = HashSet::new();
        for model in &self.models {
            if !provider_ids.contains(&model.provider_id) {
                return Err(format!(
                    "Config error: Model '{}' refers to unknown provider_id '{}'",
                    model.name, model.provider_id
                ));
            }
            model_names.insert(&model.name);
        }

        for (key, target_model) in &self.routing {
            if !model_names.contains(target_model) {
                return Err(format!(
                    "Config error: Routing key '{}' refers to unknown model '{}'",
                    key, target_model
                ));
            }
        }

        Ok(())
    }
}