use crate::config::{AppEnv, DynamicConfig, ModelSettings};
use crate::metrics::*;
use reqwest::{Client, Proxy};
use serde_json::{json};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{error, info};
use ai_gateway_client::{EmbeddingRequest, EmbeddingResponse, LLMResponse};
use common_infra::HasApiTokens;

pub struct GatewayState {
    pub config: DynamicConfig,
    pub clients: HashMap<String, Client>,
}

pub struct GatewayService {
    pub env: AppEnv,
    pub state: RwLock<Option<GatewayState>>,
}

impl HasApiTokens for GatewayService {
    fn get_api_tokens(&self) -> &HashSet<String> {
        &self.env.api_tokens
    }
}

impl GatewayService {
    pub fn new(env: AppEnv) -> Self {
        Self { env, state: RwLock::new(None) }
    }

    pub async fn update_config(&self, new_config: DynamicConfig) {
        let mut new_clients = HashMap::new();

        for p_cfg in &new_config.providers {
            let mut builder = Client::builder()
                .timeout(Duration::from_secs(600))
                .pool_max_idle_per_host(1000);

            if p_cfg.use_proxy {
                if let Some(ref url) = self.env.proxy_url {
                    if let Ok(proxy) = Proxy::all(url) { builder = builder.proxy(proxy); }
                }
            }

            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert("Authorization", format!("Bearer {}", p_cfg.api_key).parse()
                .expect("API key must be valid ASCII (checked in validate())"));
            builder = builder.default_headers(headers);

            if let Ok(client) = builder.build() {
                new_clients.insert(p_cfg.id.clone(), client);
            }
        }

        let mut state = self.state.write().await;
        *state = Some(GatewayState { config: new_config, clients: new_clients });
        info!("Config updated. Active providers loaded.");
    }

    pub async fn get_model_and_client(&self, mode_or_model: &str) -> Result<(ModelSettings, Client, String), String> {
        let guard = self.state.read().await;
        let state = guard.as_ref().ok_or("Config not loaded")?;

        let model_name = state.config.routing.get(mode_or_model)
            .map(|s| s.as_str())
            .unwrap_or(mode_or_model);

        let m_settings = state.config.models.iter()
            .find(|m| m.name == model_name)
            .ok_or_else(|| format!("Unknown mode or model '{}'", mode_or_model))?
            .clone();

        let client = state.clients.get(&m_settings.provider_id)
            .ok_or_else(|| format!("Client for provider '{}' not initialized", m_settings.provider_id))?
            .clone();

        let base_url = state.config.providers.iter()
            .find(|p| p.id == m_settings.provider_id)
            .expect("Referential integrity guaranteed by DynamicConfig::validate")
            .base_url.clone();

        Ok((m_settings, client, base_url))
    }

    pub async fn safe_request(
        &self, client: Client, url: String, payload: Option<serde_json::Value>,
        form: Option<reqwest::multipart::Form>, m_settings: &ModelSettings, mode: &str
    ) -> Result<serde_json::Value, String> {
        let start = std::time::Instant::now();
        info!(%mode, %url, "Request started");

        let mut req = client.post(&url);
        if let Some(json) = payload { req = req.json(&json); }
        if let Some(f) = form { req = req.multipart(f); }

        let res = req.send().await;
        let duration = start.elapsed().as_secs_f64();
        info!(%mode, %url, "Request ended {}", duration);
        metrics::histogram!(
            "ai_request_duration_seconds",
            "model" => m_settings.remote_model_id.clone(),
            "provider" => m_settings.provider_id.clone()
        ).record(duration);

        match res {
            Ok(response) => {
                let status = response.status();
                metrics::counter!(
                    "ai_requests_total",
                    "model" => m_settings.remote_model_id.clone(),
                    "mode" => mode.to_string(),
                    "provider" => m_settings.provider_id.clone(),
                    "status_code" => status.as_str().to_string()
                ).increment(1);

                if status.is_success() {
                    let data = response.text().await.map_err(|e| e.to_string())?;
                    match serde_json::from_str::<serde_json::Value>(&data) {
                        Ok(data) => {
                            if let Some(tokens) = data.pointer("/usage/total_tokens").and_then(|v| v.as_i64()) {
                                metrics::counter!(
                                    "ai_tokens_total",
                                    "model" => m_settings.remote_model_id.clone(),
                                    "mode" => mode.to_string(),
                                    "provider" => m_settings.provider_id.clone()
                                ).increment(tokens as u64);
                            }
                            Ok(data)
                        }
                        Err(e) => {
                            error!(
                                provider = %m_settings.provider_id,
                                error = %e,
                                body = %data,
                                "Failed to decode JSON response from provider"
                            );
                            Err(format!("Decoding error: {}. Body: {}", e, data))
                        }
                    }
                } else {
                    let err_text = response.text().await.unwrap_or_default();
                    error!(provider = %m_settings.provider_id, status = %status, "LLM Error: {}", err_text);
                    Err(format!("Provider HTTP {}: {}", status, err_text))
                }
            }
            Err(e) => {
                metrics::counter!(
                    "ai_requests_total",
                    "model" => m_settings.remote_model_id.clone(),
                    "mode" => mode.to_string(),
                    "provider" => m_settings.provider_id.clone(),
                    "status_code" => "error".to_string()
                ).increment(1);
                error!("Connection Error: {}", e);
                Err(e.to_string())
            }
        }
    }

