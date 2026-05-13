use crate::config::AppEnv;
use crate::services::chunking::ChunkingService;
use crate::services::parsing::ParsingService;
use ai_gateway_client::AiGatewayClient;
use common_infra::HasApiTokens;
use std::collections::HashSet;
use std::sync::Arc;

pub struct AppState {
    pub env: AppEnv,
    pub chunking: Arc<ChunkingService>,
    pub parsing: Arc<ParsingService>,
}

impl HasApiTokens for AppState {
    fn get_api_tokens(&self) -> &HashSet<String> {
        &self.env.api_tokens
    }
}

impl AppState {
    pub fn new(env: AppEnv) -> Self {
        let chunking = Arc::new(ChunkingService::new());

        let ai_gateway = Arc::new(AiGatewayClient::new(
            env.ai_gateway_url.clone(),
            env.ai_gateway_token.clone(),
        ));
        
        let parsing = Arc::new(ParsingService::new(
            env.clone(),
            ai_gateway
        ));

        Self {
            env,
            chunking,
            parsing,
        }
    }
}