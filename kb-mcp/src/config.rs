use std::env;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub kb_interface_url: String,
    pub kb_interface_token: String,
    pub mcp_bearer_token: String,
}

impl AppConfig {
    pub fn load() -> Self {
        dotenvy::dotenv().ok();

        Self {
            kb_interface_url: env::var("KB_INTERFACE_URL")
                .unwrap_or_else(|_| "http://localhost:8000".into()),
            kb_interface_token: env::var("KB_INTERFACE_TOKEN")
                .expect("KB_INTERFACE_TOKEN is required"),
            mcp_bearer_token: env::var("MCP_BEARER_TOKEN")
                .expect("MCP_BEARER_TOKEN is required to secure public access"),
        }
    }
}