#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub id: i32,
    pub external_type: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct VideoChunkData {
    pub start_time: f64,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct StructureRowData {
    pub id: i32,
    pub name: String,
    pub r#type: String,
    pub parent_id: Option<i32>,
    pub path: Option<String>,
    pub page_count: Option<i32>,
    pub total_time: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct NewVideoChunk {
    pub start: f64,
    pub end: f64,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct NewFilePage {
    pub page_number: i32,
    pub content: String,
}