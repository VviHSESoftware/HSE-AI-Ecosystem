use async_trait::async_trait;
use reqwest::{header, Client, StatusCode};
use thiserror::Error;
use crate::types::*;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Gateway returned error: status {0}, body: {1}")]
    ApiError(StatusCode, String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Multipart error: {0}")]
    Multipart(String),
}

#[async_trait]
pub trait AiGateway: Send + Sync {
    async fn chat_completion(&self, req: LLMRequest) -> Result<LLMResponse, ClientError>;
    async fn create_embeddings(&self, text: String) -> Result<EmbeddingResponse, ClientError>;
    async fn vlm_analyze(&self, req: VLMRequest) -> Result<LLMResponse, ClientError>;
    async fn transcribe_audio(&self, filename: String, data: Vec<u8>) -> Result<serde_json::Value, ClientError>;
}

pub struct AiGatewayClient {
    client: Client,
    base_url: String,
}

impl AiGatewayClient {
    pub fn new(base_url: String, token: String) -> Self {
        let mut headers = header::HeaderMap::new();
        let mut auth_val = header::HeaderValue::from_str(&format!("Bearer {}", token))
            .expect("Invalid API token format");
        auth_val.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, auth_val);

        let client = Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .expect("Failed to build reqwest client");

        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
        }
    }
}

#[async_trait]
impl AiGateway for AiGatewayClient {

    /// LLM Text Generation
    async fn chat_completion(&self, req: LLMRequest) -> Result<LLMResponse, ClientError> {
        let url = format!("{}/v1/llm", self.base_url);
        self.post_json(url, req).await
    }

    /// Embedding generation
    async fn create_embeddings(&self, text: String) -> Result<EmbeddingResponse, ClientError> {
        let url = format!("{}/v1/embeddings", self.base_url);
        let req = EmbeddingRequest { input: text.into() };
        self.post_json(url, req).await
    }

    /// VLM image processing
    async fn vlm_analyze(&self, req: VLMRequest) -> Result<LLMResponse, ClientError> {
        let url = format!("{}/v1/vlm", self.base_url);
        self.post_json(url, req).await
    }

    /// ASR audio transcription
    async fn transcribe_audio(&self, filename: String, data: Vec<u8>) -> Result<serde_json::Value, ClientError> {
        let url = format!("{}/v1/asr", self.base_url);

        let part = reqwest::multipart::Part::bytes(data)
            .file_name(filename)
            .mime_str("audio/mpeg")
            .map_err(|e| ClientError::Multipart(e.to_string()))?;

        let form = reqwest::multipart::Form::new().part("file", part);

        let res = self.client.post(url)
            .multipart(form)
            .send()
            .await?;

        self.handle_response(res).await
    }
}

impl AiGatewayClient {
    async fn post_json<REQ, RES>(&self, url: String, req: REQ) -> Result<RES, ClientError>
    where
        REQ: serde::Serialize,
        RES: for<'de> serde::Deserialize<'de>
    {
        let res = self.client.post(url)
            .json(&req)
            .send()
            .await?;

        self.handle_response(res).await
    }

    async fn handle_response<RES>(&self, res: reqwest::Response) -> Result<RES, ClientError>
    where
        RES: for<'de> serde::Deserialize<'de>
    {
        let status = res.status();
        if status.is_success() {
            Ok(res.json::<RES>().await?)
        } else {
            let body = res.text().await.unwrap_or_default();
            Err(ClientError::ApiError(status, body))
        }
    }
}