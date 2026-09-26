use reqwest::Client;
use serde_json::{json, Value};
use crate::config::AppEnv;

#[derive(Clone)]
pub struct KbClient {
    env: AppEnv,
    client: Client,
}

impl KbClient {
    pub fn new(env: AppEnv) -> Self {
        Self {
            env,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap(),
        }
    }

    async fn post(&self, endpoint: &str, payload: Value) -> Result<Value, String> {
        let url = format!("{}/{}", self.env.kb_url.trim_end_matches('/'), endpoint);
        let res = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", self.env.kb_system_token))
            .json(&payload)
            .send().await.map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            return Err(res.text().await.unwrap_or_default());
        }
        res.json().await.map_err(|e| e.to_string())
    }

    pub async fn add_module_base(
        &self,
        name: &str,
        url: &str,
        ext_id: &str,
        ext_type: &str,
        parent_id: i32,
    ) -> Result<i32, String> {
        let res = self.post("addModuleBase", json!({
            "system_name": self.env.kb_system_name,
            "name": name,
            "url": url,
            "external_id": ext_id,
            "external_type": ext_type,
            "parent_module_id": parent_id
        })).await?;
        res["module_id"]
            .as_i64()
            .map(|id| id as i32)
            .ok_or_else(|| format!("Invalid response: missing module_id in {}", res))
    }

    pub async fn invalidate_module(&self, ext_id: &str, ext_type: &str) -> Result<(), String> {
        let _ = self.post("invalidateModule", json!({
            "system_name": self.env.kb_system_name,
            "external_id": ext_id,
            "external_type": ext_type
        })).await;
        Ok(())
    }

    pub async fn set_text_type(&self, module_id: i32, text: &str) -> Result<(), String> {
        self.post("setTextType", json!({ "module_id": module_id, "text": text })).await?;
        Ok(())
    }

    pub async fn set_video_type(&self, module_id: i32, video_url: &str) -> Result<(), String> {
        self.post("setVideoType", json!({ "module_id": module_id, "video_url": video_url })).await?;
        Ok(())
    }
}