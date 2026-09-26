use axum::{extract::State, Json};
use schemars::schema_for;
use serde_json::json;
use std::sync::Arc;
use crate::kb_client::KbClient;
use crate::middleware::CURRENT_USER_EMAIL;
use crate::schemas::*;

pub struct AppState {
    pub kb_client: KbClient,
}

pub fn get_tools_manifest() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "semantic_search",
            description: "Поиск учебных материалов, страниц и контента базы знаний по семантическому смыслу.",
            input_schema: serde_json::to_value(schema_for!(SemanticSearchArgs)).unwrap(),
        },
        ToolDefinition {
            name: "timed_video_query",
            description: "Получение текстового транскрипта фрагмента лекции по секундам начала и конца.",
            input_schema: serde_json::to_value(schema_for!(TimedVideoArgs)).unwrap(),
        },
        ToolDefinition {
            name: "document_page_query",
            description: "Получение содержимого конкретной страницы документа или PDF-файла по номеру страницы.",
            input_schema: serde_json::to_value(schema_for!(DocumentPageArgs)).unwrap(),
        },
        ToolDefinition {
            name: "full_content_query",
            description: "Получение полного текстового содержимого выбранных модулей курсов.",
            input_schema: serde_json::to_value(schema_for!(FullContentArgs)).unwrap(),
        },
        ToolDefinition {
            name: "available_structure",
            description: "Получение доступной пользователю иерархической структуры курсов, папок и материалов.",
            input_schema: serde_json::to_value(schema_for!(AvailableStructureArgs)).unwrap(),
        },
    ]
}

pub async fn handle_json_rpc(
    State(state): State<Arc<AppState>>,
    Json(rpc_req): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    let id = rpc_req.id.clone();

    match rpc_req.method.as_str() {
        "initialize" => Json(JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "hse-kb-mcp", "version": "1.0.0" }
            })),
            error: None,
        }),

        "tools/list" => {
            let tools = get_tools_manifest();
            Json(JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: Some(json!({ "tools": tools })),
                error: None,
            })
        }

        "tools/call" => {
            let tool_name = rpc_req.params["name"].as_str().unwrap_or_default();
            let args = rpc_req.params.get("arguments").cloned().unwrap_or(json!({}));
            
            let email = CURRENT_USER_EMAIL.with(|e| e.clone());

            if email == "anonymous" {
                return Json(JsonRpcResponse {
                    jsonrpc: "2.0",
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32000,
                        message: "X-User-Email header is required to execute tools".into(),
                    }),
                });
            }

            let result = execute_tool(&state.kb_client, &email, tool_name, args).await;

            match result {
                Ok(content) => Json(JsonRpcResponse {
                    jsonrpc: "2.0",
                    id,
                    result: Some(json!({
                        "content": [{ "type": "text", "text": content.to_string() }]
                    })),
                    error: None,
                }),
                Err(err_msg) => Json(JsonRpcResponse {
                    jsonrpc: "2.0",
                    id,
                    result: None,
                    error: Some(JsonRpcError { code: -32000, message: err_msg }),
                }),
            }
        }

        _ => Json(JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError { code: -32601, message: "Method not found".into() }),
        }),
    }
}

async fn execute_tool(
    kb: &KbClient,
    email: &str,
    name: &str,
    args: serde_json::Value,
) -> Result<serde_json::Value, String> {
    match name {
        "semantic_search" => {
            let p: SemanticSearchArgs = serde_json::from_value(args).map_err(|e| e.to_string())?;
            kb.semantic_query(email, p.semantic_query, p.module_ids, p.keywords).await
        }
        "timed_video_query" => {
            let p: TimedVideoArgs = serde_json::from_value(args).map_err(|e| e.to_string())?;
            kb.timed_video_query(email, p.module_id, p.second_start, p.second_end).await
        }
        "document_page_query" => {
            let p: DocumentPageArgs = serde_json::from_value(args).map_err(|e| e.to_string())?;
            kb.document_page_query(email, p.module_id, p.page).await
        }
        "full_content_query" => {
            let p: FullContentArgs = serde_json::from_value(args).map_err(|e| e.to_string())?;
            kb.full_content_query(email, p.module_ids).await
        }
        "available_structure" => {
            let p: AvailableStructureArgs = serde_json::from_value(args).map_err(|e| e.to_string())?;
            let filter = if p.module_ids.is_empty() { None } else { Some(p.module_ids) };
            kb.available_structure(email, filter).await
        }
        _ => Err(format!("Unknown tool: {}", name)),
    }
}