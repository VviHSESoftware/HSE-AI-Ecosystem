
#[derive(Debug, Clone)]
pub struct AppEnv {
    pub database_url: String,

    pub ai_gateway_url: String,
    pub ai_gateway_token: String,

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
            database_url: std::env::var("DATABASE_URL").expect("DATABASE_URL is required"),
            ai_gateway_url: std::env::var("AI_GATEWAY_URL").expect("AI_GATEWAY_URL is required"),
            ai_gateway_token: std::env::var("AI_GATEWAY_TOKEN").expect("AI_GATEWAY_TOKEN is required"),
            doc_processor_url: std::env::var("DOC_PROCESSOR_URL").expect("DOC_PROCESSOR_URL is required"),
            doc_processor_token: std::env::var("DOC_PROCESSOR_TOKEN").expect("DOC_PROCESSOR_TOKEN is required"),
            qdrant_url: std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://qdrant:6333".into()),
            qdrant_api_key: std::env::var("QDRANT_API_KEY").unwrap_or_default(),
            admin_token: std::env::var("ADMIN_TOKEN").unwrap_or_else(|_| "SuperAdminSecret".into()),
        }
    }
}