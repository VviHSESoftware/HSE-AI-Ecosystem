use std::sync::Arc;
use bincode;
use reqwest::Client;
use serde_json::json;
use sha2::{Digest, Sha256};
use tracing::info;
use graphile_worker::WorkerUtils;

use crate::config::AppEnv;
use kb_common::repository::KbRepository;
use kb_common::repository::models::{NewVideoChunk, NewFilePage};
use crate::schemas::*;
use crate::worker::{ModuleJobPayload, ProcessModuleTask};
use common_infra::AppError;
use ai_gateway_client::AiGateway;
use doc_processor_client::{DocProcessor, ChunkTextRequest};

pub struct ModuleService {
    pub env: AppEnv,
    pub repo: KbRepository,
    pub client: Client,
    pub ai_gateway: Arc<dyn AiGateway>,
    pub doc_processor: Arc<dyn DocProcessor>,
    pub worker_utils: WorkerUtils,
}

impl ModuleService {
    pub fn new(
        env: AppEnv,
        repo: KbRepository,
        client: Client,
        ai_gateway: Arc<dyn AiGateway>,
        doc_processor: Arc<dyn DocProcessor>,
        worker_utils: WorkerUtils,
    ) -> Self {
        Self { env, repo, client, ai_gateway, doc_processor, worker_utils }
    }

    fn calculate_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    async fn build_hierarchy_description(&self, ids: &[i32]) -> String {
        if ids.is_empty() { return String::new(); }

        let mut map: std::collections::HashMap<i32, (String, String)> = self.repo.get_modules_info_by_ids(ids).await.unwrap_or_default()
            .into_iter()
            .map(|r| (r.id, (r.external_type.unwrap_or_else(|| "System".to_string()), r.name)))
            .collect();

        ids.iter()
            .filter_map(|id| map.remove(id))
            .map(|(ext_type, name)| format!("{}: {}", ext_type, name))
            .collect::<Vec<_>>()
            .join(", ")
    }

    async fn get_parent_ids_from_db(&self, module_id: i32) -> Vec<i32> {
        if let Ok(Some(path_str)) = self.repo.get_module_path(module_id).await {
            path_str.split('/').filter(|s| !s.is_empty()).filter_map(|s| s.parse::<i32>().ok()).collect()
        } else {
            vec![module_id]
        }
    }

