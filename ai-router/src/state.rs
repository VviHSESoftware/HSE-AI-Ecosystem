use crate::{config::AppEnv, schemas::*};
use reqwest::Client;
use serde_json::{json, Value};
use regex::Regex;

pub struct RouterState {
    pub env: AppEnv,
    pub client: Client,
}

impl RouterState {
    pub fn new(env: AppEnv) -> Self {
        Self { env, client: Client::builder().timeout(std::time::Duration::from_secs(60)).build().unwrap() }
    }

    pub fn extract_json_from_llm(&self, raw: &str) -> Result<Value, String> {
        let re = Regex::new(r"```(?:json)?\s*(\{.*?\})\s*```").unwrap();
        let clean_str = if let Some(caps) = re.captures(raw) {
            caps.get(1).unwrap().as_str()
        } else {
            raw.trim()
        };

        serde_json::from_str(clean_str).map_err(|e| format!("Failed to parse LLM JSON: {}. Raw: {}", e, raw))
    }

    pub async fn call_llm(&self, messages: Vec<ChatMessage>, mode: &str, temperature: f32) -> Result<String, String> {
        let payload = json!({ "messages": messages, "mode": mode, "temperature": temperature });

        let res = self.client.post(format!("{}/v1/llm", self.env.ai_gateway_url))
            .header("Authorization", format!("Bearer {}", self.env.ai_gateway_token))
            .json(&payload)
            .send().await.map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            return Err(format!("Gateway Error: {}", res.text().await.unwrap_or_default()));
        }

        let data: Value = res.json().await.map_err(|e| e.to_string())?;
        data.pointer("/content")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| "Missing content in LLM response".to_string())
    }

    pub async fn get_kb_structure(&self, email: &str) -> Result<Value, String> {
        let res = self.client.post(format!("{}/api/v1/availableStructure", self.env.kb_interface_url))
            .header("Authorization", format!("Bearer {}", self.env.kb_interface_token))
            .json(&json!({ "email": email }))
            .send().await.map_err(|e| e.to_string())?;

        if !res.status().is_success() { return Err("Failed to get KB structure".into()); }
        res.json().await.map_err(|e| e.to_string())
    }

    pub async fn query_kb(&self, endpoint: &str, payload: Value) -> Result<Vec<KbQueryChunk>, String> {
        let res = self.client.post(format!("{}/api/v1/{}", self.env.kb_interface_url, endpoint))
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