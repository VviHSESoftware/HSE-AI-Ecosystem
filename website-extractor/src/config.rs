use std::time::Duration;

#[derive(Debug, Clone)]
pub struct AppEnv {
    pub kb_url: String,
    pub kb_system_token: String,
    pub kb_system_name: String,
    pub kb_root_parent_id: i32,
    pub sync_interval: Duration,
    pub target_url: String,
    pub max_depth: usize,
}

impl AppEnv {
    pub fn load() -> Self {
        dotenvy::dotenv().ok();

        let interval_secs: u64 = std::env::var("SYNC_INTERVAL_SECS")
            .unwrap_or_else(|_| "86400".into())
            .parse()
            .unwrap_or(86400);
        let max_depth: usize = std::env::var("CRAWLER_MAX_DEPTH")
            .unwrap_or_else(|_| "1".into())
            .parse()
            .unwrap_or(1);

        Self {
            kb_url: std::env::var("KB_URL").expect("KB_URL is required"),
            kb_system_token: std::env::var("KB_SYSTEM_TOKEN").expect("KB_SYSTEM_TOKEN is required"),
            kb_system_name: std::env::var("KB_SYSTEM_NAME").unwrap_or_else(|_| "HSE_Olymp".into()),
            kb_root_parent_id: std::env::var("KB_ROOT_PARENT_ID")
                .expect("KB_ROOT_PARENT_ID is required")
                .parse()
                .expect("KB_ROOT_PARENT_ID must be an integer"),
            sync_interval: Duration::from_secs(interval_secs),
            target_url: std::env::var("TARGET_URL")
                .unwrap_or_else(|_| "https://olymp.hse.ru/championship".into()),
            max_depth,
        }
    }
}