use std::collections::HashSet;
use std::sync::Arc;
use crate::config::AppEnv;
use crate::services::query::QueryService;
use common_infra::HasApiTokens;
use kb_common::repository::KbRepository;
use reqwest::Client;
use ai_gateway_client::AiGatewayClient;

pub struct InterfaceState {
    pub env: AppEnv,
    pub query_service: Arc<QueryService>,
}

impl HasApiTokens for InterfaceState {
    fn get_api_tokens(&self) -> &HashSet<String> {
        &self.env.api_tokens
    }
}

impl InterfaceState {
    pub fn new(env: AppEnv, repo: KbRepository) -> Self {
        let client = Client::builder().timeout(std::time::Duration::from_secs(30)).build().unwrap();

        let ai_gateway = Arc::new(AiGatewayClient::new(
            env.ai_gateway_url.clone(),
            env.ai_gateway_token.clone(),
        ));

        let query_service = Arc::new(QueryService::new(env.clone(), repo, client, ai_gateway));

        Self {
            env,
            query_service,
        }
    }
}