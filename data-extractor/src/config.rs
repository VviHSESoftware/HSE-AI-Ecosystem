#[derive(Debug, Clone)]
pub struct AppEnv {
    pub project_name: String,
    pub app_version: String,
    pub webhook_token: String,

    pub moodle_url: String,
    pub moodle_wstoken: String,

    pub kb_url: String,
    pub kb_system_token: String,
    pub kb_system_name: String,
}

impl AppEnv {
    pub fn load() -> Self {
        dotenvy::dotenv().ok();

        Self {
            project_name: std::env::var("PROJECT_NAME").unwrap_or_else(|_| "HSE Moodle Extractor".into()),
            app_version: std::env::var("APP_VERSION").unwrap_or_else(|_| "1.0.0".into()),
            webhook_token: std::env::var("WEBHOOK_TOKEN").unwrap_or_default(),
            moodle_url: std::env::var("MOODLE_URL").expect("MOODLE_URL must be set"),
            moodle_wstoken: std::env::var("MOODLE_WSTOKEN").expect("MOODLE_WSTOKEN must be set"),
            kb_url: std::env::var("KB_URL").expect("KB_URL must be set"),
            kb_system_token: std::env::var("KB_SYSTEM_TOKEN").expect("KB_SYSTEM_TOKEN must be set"),
            kb_system_name: std::env::var("KB_SYSTEM_NAME").unwrap_or_else(|_| "Moodle".into()),
        }
    }
}