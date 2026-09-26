use std::collections::HashSet;
use std::fmt;
use std::sync::Arc;
use crate::{config::AppEnv, schemas::*};
use reqwest::Client;
use serde_json::{json, Value};
use regex::Regex;
use tracing::info;
use common_infra::HasApiTokens;
use ai_gateway_client::{AiGateway, AiGatewayClient, LLMRequest};

pub struct RouterState {
    pub env: AppEnv,
    pub client: Client,
    pub ai_gateway: Arc<dyn AiGateway>,
}

impl HasApiTokens for RouterState {
    fn get_api_tokens(&self) -> &HashSet<String> {
        &self.env.api_tokens
    }
}

impl RouterState {
    pub fn new(env: AppEnv) -> Self {
        let ai_gateway = Arc::new(AiGatewayClient::new(
            env.ai_gateway_url.clone(),
            env.ai_gateway_token.clone(),
        ));

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .unwrap();

        Self { env, client, ai_gateway }
    }

    pub fn extract_json_from_llm(&self, raw: &str) -> Result<Value, String> {
        info!("Raw res for parsing: {}", raw);
        let re = Regex::new(r"(?s)```(?:json)?\s*(\{.*?\})\s*```").unwrap();
        let clean_str = if let Some(caps) = re.captures(raw) {
            caps.get(1).unwrap().as_str()
        } else {
            raw.trim()
        };

        serde_json::from_str(clean_str).map_err(|e| format!("Failed to parse LLM JSON: {}. Raw: {}", e, raw))
    }

    pub async fn call_llm(
        &self,
        messages: Vec<ChatMessage>,
        mode: &str,
        temperature: f32,
    ) -> Result<String, String> {
        let req = LLMRequest {
            messages,
            mode: mode.to_string(),
            temperature: Some(temperature),
            ..Default::default()
        };

        let res = self.ai_gateway.chat_completion(req).await
            .map_err(|e| format!("Gateway error: {}", e))?;

        Ok(res.content)
    }

    pub async fn get_kb_structure(&self, email: &str) -> Result<Value, String> {
        let res = self.client.post(format!("{}/v1/availableStructure", self.env.kb_interface_url))
            .header("Authorization", format!("Bearer {}", self.env.kb_interface_token))
            .json(&json!({ "email": email }))
            .send().await.map_err(|e| e.to_string())?;

        if !res.status().is_success() { return Err("Failed to get KB structure".into()); }
        res.json().await.map_err(|e| e.to_string())
    }

    pub async fn query_kb(&self, endpoint: &str, payload: Value) -> Result<Vec<KbQueryChunk>, String> {
        let res = self.client.post(format!("{}/v1/{}", self.env.kb_interface_url, endpoint))
            .header("Authorization", format!("Bearer {}", self.env.kb_interface_token))
            .json(&payload)
            .send().await.map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let err = res.text().await.unwrap_or_default();
            return Err(format!("KB Query Error: {}", err));
        }

        let data: KbQueryRes = res.json().await.map_err(|e| e.to_string())?;
        Ok(data.chunks)
    }
}