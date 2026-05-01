use tracing_subscriber::{EnvFilter, fmt, prelude::*};

pub fn init_tracing(project_name: &str) {
    tracing_subscriber::registry()
        .with(fmt::layer().json())
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    tracing::info!("Tracing initialized for {}", project_name);
}

pub fn get_common_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .pool_max_idle_per_host(10)
        .build()
        .expect("Failed to create HTTP client")
}