    async fn upsert_to_qdrant(&self, points: serde_json::Value) -> Result<(), AppError> {
        let url = format!("{}/collections/kb_chunks/points?wait=true", self.env.qdrant_url.trim_end_matches('/'));

        let mut request = self.client.put(url).json(&json!({ "points": points }));
        if !self.env.qdrant_api_key.is_empty() {
            request = request.header("api-key", &self.env.qdrant_api_key);
        }

        let resp = request.send().await.map_err(|e| AppError::Internal(e.to_string()))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let err_body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!("Qdrant error (Status {}): {}", status, err_body)));
        }
        Ok(())
    }

    pub async fn check_access(&self, system_id: i32, module_id: i32) -> Result<(), AppError> {
        let prefix = format!("/{}", system_id);
        if let Some(path) = self.repo.get_module_path(module_id).await.map_err(|e| AppError::Internal(e.to_string()))? {
            if path == prefix || path.starts_with(&format!("{}/", prefix)) {
                return Ok(());
            }
        }
        Err(AppError::Forbidden("Access denied: Module does not belong to your system".into()))
    }

    pub async fn get_module_id_secure(&self, system_id: i32, ext_id: &str, ext_type: &str) -> Result<i32, AppError> {
        self.repo.get_module_id_secure(system_id, ext_id, ext_type).await.map_err(|e| AppError::Internal(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Module not found in your system scope".into()))
    }

    pub async fn get_supported_file_types(&self) -> Result<SupportedFileTypesRes, AppError> {
        let res = self.client.get(format!("{}/v1/getSupportedDocumentTypes", self.env.doc_processor_url))
            .header("Authorization", format!("Bearer {}", self.env.doc_processor_token))
            .send().await.map_err(|e| AppError::Internal(e.to_string()))?;

        res.json().await.map_err(|e| AppError::Internal(e.to_string()))
    }

    pub async fn add_module_base(&self, system_id: i32, req: AddModuleBaseReq) -> Result<i32, AppError> {
        self.check_access(system_id, req.parent_module_id).await?;
        self.repo.add_module_base(&req.name, &req.url, &req.external_id, &req.external_type, req.parent_module_id)
            .await.map_err(|e| AppError::Internal(e.to_string()))
    }

    pub async fn add_module_access(&self, system_id: i32, module_id: i32, emails: &[String]) -> Result<(), AppError> {
        self.check_access(system_id, module_id).await?;
        self.repo.add_module_access(module_id, emails).await.map_err(|e| AppError::Internal(e.to_string()))
    }

    pub async fn invalidate_module(&self, system_id: i32, external_id: &str, external_type: &str) -> Result<(), AppError> {
        let id = self.get_module_id_secure(system_id, external_id, external_type).await?;

        let url = format!("{}/collections/kb_chunks/points/delete", self.env.qdrant_url.trim_end_matches('/'));
        let payload = json!({ "filter": { "must": [{ "key": "parent_ids", "match": { "value": id } }] } });

        let mut request = self.client.post(url).json(&payload);
        if !self.env.qdrant_api_key.is_empty() {
            request = request.header("api-key", &self.env.qdrant_api_key);
        }

        let resp = request.send().await.map_err(|e| AppError::Internal(e.to_string()))?;
        if !resp.status().is_success() {
            let err_body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!("Qdrant delete error: {}", err_body)));
        }

        self.repo.delete_module(id).await.map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(())
    }

    pub async fn queue_text_type(&self, system_id: i32, module_id: i32, text: String) -> Result<(), AppError> {
        self.check_access(system_id, module_id).await?;
        self.worker_utils
            .add_job(ProcessModuleTask { payload: ModuleJobPayload::Text { module_id, text } }, Default::default())
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(())
    }

    pub async fn queue_video_type(&self, system_id: i32, module_id: i32, video_url: String) -> Result<(), AppError> {
        self.check_access(system_id, module_id).await?;
        self.worker_utils
            .add_job(ProcessModuleTask { payload: ModuleJobPayload::VideoUrl { module_id, video_url } }, Default::default())
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(())
    }

    pub async fn queue_video_file_type(&self, system_id: i32, module_id: i32, filename: String, file_bytes: Vec<u8>) -> Result<(), AppError> {
        self.check_access(system_id, module_id).await?;
        let temp_dir = std::env::temp_dir().join("kb_uploads");
        tokio::fs::create_dir_all(&temp_dir).await.map_err(|e| AppError::Internal(e.to_string()))?;

        let temp_path = temp_dir.join(format!("vid_{}_{}", uuid::Uuid::new_v4(), filename));
        tokio::fs::write(&temp_path, &file_bytes).await.map_err(|e| AppError::Internal(e.to_string()))?;

        self.worker_utils
            .add_job(ProcessModuleTask {
                payload: ModuleJobPayload::VideoFile {
                    module_id,
                    filename,
                    temp_path: temp_path.to_string_lossy().to_string(),
                }
            }, Default::default())
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(())
    }

    pub async fn queue_document_type(&self, system_id: i32, module_id: i32, filename: String, file_bytes: Vec<u8>) -> Result<(), AppError> {
        self.check_access(system_id, module_id).await?;
        let temp_dir = std::env::temp_dir().join("kb_uploads");
        tokio::fs::create_dir_all(&temp_dir).await.map_err(|e| AppError::Internal(e.to_string()))?;

        let temp_path = temp_dir.join(format!("doc_{}_{}", uuid::Uuid::new_v4(), filename));
        tokio::fs::write(&temp_path, &file_bytes).await.map_err(|e| AppError::Internal(e.to_string()))?;

        self.worker_utils
            .add_job(ProcessModuleTask {
                payload: ModuleJobPayload::DocumentFile {
                    module_id,
                    filename,
                    temp_path: temp_path.to_string_lossy().to_string(),
                }
            }, Default::default())
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(())
    }


    pub async fn process_text_type(&self, module_id: i32, text: String) -> Result<(), AppError> {
        let hash = Self::calculate_hash(text.as_bytes());
        let res = if let Some(cached_res) = self.load_from_cache::<HashResText>(&hash).await? {
            info!("Cache hit for text snippet");
            cached_res
        } else {
            info!("Cache miss for text. Starting processing...");
            let chunk_res = self.doc_processor.chunk_text(ChunkTextRequest {
                text: text.to_string(), max_chunk_size: 4000, overlap: 400.0,
            }).await.map_err(|e| AppError::Internal(e.to_string()))?;

            let texts: Vec<String> = chunk_res.chunks.into_iter().map(|c| c.text).collect();
            let embeddings = self.get_embeddings(texts.clone()).await?;

            let mut out = HashResText { chunks: vec![] };
            for (txt, emb) in texts.into_iter().zip(embeddings) {
                out.chunks.push(TextChunkRes { text: txt, embedding: emb });
            }
            self.save_to_cache(&hash, &out).await?;
            out
        };

        let parent_ids = self.get_parent_ids_from_db(module_id).await;
        let description = self.build_hierarchy_description(&parent_ids).await;
        self.repo.persist_text_module(module_id, &text).await.map_err(|e| AppError::Internal(e.to_string()))?;

        let mut qdrant_points = vec![];
        for chunk in res.chunks {
            qdrant_points.push(json!({
                "id": uuid::Uuid::new_v4().to_string(),
                "vector": chunk.embedding,
                "payload": {
                    "module_id": module_id, "parent_ids": parent_ids,
                    "description": description, "type": "text", "text": chunk.text
                }
            }));
        }

        if !qdrant_points.is_empty() { self.upsert_to_qdrant(json!(qdrant_points)).await?; }
        Ok(())
    }

    pub async fn process_video_type(&self, module_id: i32, video_url: String) -> Result<(), AppError> {
        let hash = Self::calculate_hash(video_url.as_bytes());
        self.process_video_pipeline(
            module_id,
            hash,
            &video_url,
            self.doc_processor.parse_video(&video_url)
        ).await
    }

    pub async fn process_video_file_type(&self, module_id: i32, filename: String, file_bytes: Vec<u8>) -> Result<(), AppError> {
        let hash = Self::calculate_hash(&file_bytes);
        self.process_video_pipeline(
            module_id,
            hash,
            &filename,
            self.doc_processor.parse_video_file(&filename, file_bytes)
        ).await
    }

    async fn process_video_pipeline(
        &self,
        module_id: i32,
        hash: String,
        source_name: &str,
        parser_future: impl Future<Output = Result<doc_processor_client::ParseVideoResponse, AppError>>,
    ) -> Result<(), AppError> {
        let res = if let Some(cached_res) = self.load_from_cache::<HashResVideo>(&hash).await? {
            info!("Cache hit for video: {}", source_name);
            cached_res
        } else {
            info!("Cache miss for video: {}. Starting processing...", source_name);
            let parse_data = parser_future.await?;

            let mut modified_parts = parse_data.parts.clone();
            for p in &mut modified_parts {
                p.text = format!("[TIMECODE: {:.0}] {}", p.start, p.text);
            }

            let chunk_res = self.doc_processor.chunk_partitioned(doc_processor_client::types::ChunkPartitionedTextRequest {
                parts: modified_parts, max_chunk_size: 4000, overlap: 400.0,
            }).await.map_err(|e| AppError::Internal(e.to_string()))?;

            let mut out = HashResVideo {
                text: parse_data.text,
                parts: parse_data.parts.iter().map(|p| VideoPartRes {
                    text: p.text.clone(), start: p.start as f32, end: p.end as f32,
                }).collect(),
                chunks: vec![]
            };

            let texts: Vec<String> = chunk_res.chunks.iter().map(|c| c.text.clone()).collect();
            let embeddings = self.get_embeddings(texts).await?;

            for (i, c) in chunk_res.chunks.into_iter().enumerate() {
                out.chunks.push(VideoChunkRes {
                    text: c.text, start: c.start as f32, end: c.end as f32, embedding: embeddings[i].clone()
                });
            }
            self.save_to_cache(&hash, &out).await?;
            out
        };

        let parent_ids = self.get_parent_ids_from_db(module_id).await;
        let description = self.build_hierarchy_description(&parent_ids).await;

        let total_time = res.parts.last().map(|p| p.end).unwrap_or(0.0) as f64;
        let chunks: Vec<NewVideoChunk> = res.parts.into_iter().map(|p| NewVideoChunk {
            start: p.start as f64, end: p.end as f64, content: p.text
        }).collect();

        self.repo.persist_video_module(module_id, &res.text, total_time, &chunks).await.map_err(|e| AppError::Internal(e.to_string()))?;

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

        if !qdrant_points.is_empty() {
            self.upsert_to_qdrant(json!(qdrant_points)).await?;
        }
        Ok(())
    }

    pub async fn process_document_type(&self, module_id: i32, filename: String, file: Vec<u8>) -> Result<(), AppError> {
        let hash = Self::calculate_hash(&file);
        let res = if let Some(cached_res) = self.load_from_cache::<HashResDocument>(&hash).await? {
            info!("Cache hit for file: {}", filename);
            cached_res
        } else {
            info!("Cache miss for file: {}. Starting processing...", filename);
            let parse_data = self.doc_processor.parse_document(&filename, file).await.map_err(|e| AppError::Internal(e.to_string()))?;
            let pseudo_parts = parse_data.parts.iter().map(|p| doc_processor_client::types::PartitionPart {
                text: p.text.clone(), start: p.page_number as f64, end: p.page_number as f64,
            }).collect();

            let chunk_res = self.doc_processor.chunk_partitioned(doc_processor_client::types::ChunkPartitionedTextRequest {
                parts: pseudo_parts, max_chunk_size: 4000, overlap: 400.0,
            }).await.map_err(|e| AppError::Internal(e.to_string()))?;

            let mut out = HashResDocument {
                text: parse_data.text, page_count: parse_data.page_count as i32,
                parts: parse_data.parts.iter().map(|p| DocPartRes {
                    text: p.text.clone(), page_number: p.page_number as i32,
                }).collect(),
                chunks: vec![]
            };

            let texts: Vec<String> = chunk_res.chunks.iter().map(|c| c.text.clone()).collect();
            let embeddings = self.get_embeddings(texts).await?;

            for (i, c) in chunk_res.chunks.into_iter().enumerate() {
                let pages: Vec<i32> = (c.start as i32 ..= c.end as i32).collect();
                out.chunks.push(DocChunkRes {
                    text: c.text, page_numbers: pages, embedding: embeddings[i].clone()
                });
            }
            self.save_to_cache(&hash, &out).await?;
            out
        };

        let parent_ids = self.get_parent_ids_from_db(module_id).await;
        let description = self.build_hierarchy_description(&parent_ids).await;

        let pages: Vec<NewFilePage> = res.parts.into_iter().map(|p| NewFilePage {
            page_number: p.page_number, content: p.text
        }).collect();

        self.repo.persist_file_module(module_id, &filename, &res.text, res.page_count, &pages).await.map_err(|e| AppError::Internal(e.to_string()))?;

        let mut qdrant_points = vec![];
        for chunk in res.chunks {
            qdrant_points.push(json!({
                "id": uuid::Uuid::new_v4().to_string(), "vector": chunk.embedding,
                "payload": {
                    "module_id": module_id, "parent_ids": parent_ids, "description": description,
                    "type": "file", "text": chunk.text, "page_numbers": chunk.page_numbers
                }
            }));
        }

        if !qdrant_points.is_empty() { self.upsert_to_qdrant(json!(qdrant_points)).await?; }
        Ok(())
    }

    async fn get_embeddings(&self, inputs: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        let mut all_embeddings = Vec::new();
        for input in inputs {
            let res = self.ai_gateway.create_embeddings(input, "passage".to_string()).await.map_err(|e| AppError::Internal(e.to_string()))?;
            if let Some(emb) = res.embeddings.first() {
                all_embeddings.push(emb.clone());
            }
        }
        Ok(all_embeddings)
    }

    async fn save_to_cache<T: serde::Serialize>(&self, hash: &str, data: &T) -> Result<(), AppError> {
        let bytes = bincode::serialize(data).map_err(|e| AppError::Internal(e.to_string()))?;
        self.repo.set_hash_cache(hash, &bytes).await.map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn load_from_cache<T: serde::de::DeserializeOwned>(&self, hash: &str) -> Result<Option<T>, AppError> {
        if let Some(bytes) = self.repo.get_hash_cache(hash).await.map_err(|e| AppError::Internal(e.to_string()))? {
            return bincode::deserialize(&bytes).map_err(|e| AppError::Internal(e.to_string())).map(Some);
        }
        Ok(None)
    }
}