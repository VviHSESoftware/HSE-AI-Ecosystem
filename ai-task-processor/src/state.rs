use std::collections::{HashMap, HashSet};
use std::fmt;
use crate::config::AppEnv;
use sqlx::PgPool;
use std::sync::{Arc};
use graphile_worker::WorkerUtils;
use ai_gateway_client::{AiGateway, AiGatewayClient};
use doc_processor_client::{DocProcessor, DocProcessorClient};
use common_infra::HasApiTokens;
use crate::repository::AutocheckRepository;

pub struct AppState {
    pub env: AppEnv,
    pub repo: Arc<AutocheckRepository>,
    pub ai_gateway: Arc<dyn AiGateway>,
    pub doc_processor: Arc<dyn DocProcessor>,
    pub worker_utils: WorkerUtils,
    pub prompts: HashMap<String, String>,
}

impl fmt::Debug for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppState")
            .field("env", &self.env)
            .field("repo", &self.repo)
            .field("prompts", &self.prompts)
            .field("ai_gateway", &"Arc<dyn AiGateway>")
            .field("doc_processor", &"Arc<dyn DocProcessor>")
            .field("worker_utils", &"WorkerUtils")
            .finish()
    }
}

impl HasApiTokens for AppState {
    fn get_api_tokens(&self) -> &HashSet<String> {
        &self.env.api_tokens
    }
}

impl AppState {
    pub fn new(env: AppEnv, db: PgPool) -> Self {
        let prompts = Self::load_prompts(&env.prompts_dir);

        let ai_gateway = Arc::new(AiGatewayClient::new(
            env.ai_gateway_url.clone(),
            env.ai_gateway_token.clone(),
        ));

        let doc_processor = Arc::new(DocProcessorClient::new(
            env.doc_processor_url.clone(),
            env.doc_processor_token.clone(),
        ));

        let repo = Arc::new(AutocheckRepository::new(db.clone()));

        let worker_utils = WorkerUtils::new(db, "graphile_worker".to_string());

        Self {
            env,
            repo,
            ai_gateway,
            doc_processor,
            worker_utils,
            prompts,
        }
    }

    fn load_prompts(dir: &str) -> HashMap<String, String> {
        let mut map = HashMap::new();
        let types = vec!["autocheck", "quiz_gen"];

        for t in types {
            let path = format!("{}/{}.txt", dir, t);
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    tracing::info!("Loaded prompt for type <{}> from {}", t, path);
                    map.insert(t.to_string(), content);
                }
                Err(e) => {
                    tracing::error!("CRITICAL: Failed to load prompt for <{}> at {}: {}", t, path, e);
                    map.insert(t.to_string(), "You are a helpful assistant.".into());
                }
            }
        }
        map
    }

    pub fn get_prompt(&self, task_type: &str) -> &str {
        self.prompts.get(task_type)
            .map(|s| s.as_str())
            .unwrap_or("You are a helpful assistant.")
    }
}