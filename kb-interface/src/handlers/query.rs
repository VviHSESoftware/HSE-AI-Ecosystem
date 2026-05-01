use axum::{extract::State, Json};
use std::sync::Arc;
use serde_json::json;
use sqlx::Row;
use tracing::error;
use crate::{schemas::*, state::InterfaceState};
use common_infra::AppError;

#[utoipa::path(
    post,
    path = "/api/v1/semanticQuery",
    request_body = SemanticQueryReq,
    responses((status = 200, body = QueryRes)),
    security(("bearerAuth" = []))
)]
pub async fn semantic_query(State(state): State<Arc<InterfaceState>>, Json(req): Json<SemanticQueryReq>) -> Result<Json<QueryRes>, AppError> {
    let allowed_roots = state.get_user_allowed_roots(&req.email).await?;
    if allowed_roots.is_empty() { return Ok(Json(QueryRes { chunks: vec![] })); }

    let vector = state.get_embedding(&req.semantic_query).await?;

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

    let qdrant_res = state.client.post(format!("{}/collections/kb_chunks/points/search", state.env.qdrant_url))
        .header("api-key", &state.env.qdrant_api_key)
        .json(&json!({
            "vector": vector,
            "limit": 5,
            "filter": filter,
            "with_payload": true
        })).send().await?.json::<serde_json::Value>().await?;

    let mut chunks = vec![];
    if let Some(hits) = qdrant_res["result"].as_array() {
        for hit in hits {
            let score = hit["score"].as_f64().unwrap_or(0.0);
            tracing::info!("Found chunk with score: {}", score);
            let p = &hit["payload"];
            chunks.push(QueryChunk {
                desc: p["description"].as_str().unwrap_or_default().to_string(),
                text: p["text"].as_str().unwrap_or_default().to_string(),
                url: p["url"].to_string()
            });
        }
    }

    Ok(Json(QueryRes { chunks }))
}

#[utoipa::path(
    post,
    path = "/api/v1/timedVideoQuery",
    request_body = TimedVideoQueryReq,
    responses((status = 200, body = QueryRes)),
    security(("bearerAuth" = []))
)]
pub async fn timed_video_query(State(state): State<Arc<InterfaceState>>, Json(req): Json<TimedVideoQueryReq>) -> Result<Json<QueryRes>, AppError> {
    let allowed = state.get_user_allowed_roots(&req.email).await?;
    let path_row = sqlx::query("SELECT path FROM modules WHERE id = $1")
        .bind(req.module_id)
        .fetch_optional(&state.db).await?;

    let path_str = match path_row {
        Some(row) => row.get::<Option<String>, _>("path").unwrap_or_default(),
        None => return Err(AppError::NotFound("Module not found".into())),
    };

    let path_ids: Vec<i32> = path_str.split('/').filter(|s| !s.is_empty()).filter_map(|s| s.parse().ok()).collect();

    let has_access = path_ids.iter().any(|id| allowed.contains(id));

    if !has_access {
        return Err(AppError::Forbidden("Access denied".into()));
    }

    let rows = sqlx::query("SELECT start_time, content FROM video_chunks WHERE module_id = $1 AND start_time >= $2 AND end_time <= $3")
        .bind(req.module_id).bind(req.second_start).bind(req.second_end).fetch_all(&state.db).await?;

    if rows.is_empty() {
        return Ok(Json(QueryRes { chunks: vec![] }));
    }

    let mut combined_text = String::new();
    for row in rows {
        let start: f64 = row.get("start_time");
        let content: String = row.get("content");

        combined_text.push_str(&format!("[TIMECODE: {:.0}] {} ", start, content));
    }

    let module_row = sqlx::query("SELECT url FROM modules WHERE id = $1")
        .bind(req.module_id)
        .fetch_optional(&state.db).await?;

    let module_url = match module_row {
        Some(row) => row.get::<Option<String>, _>("url").unwrap_or_default(),
        None => return Err(AppError::NotFound("Module not found".into())),
    };

    let desc = state.get_module_desc(req.module_id).await;
    let chunks = vec![QueryChunk {
        desc,
        text: combined_text.trim().to_string(),
        url: module_url
    }];

    Ok(Json(QueryRes { chunks }))
}

