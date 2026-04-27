use crate::config::AppEnv;
use reqwest::Client;
use serde_json::json;
use sqlx::{PgPool, Row};

pub struct InterfaceState {
    pub env: AppEnv,
    pub db: PgPool,
    pub client: Client,
}

impl InterfaceState {
    pub fn new(env: AppEnv, db: PgPool) -> Self {
        Self {
            env,
            db,
            client: Client::builder().timeout(std::time::Duration::from_secs(30)).build().unwrap()
        }
    }

    pub async fn get_user_allowed_roots(&self, email: &str) -> Result<Vec<i32>, String> {
        let rows = sqlx::query(
            "SELECT module_id FROM module_access ma
             JOIN users u ON ma.user_id = u.id
             WHERE u.email = $1"
        )
            .bind(email)
            .fetch_all(&self.db).await.map_err(|e| e.to_string())?;

        Ok(rows.into_iter().map(|r| r.get("module_id")).collect())
    }

    pub async fn get_embedding(&self, text: &str) -> Result<Vec<f32>, String> {
        let res = self.client.post(format!("{}/v1/embeddings", self.env.gateway_url))
            .header("Authorization", format!("Bearer {}", self.env.gateway_token))
            .json(&json!({ "input": text }))
            .send().await.map_err(|e| e.to_string())?;

        let val: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
        let emb = val["embeddings"][0].as_array()
            .ok_or("Embedding failed")?
            .iter().map(|v| v.as_f64().unwrap_or(0.0) as f32).collect();
        Ok(emb)
    }

    pub async fn get_module_desc(&self, module_id: i32) -> String {
        let row = sqlx::query("SELECT path FROM modules WHERE id = $1").bind(module_id).fetch_one(&self.db).await;
        if let Ok(r) = row {
            let path_str: String = r.get::<Option<String>, _>("path").unwrap_or_default();
            let ids: Vec<i32> = path_str.split('/').filter(|s| !s.is_empty()).filter_map(|s| s.parse().ok()).collect();

            let m_rows = sqlx::query("SELECT external_type, name FROM modules WHERE id = ANY($1)")
                .bind(&ids).fetch_all(&self.db).await.unwrap_or_default();

            return m_rows.iter()
                .map(|mr| format!("{}: {}", mr.get::<Option<String>, _>("external_type").unwrap_or_else(|| "System".to_string()), mr.get::<String, _>("name")))
                .collect::<Vec<_>>().join(", ");
        }
        "Unknown".to_string()
    }
}