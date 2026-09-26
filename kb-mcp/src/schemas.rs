use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SemanticSearchArgs {
    pub semantic_query: String,
    #[serde(default)]
    pub module_ids: Vec<i32>,
    /// Опциональный список ключевых слов для жёсткой текстовой фильтрации (например, фамилии авторов,
    /// точные термины, формулы, аббревиатуры вроде "РТУ", "LSTM").
    /// Если строгая фильтрация по словам не требуется — ОСТАВЬТЕ ПУСТЫМ ([]).
    #[serde(default)]
    pub keywords: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TimedVideoArgs {
    pub module_id: i32,
    /// Время начала интересующего фрагмента лекции в секундах (например, 120.0 для 2-й минуты).
    pub second_start: f64,
    /// Время окончания интересующего фрагмента лекции в секундах (например, 300.0 для 5-й минуты).
    pub second_end: f64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocumentPageArgs {
    pub module_id: i32,
    pub page: i32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FullContentArgs {
    /// Список ID модулей (текстовых заметок, статей или документов),
    /// всё текстовое содержимое которых необходимо выгрузить целиком.
    /// ВНИМАНИЕ: Используйте осторожно и только для небольших модулей, чтобы не перегрузить контекст.
    pub module_ids: Vec<i32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AvailableStructureArgs {
    /// Список ID модулей/курсов, для которых нужно получить поддерево.
    /// Оставьте пустым ([]), чтобы получить всё доступное дерево от корня.
    #[serde(default)]
    pub module_ids: Vec<i32>,
}

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ToolDefinition {
    pub name: &'static str,
    pub description: &'static str,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}