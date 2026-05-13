use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};
use crate::repository::AutocheckRepository;

pub fn spawn_metrics_collector(repo: Arc<AutocheckRepository>) {
    tokio::spawn(async move {
        info!("Starting background metrics collector task...");

        let task_types = vec!["autocheck", "quiz_gen"];
        let modes = vec!["normal", "precise"];
        let business_statuses = vec!["pending", "processing"];

        let queue_types = vec!["ai_normal", "ai_precise"];
        let queue_states = vec!["pending", "processing", "failed", "retrying", "scheduled"];

        loop {
            collect_queue_metrics(&repo, &queue_types, &queue_states).await;

            collect_business_metrics(&repo, &task_types, &modes, &business_statuses).await;

            tokio::time::sleep(Duration::from_secs(15)).await;
        }
    });
}

async fn collect_queue_metrics(
    repo: &AutocheckRepository,
    types: &[&str],
    states: &[&str]
) {
    let mut stats_map = std::collections::HashMap::new();

    for t in types {
        for s in states {
            stats_map.insert((t.to_string(), s.to_string()), 0i64);
        }
    }

    match repo.get_queue_depth().await {
        Ok(stats) => {
            for stat in stats {
                stats_map.insert((stat.task_type, stat.state), stat.count);
            }
        }
        Err(e) => error!("Metrics Error (queue): {}", e),
    }

    for ((task_type, state), count) in stats_map {
        metrics::gauge!(
            "graphile_queue_depth",
            "state" => state,
            "mode" => task_type
        ).set(count as f64);
    }
}

async fn collect_business_metrics(
    repo: &AutocheckRepository,
    types: &[&str],
    modes: &[&str],
    statuses: &[&str]
) {
    match repo.get_active_tasks_stats().await {
        Ok(stats) => {
            let mut current_stats = std::collections::HashMap::new();
            for stat in stats {
                current_stats.insert((stat.task_type, stat.mode, stat.status), stat.count);
            }

            for t in types {
                for m in modes {
                    for s in statuses {
                        let count = *current_stats
                            .get(&(t.to_string(), m.to_string(), s.to_string()))
                            .unwrap_or(&0);

                        metrics::gauge!(
                            "ai_task_business_status",
                            "status" => s.to_string(),
                            "type" => t.to_string(),
                            "mode" => m.to_string()
                        ).set(count as f64);
                    }
                }
            }
        }
        Err(e) => error!("Metrics Error (business): {}", e),
    }
}