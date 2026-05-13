use serde_json::Value;

pub fn update_gateway_status_metrics(status_report: &Value) {
    if let Some(providers) = status_report.get("providers").and_then(|p| p.as_array()) {
        for p in providers {
            let p_id = p["provider_id"].as_str().unwrap_or("unknown").to_string();
            let p_status = if p["status"] == "online" { 1 } else { 0 };
            metrics::gauge!("ai_provider_status", "provider_id" => p_id.clone()).set(p_status);

            if let Some(models) = p.get("checked_models").and_then(|m| m.as_array()) {
                for m in models {
                    let internal = m["internal_name"].as_str().unwrap_or("").to_string();
                    let remote = m["remote_model_id"].as_str().unwrap_or("").to_string();
                    let m_status = if m["available"].as_bool().unwrap_or(false) { 1 } else { 0 };

                    metrics::gauge!(
                        "ai_model_status",
                        "provider_id" => p_id.clone(),
                        "internal_name" => internal,
                        "remote_model_id" => remote
                    ).set(m_status);
                }
            }
        }
    }
}