#[utoipa::path(
    post,
    path = "/api/v1/documentPageQuery",
    request_body = DocumentPageQueryReq,
    responses((status = 200, body = QueryRes)),
    security(("bearerAuth" = []))
)]
pub async fn document_page_query(State(state): State<Arc<InterfaceState>>, Json(req): Json<DocumentPageQueryReq>) -> Result<Json<QueryRes>, AppError> {
    let allowed = state.get_user_allowed_roots(&req.email).await?;
    let path_row = sqlx::query("SELECT path FROM modules WHERE id = $1")
        .bind(req.module_id)
        .fetch_optional(&state.db).await?;

    let path_str = match path_row {
        Some(row) => row.get::<Option<String>, _>("path").unwrap_or_default(),
        None => return Err(AppError::NotFound("Module not found".into())),
    };

    let path_ids: Vec<i32> = path_str.split('/').filter(|s| !s.is_empty()).filter_map(|s| s.parse().ok()).collect();

    let has_access = path_ids.iter().any(|id| allowed.contains(id));

    if !has_access {
        return Err(AppError::Forbidden("Access denied".into()));
    }

    let module_row = sqlx::query("SELECT url FROM modules WHERE id = $1")
        .bind(req.module_id)
        .fetch_optional(&state.db).await?;

    let module_url = match module_row {
        Some(row) => row.get::<Option<String>, _>("url").unwrap_or_default(),
        None => return Err(AppError::NotFound("Module not found".into())),
    };

    let row = sqlx::query("SELECT content FROM module_file_pages WHERE module_id = $1 AND page_number = $2")
        .bind(req.module_id).bind(req.page).fetch_one(&state.db).await?;

    let desc = state.get_module_desc(req.module_id).await;
    Ok(Json(QueryRes {
        chunks: vec![QueryChunk { desc, text: row.get("content"), url: module_url }]
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/fullContentQuery",
    request_body = FullContentQueryReq,
    responses((status = 200, body = QueryRes)),
    security(("bearerAuth" = []))
)]
pub async fn full_content_query(State(state): State<Arc<InterfaceState>>, Json(req): Json<FullContentQueryReq>) -> Result<Json<QueryRes>, AppError> {
    let allowed = state.get_user_allowed_roots(&req.email).await?;


    let mut all_chunks = vec![];

    for mid in req.module_ids {
        let allowed = state.get_user_allowed_roots(&req.email).await?;
        let path_row = sqlx::query("SELECT path FROM modules WHERE id = $1")
            .bind(mid)
            .fetch_optional(&state.db).await?;

        let path_str = match path_row {
            Some(row) => row.get::<Option<String>, _>("path").unwrap_or_default(),
            None => return Err(AppError::NotFound("Module not found".into())),
        };

        let path_ids: Vec<i32> = path_str.split('/').filter(|s| !s.is_empty()).filter_map(|s| s.parse().ok()).collect();

        let has_access = path_ids.iter().any(|id| allowed.contains(id));

        if !has_access {
            error!("User {} asked for forbidden material {}", &req.email, mid);
            continue;
        }

        let desc = state.get_module_desc(mid).await;

        let module_row = sqlx::query("SELECT url FROM modules WHERE id = $1")
            .bind(mid)
            .fetch_optional(&state.db).await?;

        let module_url = match module_row {
            Some(row) => row.get::<Option<String>, _>("url").unwrap_or_default(),
            None => return Err(AppError::NotFound("Module not found".into())),
        };

        let text_rows = sqlx::query("SELECT content FROM module_text WHERE module_id = $1").bind(mid).fetch_all(&state.db).await?;
        for r in text_rows { all_chunks.push(QueryChunk { desc: desc.clone(), text: r.get("content"), url: module_url.clone() }); }

        let file_rows = sqlx::query("SELECT content FROM module_file_pages WHERE module_id = $1 ORDER BY page_number").bind(mid).fetch_all(&state.db).await?;
        for r in file_rows { all_chunks.push(QueryChunk { desc: desc.clone(), text: r.get("content"), url: module_url.clone() }); }
    }

    Ok(Json(QueryRes { chunks: all_chunks }))
}

#[utoipa::path(
    post,
    path = "/api/v1/availableStructure",
    request_body = AvailableStructureReq,
    responses((status = 200, body = StructureRes)),
    security(("bearerAuth" = []))
)]
pub async fn available_structure(State(state): State<Arc<InterfaceState>>, Json(req): Json<AvailableStructureReq>) -> Result<Json<StructureRes>, AppError> {
    let allowed_roots = state.get_user_allowed_roots(&req.email).await?;

    let roots = if let Some(req_ids) = req.module_ids {
        req_ids.into_iter().filter(|id| allowed_roots.contains(id)).collect::<Vec<_>>()
    } else {
        allowed_roots
    };

    if roots.is_empty() { return Ok(Json(StructureRes { modules: vec![] })); }

    let mut rows = sqlx::query(
        "WITH RECURSIVE tree AS (
            SELECT m.id, m.name, m.type, m.parent_id, m.path, mf.page_count, mv.total_time
            FROM modules m
            LEFT JOIN module_file mf ON m.id = mf.module_id
            LEFT JOIN module_video mv ON m.id = mv.module_id
            WHERE m.id = ANY($1)
            UNION ALL
            SELECT m.id, m.name, m.type, m.parent_id, m.path, mf.page_count, mv.total_time
            FROM modules m
            JOIN tree t ON m.parent_id = t.id
            LEFT JOIN module_file mf ON m.id = mf.module_id
            LEFT JOIN module_video mv ON m.id = mv.module_id
        ) SELECT DISTINCT * FROM tree"
    )
        .bind(&roots)
        .fetch_all(&state.db).await?;

    let mut nodes: std::collections::HashMap<i32, ModuleNode> = rows.iter().map(|r| {
        let id: i32 = r.get("id");
        (id, ModuleNode {
            id,
            name: r.get("name"),
            r#type: r.get("type"),
            page_count: r.get("page_count"),
            video_length: r.get("total_time"),
            sub_modules: vec![]
        })
    }).collect();

    rows.sort_by_cached_key(|r| {
        r.get::<Option<String>, _>("path")
            .unwrap_or_default()
            .split('/')
            .count()
    });
    rows.reverse();

    for r in &rows {
        let id: i32 = r.get("id");
        let parent_id: Option<i32> = r.get("parent_id");

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

    let mut result_roots: Vec<ModuleNode> = nodes.into_values().collect();

    result_roots.sort_by_key(|n| n.id);

    Ok(Json(StructureRes { modules: result_roots }))
}