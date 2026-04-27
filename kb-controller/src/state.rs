use crate::{config::AppEnv, schemas::*};
use reqwest::{Client, multipart};
use sqlx::PgPool;
use serde_json::json;
use sqlx::Row;
use sha2::{Sha256, Digest};
use bincode;
use tracing::info;

pub struct KbControllerState {
    pub env: AppEnv,
    pub db: PgPool,
    pub client: Client,
}

impl KbControllerState {
    pub async fn new(env: AppEnv, db: PgPool) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(600))
            .build()
            .unwrap();
        Self { env, db, client }
    }

    fn calculate_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    pub async fn build_hierarchy_description(&self, ids: &[i32]) -> String {
        if ids.is_empty() {
            return String::new();
        }

        let rows = sqlx::query("SELECT id, external_type, name FROM modules WHERE id = ANY($1)")
            .bind(ids)
            .fetch_all(&self.db)
            .await
            .unwrap_or_default();

        let mut map: std::collections::HashMap<i32, (String, String)> = rows.into_iter()
            .map(|r| {
                let id: i32 = r.get("id");
                let ext_type = r.get::<Option<String>, _>("external_type")
                    .unwrap_or_else(|| "System".to_string());
                let name = r.get::<String, _>("name");
                (id, (ext_type, name))
            })
            .collect();

        ids.iter()
            .filter_map(|id| map.remove(id))
            .map(|(ext_type, name)| format!("{}: {}", ext_type, name))
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub async fn check_access(&self, system_id: i32, module_id: i32) -> Result<(), String> {
        let prefix = format!("/{}", system_id);
        let row = sqlx::query("SELECT path FROM modules WHERE id = $1")
            .bind(module_id).fetch_optional(&self.db).await.map_err(|e| e.to_string())?;

        if let Some(r) = row {
            let path: String = Row::get(&r, "path");
            if path == prefix || path.starts_with(&format!("{}/", prefix)) {
                return Ok(());
            }
        }
        Err("Access denied: Module does not belong to your system".into())
    }

    pub async fn get_module_id_secure(&self, system_id: i32, ext_id: &str, ext_type: &str) -> Result<i32, String> {
        let prefix = format!("/{}%", system_id);

        let row = sqlx::query("SELECT id FROM modules WHERE external_id = $1 AND external_type = $2 AND path LIKE $3")
            .bind(ext_id).bind(ext_type).bind(prefix)
            .fetch_optional(&self.db).await.map_err(|e| e.to_string())?;

        match row {
            Some(r) => Ok(sqlx::Row::get(&r, "id")),
            None => Err("Module not found in your system scope".into())
        }
    }

    async fn upsert_to_qdrant(&self, points: serde_json::Value) -> Result<(), String> {
        let url = format!("{}/collections/kb_chunks/points?wait=true", self.env.qdrant_url.trim_end_matches('/'));

        let mut request = self.client.put(url)
            .json(&json!({ "points": points }));

        if !self.env.qdrant_api_key.is_empty() {
            request = request.header("api-key", &self.env.qdrant_api_key);
        }

        let resp = request.send().await.map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_body = resp.text().await.unwrap_or_default();
            return Err(format!("Qdrant error (Status {}): {}", status, err_body));
        }

        Ok(())
    }

    pub async fn delete_subtree_from_qdrant(&self, module_id: i32) -> Result<(), String> {
        let url = format!("{}/collections/kb_chunks/points/delete", self.env.qdrant_url.trim_end_matches('/'));

        let payload = json!({
            "filter": {
                "must": [
                    {
                        "key": "parent_ids",
                        "match": { "value": module_id }
                    }
                ]
            }
        });

        let mut request = self.client.post(url).json(&payload);

        if !self.env.qdrant_api_key.is_empty() {
            request = request.header("api-key", &self.env.qdrant_api_key);
        }

        let resp = request.send().await.map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            let err_body = resp.text().await.unwrap_or_default();
            return Err(format!("Qdrant delete error: {}", err_body));
        }

        Ok(())
    }

    async fn get_parent_ids_from_db(&self, module_id: i32) -> Vec<i32> {
        let row = sqlx::query("SELECT path FROM modules WHERE id = $1")
            .bind(module_id)
            .fetch_one(&self.db).await;

        match row {
            Ok(r) => {
                let path_str = r.get::<Option<String>, _>("path").unwrap_or_default();
                path_str.split('/')
                    .filter(|s| !s.is_empty())
                    .filter_map(|s| s.parse::<i32>().ok())
                    .collect()
            },
            Err(_) => vec![module_id]
        }
    }

    pub async fn persist_text_module(&self, module_id: i32, text: String, res: HashResText) -> Result<(), String> {
        let parent_ids = self.get_parent_ids_from_db(module_id).await;
        let description = self.build_hierarchy_description(&parent_ids).await;
        let mut tx = self.db.begin().await.map_err(|e| e.to_string())?;

        sqlx::query("INSERT INTO module_text (module_id, content)
                     VALUES ($1, $2)
                     ON CONFLICT (module_id) DO UPDATE SET content = EXCLUDED.content")
            .bind(module_id).bind(text).execute(&mut *tx).await.map_err(|e| e.to_string())?;

        let mut qdrant_points = vec![];
        for chunk in res.chunks {
            qdrant_points.push(json!({
                "id": uuid::Uuid::new_v4().to_string(),
                "vector": chunk.embedding,
                "payload": {
                    "module_id": module_id,
                    "parent_ids": parent_ids,
                    "description": description,
                    "type": "text",
                    "text": chunk.text
                }
            }));
        }

        if !qdrant_points.is_empty() { self.upsert_to_qdrant(json!(qdrant_points)).await?; }

        sqlx::query("UPDATE modules SET type = 'Text' WHERE id = $1")
            .bind(module_id).execute(&mut *tx).await.map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn persist_video_module(&self, module_id: i32, res: HashResVideo) -> Result<(), String> {
        let parent_ids = self.get_parent_ids_from_db(module_id).await;
        let description = self.build_hierarchy_description(&parent_ids).await;
        let mut tx = self.db.begin().await.map_err(|e| e.to_string())?;

        let total_time = res.parts.last().map(|p| p.end).unwrap_or(0.0);

        sqlx::query("INSERT INTO module_video (module_id, transcription, total_time) VALUES ($1, $2, $3)
                     ON CONFLICT (module_id) DO UPDATE SET transcription = EXCLUDED.transcription, total_time = EXCLUDED.total_time")
            .bind(module_id).bind(&res.text).bind(total_time).execute(&mut *tx).await.map_err(|e| e.to_string())?;

        sqlx::query("DELETE FROM video_chunks WHERE module_id = $1")
            .bind(module_id).execute(&mut *tx).await.map_err(|e| e.to_string())?;

        let mut qdrant_points = vec![];
        for chunk in res.chunks {
            qdrant_points.push(json!({
                "id": uuid::Uuid::new_v4().to_string(),
                "vector": chunk.embedding,
                "payload": {
                    "module_id": module_id,
                    "parent_ids": parent_ids,
                    "description": description,
                    "type": "video",
                    "text": chunk.text,
                    "start": chunk.start,
                    "end": chunk.end
                }
            }));
        }

        if !qdrant_points.is_empty() { self.upsert_to_qdrant(json!(qdrant_points)).await?; }

        for part in res.parts {
            sqlx::query("INSERT INTO video_chunks (module_id, start_time, end_time, content) VALUES ($1, $2, $3, $4)")
                .bind(module_id).bind(part.start).bind(part.end).bind(part.text).execute(&mut *tx).await.map_err(|e| e.to_string())?;
        }

        sqlx::query("UPDATE modules SET type = 'Video' WHERE id = $1")
            .bind(module_id).execute(&mut *tx).await.map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn persist_file_module(&self, module_id: i32, filename: String, res: HashResDocument) -> Result<(), String> {
        let parent_ids = self.get_parent_ids_from_db(module_id).await;
        let description = self.build_hierarchy_description(&parent_ids).await;
        let mut tx = self.db.begin().await.map_err(|e| e.to_string())?;

        sqlx::query("INSERT INTO module_file (module_id, content, filename, page_count) VALUES ($1, $2, $3, $4)
                     ON CONFLICT (module_id) DO UPDATE SET content = EXCLUDED.content, filename = EXCLUDED.filename, page_count = EXCLUDED.page_count")
            .bind(module_id).bind(&res.text).bind(filename).bind(res.page_count).execute(&mut *tx).await.map_err(|e| e.to_string())?;

        sqlx::query("DELETE FROM module_file_pages WHERE module_id = $1")
            .bind(module_id)
            .execute(&mut *tx).await.map_err(|e| e.to_string())?;

        let mut qdrant_points = vec![];
        for chunk in res.chunks {
            qdrant_points.push(json!({
                "id": uuid::Uuid::new_v4().to_string(),
                "vector": chunk.embedding,
                "payload": {
                    "module_id": module_id,
                    "parent_ids": parent_ids,
                    "description": description,
                    "type": "file",
                    "text": chunk.text,
                    "page_numbers": chunk.page_numbers
                }
            }));
        }

        if !qdrant_points.is_empty() { self.upsert_to_qdrant(json!(qdrant_points)).await?; }

        for page in res.parts {
            sqlx::query("INSERT INTO module_file_pages (module_id, page_number, content) VALUES ($1, $2, $3)")
                .bind(module_id).bind(page.page_number).bind(page.text).execute(&mut *tx).await.map_err(|e| e.to_string())?;
        }

        sqlx::query("UPDATE modules SET type = 'File' WHERE id = $1")
            .bind(module_id).execute(&mut *tx).await.map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn get_embeddings(&self, inputs: Vec<String>) -> Result<Vec<Vec<f32>>, String> {
        let mut all_embeddings = Vec::new();
        for input in inputs {
            let req = json!({ "input": input });
            let res = self.client.post(format!("{}/v1/embeddings", self.env.gateway_url))
                .header("Authorization", format!("Bearer {}", self.env.gateway_token))
                .json(&req).send().await.map_err(|e| e.to_string())?;
            let data: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;

            if let Some(arr) = data["embeddings"][0].as_array() {
                let vec_f32: Vec<f32> = arr.iter().filter_map(|v| v.as_f64()).map(|v| v as f32).collect();
                all_embeddings.push(vec_f32);
            }
        }
        Ok(all_embeddings)
    }

    pub async fn process_text(&self, text: &str) -> Result<HashResText, String> {
        let hash = Self::calculate_hash(text.as_bytes());

        if let Some(row) = sqlx::query("SELECT data FROM hash_cache WHERE hash = $1").bind(hash.clone()).fetch_optional(&self.db).await.map_err(|e| e.to_string())? {
            return bincode::deserialize(&row.get::<Vec<u8>, _>("data")).map_err(|e| e.to_string());
        }

        let chunk_req = json!({ "text": text, "max_chunk_size": 4000, "overlap": 400.0 });
        let chunk_res = self.client.post(format!("{}/api/v1/chunkText", self.env.doc_processor_url))
            .header("Authorization", format!("Bearer {}", self.env.doc_processor_token))
            .json(&chunk_req).send().await.map_err(|e| e.to_string())?;

        let chunk_data: serde_json::Value = chunk_res.json().await
            .map_err(|e| format!("Failed to parse DocProcessor JSON: {}", e))?;
        let texts: Vec<String> = chunk_data["chunks"]
            .as_array()
            .ok_or_else(|| "DocProcessor response missing 'chunks' array".to_string())?
            .iter()
            .map(|c| c["text"].as_str().ok_or_else(|| "Chunk is missing 'text' field".to_string())
                .map(|s| s.to_string())).collect::<Result<Vec<_>, String>>()?;

        let embeddings = self.get_embeddings(texts.clone()).await?;

        let mut res = HashResText { chunks: vec![] };
        for (txt, emb) in texts.into_iter().zip(embeddings) {
            res.chunks.push(TextChunkRes { text: txt, embedding: emb });
        }

        let bytes = bincode::serialize(&res).map_err(|e| e.to_string())?;
        sqlx::query("INSERT INTO hash_cache (hash, data) VALUES ($1, $2)").bind(hash).bind(bytes).execute(&self.db).await.map_err(|e| e.to_string())?;

        Ok(res)
    }

    pub async fn process_video(&self, video_url: &str) -> Result<HashResVideo, String> {
        let hash = Self::calculate_hash(video_url.as_bytes());

        if let Some(row) = sqlx::query("SELECT data FROM hash_cache WHERE hash = $1").bind(hash.clone()).fetch_optional(&self.db).await.map_err(|e| e.to_string())? {
            return bincode::deserialize(&row.get::<Vec<u8>, _>("data")).map_err(|e| e.to_string());
        }

        let req = json!({ "video_url": video_url });
        let parse_res = self.client.post(format!("{}/api/v1/parseVideo", self.env.doc_processor_url))
            .header("Authorization", format!("Bearer {}", self.env.doc_processor_token))
            .json(&req).send().await.map_err(|e| e.to_string())?;

        if !parse_res.status().is_success() {
            let status = parse_res.status();
            let err_body = parse_res.text().await.unwrap_or_default();
            return Err(format!("DocumentProcessor returned error {}: {}", status, err_body));
        }

        let parse_data: serde_json::Value = parse_res.json().await
            .map_err(|e| format!("Failed to parse DocProcessor JSON: {}", e))?;
        let text = parse_data["text"].as_str().unwrap_or("").to_string();
        let parts_json = parse_data["parts"].as_array().unwrap_or(&vec![]).clone();

        let mut modified_parts = Vec::new();
        if let Some(parts_array) = parse_data["parts"].as_array() {
            for part in parts_array {
                let mut p = part.clone();
                let start = p["start"].as_f64().unwrap_or(0.0);
                let original_text = p["text"].as_str().unwrap_or("");

                let new_text = format!("[TIMECODE: {:.0}] {}", start, original_text);

                if let Some(obj) = p.as_object_mut() {
                    obj.insert("text".to_string(), json!(new_text));
                }
                modified_parts.push(p);
            }
        }

        let chunk_req = json!({ "parts": modified_parts, "max_chunk_size": 4000, "overlap": 400.0 });
        let chunk_res = self.client.post(format!("{}/api/v1/chunkPartitionedText", self.env.doc_processor_url))
            .header("Authorization", format!("Bearer {}", self.env.doc_processor_token))
            .json(&chunk_req).send().await.map_err(|e| e.to_string())?;

        let chunk_data: serde_json::Value = chunk_res.json().await
            .map_err(|e| format!("Failed to parse DocProcessor JSON: {}", e))?;

        let mut res = HashResVideo { text, parts: vec![], chunks: vec![] };

        for p in parts_json {
            res.parts.push(VideoPartRes {
                text: p["text"].as_str().unwrap_or("").to_string(),
                start: p["start"].as_f64().unwrap_or(0.0) as f32,
                end: p["end"].as_f64().unwrap_or(0.0) as f32,
            });
        }

        let c_arr = chunk_data["chunks"].as_array()
            .ok_or_else(|| "DocProcessor response missing 'chunks' array".to_string())?;
        let texts: Vec<String> = c_arr.iter().map(|c| c["text"].as_str()
            .ok_or_else(|| "Chunk is missing 'text' field".to_string()).map(|s| s.to_string()))
            .collect::<Result<Vec<_>, String>>()?;
        let embeddings = self.get_embeddings(texts).await?;

        for (i, c) in c_arr.iter().enumerate() {
            res.chunks.push(VideoChunkRes {
                text: c["text"].as_str().unwrap_or("").to_string(),
                start: c["start"].as_f64().unwrap_or(0.0) as f32,
                end: c["end"].as_f64().unwrap_or(0.0) as f32,
                embedding: embeddings[i].clone()
            });
        }

        let bytes = bincode::serialize(&res).map_err(|e| e.to_string())?;
        sqlx::query("INSERT INTO hash_cache (hash, data) VALUES ($1, $2)").bind(hash).bind(bytes).execute(&self.db).await.map_err(|e| e.to_string())?;

        Ok(res)
    }

    pub async fn process_document(&self, filename: &str, file: Vec<u8>) -> Result<HashResDocument, String> {
        let hash = Self::calculate_hash(&file);

        if let Some(row) = sqlx::query("SELECT data FROM hash_cache WHERE hash = $1").bind(hash.clone()).fetch_optional(&self.db).await.map_err(|e| e.to_string())? {
            return bincode::deserialize(&row.get::<Vec<u8>, _>("data")).map_err(|e| e.to_string());
        }

        let part = multipart::Part::bytes(file).file_name(filename.to_string());
        let form = multipart::Form::new().part("file", part);

        let parse_res = self.client.post(format!("{}/api/v1/parseDocument", self.env.doc_processor_url))
            .header("Authorization", format!("Bearer {}", self.env.doc_processor_token))
            .multipart(form).send().await.map_err(|e| e.to_string())?;

        if !parse_res.status().is_success() {
            let status = parse_res.status();
            let err_body = parse_res.text().await.unwrap_or_default();
            return Err(format!("DocumentProcessor returned error {}: {}", status, err_body));
        }

        let parse_data: serde_json::Value = parse_res.json().await
            .map_err(|e| format!("Failed to parse DocProcessor JSON: {}", e))?;

        let text = parse_data["text"].as_str().unwrap_or("").to_string();
        let page_count = parse_data["page_count"].as_i64().unwrap_or(0) as i32;
        let parts_json = parse_data["parts"].as_array().unwrap_or(&vec![]).clone();

        if page_count == 0 {
            return Err("DocumentProcessor returned 0 pages. Processing failed.".into());
        }

        let mut pseudo_parts = vec![];
        for p in &parts_json {
            let pg = p["page_number"].as_f64().unwrap_or(0.0);
            pseudo_parts.push(json!({
                "text": p["text"].as_str().unwrap_or(""),
                "start": pg,
                "end": pg
            }));
        }

        let chunk_req = json!({ "parts": pseudo_parts, "max_chunk_size": 4000, "overlap": 400.0 });
        let chunk_res = self.client.post(format!("{}/api/v1/chunkPartitionedText", self.env.doc_processor_url))
            .header("Authorization", format!("Bearer {}", self.env.doc_processor_token))
            .json(&chunk_req).send().await.map_err(|e| e.to_string())?;

        let chunk_data: serde_json::Value = chunk_res.json().await
            .map_err(|e| format!("Failed to parse DocProcessor JSON: {}", e))?;

        let mut res = HashResDocument { text, page_count, parts: vec![], chunks: vec![] };

        for p in parts_json {
            res.parts.push(DocPartRes {
                text: p["text"].as_str().unwrap_or("").to_string(),
                page_number: p["page_number"].as_i64().unwrap_or(0) as i32,
            });
        }

        let c_arr = chunk_data["chunks"].as_array()
            .ok_or_else(|| "DocProcessor response missing 'chunks' array".to_string())?;
        let texts: Vec<String> = c_arr.iter().map(|c| c["text"].as_str()
            .ok_or_else(|| "Chunk is missing 'text' field".to_string()).map(|s| s.to_string()))
            .collect::<Result<Vec<_>, String>>()?;
        let embeddings = self.get_embeddings(texts).await?;

        for (i, c) in c_arr.iter().enumerate() {
            let start = c["start"].as_f64().unwrap_or(0.0) as i32;
            let end = c["end"].as_f64().unwrap_or(0.0) as i32;
            let pages: Vec<i32> = (start..=end).collect();

            res.chunks.push(DocChunkRes {
                text: c["text"].as_str().unwrap_or("").to_string(),
                page_numbers: pages,
                embedding: embeddings[i].clone()
            });
        }

        let bytes = bincode::serialize(&res).map_err(|e| e.to_string())?;
        sqlx::query("INSERT INTO hash_cache (hash, data) VALUES ($1, $2)").bind(hash).bind(bytes).execute(&self.db).await.map_err(|e| e.to_string())?;

        Ok(res)
    }
}