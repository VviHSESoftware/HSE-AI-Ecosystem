use tracing_subscriber::{EnvFilter, fmt, prelude::*};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

pub fn register_process_metrics() -> PrometheusHandle {
    PrometheusBuilder::new()
        .install_recorder()
        .expect("failed to install prometheus recorder")
}

pub fn init_tracing(project_name: &str) {
    let filter = EnvFilter::from_default_env()
        .add_directive(tracing::Level::INFO.into()) 
        .add_directive("graphile_worker=off".parse().unwrap());

    tracing_subscriber::registry()
        .with(fmt::layer().json())
        .with(filter)
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