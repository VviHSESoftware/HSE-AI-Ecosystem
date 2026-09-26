mod config;
mod kb_client;
mod crawler;

use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let env = Arc::new(config::AppEnv::load());
    let kb = kb_client::KbClient::new(env.as_ref().clone());
    let crawler = crawler::Crawler::new(env.as_ref().clone(), kb);

    info!("HSE Olymp Extractor started.");
    info!("Target URL: {}", env.target_url);
    info!("Root parent module ID: {}", env.kb_root_parent_id);
    info!("Sync interval: {:?}", env.sync_interval);

    let mut interval = tokio::time::interval(env.sync_interval);

    loop {
        interval.tick().await;
        info!("Triggering scheduled synchronization...");
        crawler.run_sync().await;
        info!("Sleeping until next cycle...");
    }
}