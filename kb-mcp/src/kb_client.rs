use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use reqwest::Client;
use serde_json::json;

#[derive(Clone)]
pub struct KbClient {
    client: Client,
    base_url: String,
}

impl KbClient {
    pub fn new(base_url: String, kb_token: String) -> Self {
        let mut headers = HeaderMap::new();
        let auth_val = HeaderValue::from_str(&format!("Bearer {}", kb_token))
            .expect("Invalid KB token format");
        headers.insert(AUTHORIZATION, auth_val);

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .expect("Failed to build HTTP client");

        Self { client, base_url }
    }

    pub async fn semantic_query(
        &self,
        email: &str,
        query: String,
        module_ids: Vec<i32>,
        keywords: Vec<String>,
    ) -> Result<serde_json::Value, String> {
        let payload = json!({
            "email": email,
            "semantic_query": query,
            "module_ids": module_ids,
            "keywords": keywords
        });

        self.post("/v1/semanticQuery", payload).await
    }

    pub async fn timed_video_query(
        &self,
        email: &str,
        module_id: i32,
        second_start: f64,
        second_end: f64,
    ) -> Result<serde_json::Value, String> {
        let payload = json!({
            "email": email,
            "module_id": module_id,
            "second_start": second_start,
            "second_end": second_end
        });

        self.post("/v1/timedVideoQuery", payload).await
    }

    pub async fn document_page_query(
        &self,
        email: &str,
        module_id: i32,
        page: i32,
    ) -> Result<serde_json::Value, String> {
        let payload = json!({
            "email": email,
            "module_id": module_id,
            "page": page
        });

        self.post("/v1/documentPageQuery", payload).await
    }

    pub async fn full_content_query(
        &self,
        email: &str,
        module_ids: Vec<i32>,
    ) -> Result<serde_json::Value, String> {
        let payload = json!({
            "email": email,
            "module_ids": module_ids
        });

        self.post("/v1/fullContentQuery", payload).await
    }

    pub async fn available_structure(
        &self,
        email: &str,
        module_ids: Option<Vec<i32>>,
    ) -> Result<serde_json::Value, String> {
        let payload = json!({
            "email": email,
            "module_ids": module_ids
        });

        self.post("/v1/availableStructure", payload).await
    }

    async fn post(&self, endpoint: &str, payload: serde_json::Value) -> Result<serde_json::Value, String> {
        let res = self
            .client
            .post(format!("{}{}", self.base_url, endpoint))
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(format!("Downstream error {}: {}", status, body));
        }

        res.json::<serde_json::Value>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    }
}