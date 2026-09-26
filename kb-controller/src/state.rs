use std::collections::HashSet;
use std::fmt;
use std::sync::Arc;
use reqwest::Client;
use sqlx::PgPool;
use graphile_worker::WorkerUtils;

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
    pub worker_utils: WorkerUtils,

    pub admin_service: Arc<AdminService>,
    pub module_service: Arc<ModuleService>,
}

impl fmt::Debug for KbControllerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KbControllerState")
            .field("env", &self.env)
            .field("worker_utils", &"WorkerUtils")
            .finish_non_exhaustive()
    }
}

impl HasApiTokens for KbControllerState {
    fn get_api_tokens(&self) -> &HashSet<String> {
        &self.api_tokens
    }
}

impl KbControllerState {
    pub async fn new(env: AppEnv, repo: KbRepository, pool: PgPool) -> Self {
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

        let worker_utils = WorkerUtils::new(pool, "graphile_worker".to_string());

        let admin_service = Arc::new(AdminService::new(repo.clone()));
        let module_service = Arc::new(ModuleService::new(
            env.clone(), repo.clone(), client, ai_gateway, doc_processor, worker_utils.clone()
        ));

        Self {
            env,
            api_tokens,
            repo,
            worker_utils,
            admin_service,
            module_service,
        }
    }
}