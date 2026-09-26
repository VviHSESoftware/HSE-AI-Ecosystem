pub use doc_processor_client::types::*;

#[derive(serde::Deserialize)]
pub struct IpynbNotebook {
    pub cells: Option<Vec<IpynbCell>>,
}

#[derive(serde::Deserialize)]
pub struct IpynbCell {
    pub cell_type: String,
    pub source: Option<IpynbSource>,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
pub enum IpynbSource {
    Array(Vec<String>),
    String(String),
}