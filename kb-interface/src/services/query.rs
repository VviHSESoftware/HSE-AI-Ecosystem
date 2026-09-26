use std::sync::Arc;
use reqwest::Client;
use serde_json::json;
use tracing::error;
use ai_gateway_client::AiGateway;
use crate::config::AppEnv;
use kb_common::repository::KbRepository;
use crate::schemas::*;
use common_infra::AppError;

pub struct QueryService {
    pub env: AppEnv,
    pub repo: KbRepository,
    pub client: Client,
    ai_gateway: Arc<dyn AiGateway>
}

impl QueryService {
    pub fn new(env: AppEnv, repo: KbRepository, client: Client, ai_gateway: Arc<dyn AiGateway>) -> Self {
        Self { env, repo, client, ai_gateway }
    }
    
    fn get_ids_by_path(&self, path: &str) -> Vec<i32> {
        path.split('/').filter(|s| !s.is_empty()).filter_map(|s| s.parse().ok()).collect()
    }

    pub async fn get_user_allowed_roots(&self, email: &str) -> Result<Vec<i32>, AppError> {
        if let Some(fake_roots) = self.env.fake_users.get(email.trim()) {
            return Ok(fake_roots.clone());
        }

        self.repo.get_user_allowed_roots(email).await.map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn get_embedding(&self, text: &str) -> Result<Vec<f32>, AppError> {
        let res = self.ai_gateway.create_embeddings(text.to_string(), "query".to_string()).await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        res.embeddings.first()
            .cloned()
            .ok_or_else(|| AppError::Internal("AI Gateway returned empty embeddings".into()))
    }


    async fn get_module_desc(&self, module_id: i32) -> String {
        if let Ok(Some(path_str)) = self.repo.get_module_path(module_id).await {
            let ids = self.get_ids_by_path(&path_str);

            if let Ok(m_rows) = self.repo.get_modules_info_by_ids(&ids).await {
                return m_rows.into_iter()
                    .map(|mr| format!("{}: {}", mr.external_type.unwrap_or_else(|| "System".to_string()), mr.name))
                    .collect::<Vec<_>>().join(", ");
            }
        }
        "Unknown".to_string()
    }

    pub async fn semantic_query(&self, req: SemanticQueryReq) -> Result<QueryRes, AppError> {
        let allowed_roots = self.get_user_allowed_roots(&req.email).await?;
        if allowed_roots.is_empty() { return Ok(QueryRes { chunks: vec![] }); }

        let vector = self.get_embedding(&req.semantic_query).await?;

        let mut must_filters = vec![
            json!({ "key": "parent_ids", "match": { "any": allowed_roots } })
        ];

        if !req.module_ids.is_empty() {
            must_filters.push(json!({
                "key": "parent_ids",
                "match": { "any": req.module_ids }
            }));
        }

        for kw in &req.keywords {
            must_filters.push(json!({
                "key": "text", "match": { "text": kw }
            }));
        }

        let filter = json!({ "must": must_filters });

        let qdrant_res = self.client.post(format!("{}/collections/kb_chunks/points/search", self.env.qdrant_url))
            .header("api-key", &self.env.qdrant_api_key)
            .json(&json!({
                "vector": vector,
                "limit": 5,
                "filter": filter,
                "with_payload": true
            })).send().await.map_err(|e| AppError::Internal(e.to_string()))?
            .json::<serde_json::Value>().await.map_err(|e| AppError::Internal(e.to_string()))?;

        let mut chunks = vec![];
        if let Some(hits) = qdrant_res["result"].as_array() {
            for hit in hits {
                let score = hit["score"].as_f64().unwrap_or(0.0);
                tracing::info!("Found chunk with score: {}", score);
                let p = &hit["payload"];
                let module_url = self.repo.get_module_url(
                    p["module_id"]
                    .as_i64()
                    .and_then(|val| val.try_into().ok())
                    .unwrap_or_default()
                ).await.map_err(|e| AppError::Internal(e.to_string()))?.unwrap_or_default();
                chunks.push(QueryChunk {
                    desc: p["description"].as_str().unwrap_or_default().to_string(),
                    text: p["text"].as_str().unwrap_or_default().to_string(),
                    url: module_url
                });
            }
        }

        Ok(QueryRes { chunks })
    }

    pub async fn timed_video_query(&self, req: TimedVideoQueryReq) -> Result<QueryRes, AppError> {
        let allowed = self.get_user_allowed_roots(&req.email).await?;
        let path_str = self.repo.get_module_path(req.module_id).await.map_err(|e| AppError::Internal(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Module not found".into()))?;

        let path_ids = self.get_ids_by_path(&path_str);
        let has_access = path_ids.iter().any(|id| allowed.contains(id));

        if !has_access {
            return Err(AppError::Forbidden("Access denied".into()));
        }

        let rows = self.repo.get_video_chunks(req.module_id, req.second_start, req.second_end).await.map_err(|e| AppError::Internal(e.to_string()))?;
        if rows.is_empty() {
            return Ok(QueryRes { chunks: vec![] });
        }

        let mut combined_text = String::new();
        for row in rows {
            combined_text.push_str(&format!("[TIMECODE: {:.0}] {} ", row.start_time, row.content));
        }

        let module_url = self.repo.get_module_url(req.module_id).await.map_err(|e| AppError::Internal(e.to_string()))?.unwrap_or_default();
        let desc = self.get_module_desc(req.module_id).await;

        let chunks = vec![QueryChunk {
            desc,
            text: combined_text.trim().to_string(),
            url: module_url
        }];

        Ok(QueryRes { chunks })
    }

    pub async fn document_page_query(&self, req: DocumentPageQueryReq) -> Result<QueryRes, AppError> {
        let allowed = self.get_user_allowed_roots(&req.email).await?;
        let path_str = self.repo.get_module_path(req.module_id).await.map_err(|e| AppError::Internal(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Module not found".into()))?;

        let path_ids = self.get_ids_by_path(&path_str);
        let has_access = path_ids.iter().any(|id| allowed.contains(id));

        if !has_access {
            return Err(AppError::Forbidden("Access denied".into()));
        }

        let module_url = self.repo.get_module_url(req.module_id).await.map_err(|e| AppError::Internal(e.to_string()))?.unwrap_or_default();
        let content = self.repo.get_document_page_content(req.module_id, req.page).await.map_err(|e| AppError::Internal(e.to_string()))?.unwrap_or_default();
        let desc = self.get_module_desc(req.module_id).await;

        Ok(QueryRes {
            chunks: vec![QueryChunk { desc, text: content, url: module_url }]
        })
    }

    pub async fn full_content_query(&self, req: FullContentQueryReq) -> Result<QueryRes, AppError> {
        let allowed = self.get_user_allowed_roots(&req.email).await?;
        let mut all_chunks = vec![];

        for mid in req.module_ids {
            let path_str = match self.repo.get_module_path(mid).await.map_err(|e| AppError::Internal(e.to_string()))? {
                Some(path) => path,
                None => return Err(AppError::NotFound("Module not found".into())),
            };

            let path_ids: Vec<i32> = path_str.split('/').filter(|s| !s.is_empty()).filter_map(|s| s.parse().ok()).collect();
            let has_access = path_ids.iter().any(|id| allowed.contains(id));

            if !has_access {
                error!("User {} asked for forbidden material {}", &req.email, mid);
                continue;
            }

            let desc = self.get_module_desc(mid).await;
            let module_url = self.repo.get_module_url(mid).await.map_err(|e| AppError::Internal(e.to_string()))?.unwrap_or_default();

            let text_contents = self.repo.get_text_module_content(mid).await.map_err(|e| AppError::Internal(e.to_string()))?;
            for content in text_contents {
                all_chunks.push(QueryChunk { desc: desc.clone(), text: content, url: module_url.clone() });
            }

            let file_contents = self.repo.get_all_document_pages_content(mid).await.map_err(|e| AppError::Internal(e.to_string()))?;
            for content in file_contents {
                all_chunks.push(QueryChunk { desc: desc.clone(), text: content, url: module_url.clone() });
            }
        }

        Ok(QueryRes { chunks: all_chunks })
    }

    pub async fn available_structure(&self, req: AvailableStructureReq) -> Result<StructureRes, AppError> {
        let allowed_roots = self.get_user_allowed_roots(&req.email).await?;

        let roots = if let Some(req_ids) = req.module_ids {
            req_ids.into_iter().filter(|id| allowed_roots.contains(id)).collect::<Vec<_>>()
        } else {
            allowed_roots
        };

        if roots.is_empty() { return Ok(StructureRes { modules: vec![] }); }

        let mut rows = self.repo.get_available_structure(&roots).await.map_err(|e| AppError::Internal(e.to_string()))?;

        let mut nodes: std::collections::HashMap<i32, ModuleNode> = rows.iter().map(|r| {
            (r.id, ModuleNode {
                id: r.id,
                name: r.name.clone(),
                r#type: r.r#type.clone(),
                page_count: r.page_count,
                video_length: r.total_time,
                sub_modules: vec![]
            })
        }).collect();

        rows.sort_by_cached_key(|r| {
            r.path.clone().unwrap_or_default().split('/').count()
        });
        rows.reverse();

        for r in &rows {
            let id: i32 = r.id;
            let parent_id: Option<i32> = r.parent_id;

            if let Some(pid) = parent_id {
                if nodes.contains_key(&pid) {
                    if let Some(current_node) = nodes.remove(&id) {
                        if let Some(parent_node) = nodes.get_mut(&pid) {
                            parent_node.sub_modules.push(current_node);
                        } else {
                            nodes.insert(id, current_node);
                        }
                    }
                }
            }
        }

        let result_roots: Vec<ModuleNode> = nodes.into_values().collect();

        let mut transformed_roots = Vec::new();
        for root in result_roots {
            if let Some(processed) = Self::post_process_node(root) {
                transformed_roots.push(processed);
            }
        }

        transformed_roots.sort_by_key(|n| n.id);

        Ok(StructureRes { modules: transformed_roots })
    }

    fn post_process_node(mut node: ModuleNode) -> Option<ModuleNode> {
        let mut processed_subs = Vec::new();
        for sub in node.sub_modules {
            if let Some(processed) = Self::post_process_node(sub) {
                processed_subs.push(processed);
            }
        }
        node.sub_modules = processed_subs;

        if node.sub_modules.is_empty() && node.r#type == "Base" {
            return None;
        }

        if node.sub_modules.len() == 1 {
            let child = &node.sub_modules[0];
            if child.sub_modules.is_empty() {
                if child.r#type == "Video" {
                    let mut compacted = node.sub_modules.remove(0);
                    compacted.name = node.name;
                    return Some(compacted);
                } else if child.r#type == "File" {
                    let mut compacted = node.sub_modules.remove(0);
                    compacted.name = format!("{} ; {}", node.name, compacted.name);
                    return Some(compacted);
                }
            }
        }

        node.sub_modules.sort_by_key(|sub| sub.id);

        Some(node)
    }
}