    pub async fn get_status(&self) -> serde_json::Value {
        let guard = self.state.read().await;
        let state = match guard.as_ref() {
            Some(s) => s,
            None => return json!({ "config_loaded": false, "providers": [] }),
        };

        let mut tasks = vec![];

        for provider_cfg in &state.config.providers {
            let p_id = provider_cfg.id.clone();
            let url = format!("{}/models", provider_cfg.base_url);
            let client = state.clients.get(&p_id).cloned();

            tasks.push(tokio::spawn(async move {
                let client = match client {
                    Some(c) => c,
                    None => return (p_id, false, HashSet::new(), Some("Client not initialized".to_string())),
                };

                match client.get(&url).timeout(Duration::from_secs(5)).send().await {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            let body: serde_json::Value = resp.json().await.unwrap_or(json!({}));
                            let mut remote_ids = HashSet::new();
                            if let Some(data) = body.get("data").and_then(|d| d.as_array()) {
                                for model_obj in data {
                                    if let Some(id) = model_obj.get("id").and_then(|v| v.as_str()) {
                                        remote_ids.insert(id.to_string());
                                    }
                                }
                            }
                            (p_id, true, remote_ids, None)
                        } else {
                            let status = resp.status();
                            let err_msg = resp.text().await.unwrap_or_default();
                            (p_id, false, HashSet::new(), Some(format!("HTTP {}: {}", status, err_msg)))
                        }
                    }
                    Err(e) => (p_id, false, HashSet::new(), Some(e.to_string())),
                }
            }));
        }

        let mut remote_results = HashMap::new();
        for task in tasks {
            if let Ok((p_id, online, models, err)) = task.await {
                remote_results.insert(p_id, (online, models, err));
            }
        }

        let mut report = vec![];
        for p_cfg in &state.config.providers {
            let p_id = &p_cfg.id;
            let (online, available_remote_ids, error_msg) = remote_results.get(p_id)
                .cloned()
                .unwrap_or((false, HashSet::new(), Some("Not checked".to_string())));

            let models_in_config: Vec<_> = state.config.models.iter()
                .filter(|m| &m.provider_id == p_id)
                .map(|m| {
                    json!({
                        "internal_name": m.name,
                        "remote_model_id": m.remote_model_id,
                        "available": available_remote_ids.contains(&m.remote_model_id)
                    })
                })
                .collect();

            report.push(json!({
                "provider_id": p_id,
                "status": if online { "online" } else { "offline" },
                "error": error_msg,
                "checked_models": models_in_config
            }));
        }

        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64())
            .unwrap_or(0.0);

        json!({
            "config_loaded": true,
            "timestamp": now,
            "providers": report
        })
    }

    pub async fn warmup_loop(self: Arc<Self>) {
        info!("Warmup task started");
        loop {
            let status = self.get_status().await;
            update_gateway_status_metrics(&status);
            tokio::time::sleep(Duration::from_secs(20)).await;
        }
    }

    fn parse_llm_response(&self, raw: serde_json::Value) -> Result<LLMResponse, String> {
        if let Some(content) = raw.pointer("/choices/0/message/content").and_then(|v| v.as_str()) {
            return Ok(LLMResponse {
                content: content.to_string(),
                model: raw["model"].as_str().unwrap_or("unknown").to_string(),
                usage: raw["usage"].clone(),
            });
        }

        if let Some(outputs) = raw.get("output").and_then(|v| v.as_array()) {
            let message_block = outputs.iter().find(|item| item["type"] == "message");

            if let Some(block) = message_block {
                if let Some(text) = block.pointer("/content/0/text").and_then(|v| v.as_str()) {
                    return Ok(LLMResponse {
                        content: text.to_string(),
                        model: raw["model"].as_str().unwrap_or("unknown").to_string(),
                        usage: raw["usage"].clone(),
                    });
                }
            }
        }

        if let Some(content) = raw["content"].as_str() {
            return Ok(LLMResponse {
                content: content.to_string(),
                model: raw["model"].as_str().unwrap_or("unknown").to_string(),
                usage: raw["usage"].clone(),
            });
        }

        Err(format!("Could not extract message content from provider response. Raw: {}", raw))
    }

    pub async fn chat_completion(&self, mut payload: serde_json::Value, mode_or_model: &str) -> Result<LLMResponse, String> {
        let (m_settings, client, base_url) = self.get_model_and_client(mode_or_model).await?;

        if let Some(obj) = payload.as_object_mut() {
            obj.remove("mode");
        }

        let mut req_body = m_settings.extra_payload.clone();
        req_body["model"] = json!(m_settings.remote_model_id);
        req_body["temperature"] = json!(m_settings.temperature);

        let is_grok_responses_api = m_settings.provider_id == "grok";
        let url = if is_grok_responses_api {
            if let Some(msgs) = payload.get("messages") {
                req_body["input"] = msgs.clone();
            }

            req_body["store"]=json!(false);

            if let Some(max) = m_settings.max_tokens {
                req_body["max_output_tokens"] = json!(max);
            }

            if let Some(rf) = payload.get("response_format") {
                if let Some(rf_type) = rf.get("type").and_then(|v| v.as_str()) {
                    match rf_type {
                        "json_object" => {
                            req_body["text"] = json!({
                        "format": {
                            "type": "json_object"
                        }
                    });
                        }
                        "json_schema" => {
                            if let Some(js) = rf.get("json_schema") {
                                let mut format_obj = js.clone();
                                format_obj["type"] = json!("json_schema");
                                req_body["text"] = json!({
                            "format": format_obj
                        });
                            } else {
                                req_body["text"] = json!({
                            "format": rf
                        });
                            }
                        }
                        _ => {}
                    }
                }
            }

            format!("{}/responses", base_url)
        }else{
            if let Some(max) = m_settings.max_tokens { req_body["max_tokens"] = json!(max); }

            if let Some(obj) = req_body.as_object_mut() {
                if let Some(user_obj) = payload.as_object_mut() { obj.append(user_obj); }
            }

            format!("{}/chat/completions", base_url)
        };

        let raw = self.safe_request(client, url, Some(req_body), None, &m_settings, mode_or_model).await?;

        self.parse_llm_response(raw)
    }

    pub async fn create_embeddings(&self, req: EmbeddingRequest) -> Result<EmbeddingResponse, String> {
        let (m_settings, client, base_url) = self.get_model_and_client("embeddings").await?;
        let mut req_body = m_settings.extra_payload.clone();

        let model_id = if m_settings.provider_id == "jina" {
            m_settings.remote_model_id.strip_prefix("jina-ai/").unwrap_or(&m_settings.remote_model_id)
        } else {
            m_settings.remote_model_id.as_str()
        };

        req_body["model"] = json!(model_id);
        req_body["input"] = json!(req.input);
        if m_settings.provider_id == "jina" {
            req_body["task"] = json!("retrieval.".to_owned()+req.mode.as_str());
        }
        let raw = self.safe_request(client, format!("{}/embeddings", base_url), Some(req_body), None, &m_settings, "embeddings").await?;

        let mut embeddings = vec![];
        if let Some(data) = raw["data"].as_array() {
            for item in data {
                if let Some(arr) = item["embedding"].as_array() {
                    let vec_f32: Vec<f32> = arr.iter().filter_map(|v| v.as_f64())
                        .map(|v| v as f32)
                        .collect();
                    embeddings.push(vec_f32);
                }
            }
        }

        Ok(EmbeddingResponse { embeddings, model: raw["model"].as_str().unwrap_or("").to_string(), usage: raw["usage"].clone() })
    }

    pub async fn vlm_analyze(&self, req: crate::schemas::VLMRequest) -> Result<LLMResponse, String> {
        let (m_settings, client, base_url) = self.get_model_and_client("vlm").await?;
        let mut content = vec![
            json!({ "type": "text", "text": req.text })
        ];

        for url in req.image_urls {
            content.push(json!({"type": "image_url", "image_url": { "url": url }} ));
        }

        let req_body = json!({
            "model": m_settings.remote_model_id,
            "temperature": req.temperature.unwrap_or(m_settings.temperature),
            "max_tokens": req.max_tokens.or(m_settings.max_tokens),
            "messages": [{ "role": "user", "content": content }]
        });
        let raw = self.safe_request(client, format!("{}/chat/completions", base_url), Some(req_body), None, &m_settings, "vlm").await?;

        self.parse_llm_response(raw)
    }

    pub async fn transcribe_audio(&self, file_name: String, file_bytes: Vec<u8>) -> Result<serde_json::Value, String> {
        let (m_settings, client, base_url) = self.get_model_and_client("asr").await?;

        let part = reqwest::multipart::Part::bytes(file_bytes).file_name(file_name);
        let form = reqwest::multipart::Form::new()
            .text("model", m_settings.remote_model_id.clone())
            .text("response_format", "verbose_json")
            .text("language", "ru")
            .part("file", part);

        self.safe_request(client, format!("{}/audio/transcriptions", base_url), None, Some(form), &m_settings, "asr").await
    }
}