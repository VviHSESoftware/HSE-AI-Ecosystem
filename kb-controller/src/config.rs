
#[derive(Debug, Clone)]
pub struct AppEnv {
    pub database_url: String,

    pub gateway_url: String,
    pub gateway_token: String,

    pub doc_processor_url: String,
    pub doc_processor_token: String,

    pub qdrant_url: String,
    pub qdrant_api_key: String,

    pub admin_token: String,
}

impl AppEnv {
    pub fn load() -> Self {
        dotenvy::dotenv().ok();

        Self {
            database_url: std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/kb_db".into()),
            gateway_url: std::env::var("GATEWAY_URL").unwrap_or_else(|_| "http://localhost:8000".into()),
            gateway_token: std::env::var("GATEWAY_TOKEN").unwrap_or_default(),
            doc_processor_url: std::env::var("DOC_PROCESSOR_URL").unwrap_or_else(|_| "http://localhost:8001".into()),
            doc_processor_token: std::env::var("DOC_PROCESSOR_TOKEN").unwrap_or_default(),
            qdrant_url: std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://qdrant:6333".into()),
            qdrant_api_key: std::env::var("QDRANT_API_KEY").unwrap_or_default(),
            admin_token: std::env::var("ADMIN_TOKEN").unwrap_or_else(|_| "SuperAdminSecret".into()),
        }
    }
}