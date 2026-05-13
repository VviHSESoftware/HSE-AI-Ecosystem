use async_trait::async_trait;
use reqwest::{header, Client, multipart};
use serde::Deserialize;
use common_infra::AppError;
use crate::types::*;

#[async_trait]
pub trait DocProcessor: Send + Sync {
    async fn chunk_text(&self, req: ChunkTextRequest) -> Result<ChunkTextResponse, AppError>;
    async fn chunk_partitioned(&self, req: ChunkPartitionedTextRequest) -> Result<ChunkPartitionedTextResponse, AppError>;
    async fn parse_video(&self, video_url: &str) -> Result<ParseVideoResponse, AppError>;
    async fn parse_document(&self, filename: &str, file_bytes: Vec<u8>) -> Result<ParseDocumentResponse, AppError>;
    async fn get_supported_types(&self) -> Result<SupportedTypesResponse, AppError>;
}

pub struct DocProcessorClient {
    client: Client,
    base_url: String,
}

impl DocProcessorClient {
    pub fn new(base_url: String, token: String) -> Self {
        let mut headers = header::HeaderMap::new();
        let mut auth_val = header::HeaderValue::from_str(&format!("Bearer {}", token))
            .expect("Invalid Token");
        auth_val.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, auth_val);

        let client = Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(600))
            .build()
            .unwrap();

        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
        }
    }

    async fn handle_res<T: for<'de> serde::Deserialize<'de>>(&self, res: reqwest::Response) -> Result<T, AppError> {
        if res.status().is_success() {
            res.json::<T>().await.map_err(|e| AppError::Internal(format!("Serialization error: {}", e)))
        } else {
            let status = res.status();
            let detail = res.text().await.unwrap_or_default();
            Err(AppError::BadGateway(format!("DocProcessor Error ({}): {}", status, detail)))
        }
    }
}

#[async_trait]
impl DocProcessor for DocProcessorClient {
    async fn chunk_text(&self, req: ChunkTextRequest) -> Result<ChunkTextResponse, AppError> {
        let res = self.client.post(format!("{}/api/v1/chunkText", self.base_url))
            .json(&req).send().await?;
        self.handle_res(res).await
    }

    async fn chunk_partitioned(&self, req: ChunkPartitionedTextRequest) -> Result<ChunkPartitionedTextResponse, AppError> {
        let res = self.client.post(format!("{}/api/v1/chunkPartitionedText", self.base_url))
            .json(&req).send().await?;
        self.handle_res(res).await
    }

    async fn parse_video(&self, video_url: &str) -> Result<ParseVideoResponse, AppError> {
        let res = self.client.post(format!("{}/api/v1/parseVideo", self.base_url))
            .json(&serde_json::json!({ "video_url": video_url }))
            .send().await?;
        self.handle_res(res).await
    }

    async fn parse_document(&self, filename: &str, file_bytes: Vec<u8>) -> Result<ParseDocumentResponse, AppError> {
        let part = multipart::Part::bytes(file_bytes).file_name(filename.to_string());
        let form = multipart::Form::new().part("file", part);

        let res = self.client.post(format!("{}/api/v1/parseDocument", self.base_url))
            .multipart(form).send().await?;
        self.handle_res(res).await
    }

    async fn get_supported_types(&self) -> Result<SupportedTypesResponse, AppError> {
        let res = self.client.get(format!("{}/api/v1/getSupportedDocumentTypes", self.base_url))
            .send().await?;

        Ok(self.handle_res(res).await?)
    }
}