use std::collections::HashSet;
use std::sync::Arc;
use reqwest::Client;

use crate::config::AppEnv;
use crate::services::admin::AdminService;
use crate::services::module::ModuleService;
use common_infra::HasApiTokens;
use kb_common::repository::KbRepository;
use ai_gateway_client::AiGatewayClient;
use doc_processor_client::DocProcessorClient;

pub struct KbControllerState {
    pub env: AppEnv,
    pub api_tokens: HashSet<String>,
    pub repo: KbRepository,

    pub admin_service: Arc<AdminService>,
    pub module_service: Arc<ModuleService>,
}

impl HasApiTokens for KbControllerState {
    fn get_api_tokens(&self) -> &HashSet<String> {
        &self.api_tokens
    }
}

impl KbControllerState {
    pub async fn new(env: AppEnv, repo: KbRepository) -> Self {
        let ai_gateway = Arc::new(AiGatewayClient::new(
            env.ai_gateway_url.clone(),
            env.ai_gateway_token.clone(),
        ));

        let doc_processor = Arc::new(DocProcessorClient::new(
            env.doc_processor_url.clone(),
            env.doc_processor_token.clone(),
        ));

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(600))
            .build()
            .unwrap();

        let mut api_tokens = HashSet::new();
        api_tokens.insert(env.admin_token.clone());

        let admin_service = Arc::new(AdminService::new(repo.clone()));
        let module_service = Arc::new(ModuleService::new(
            env.clone(), repo.clone(), client, ai_gateway, doc_processor
        ));

        Self { env, api_tokens, repo, admin_service, module_service }
    }
}