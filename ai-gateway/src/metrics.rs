use lazy_static::lazy_static;
use prometheus::{
    register_histogram_vec, register_int_counter_vec, register_int_gauge_vec, HistogramVec,
    IntCounterVec, IntGaugeVec,
};
use serde_json::Value;

lazy_static! {
    pub static ref PROVIDER_STATUS: IntGaugeVec = register_int_gauge_vec!(
        "ai_provider_status", "Status of AI provider (1 online, 0 offline)", &["provider_id"]
    ).unwrap();

    pub static ref MODEL_STATUS: IntGaugeVec = register_int_gauge_vec!(
        "ai_model_status", "Availability of specific model", &["provider_id", "internal_name", "remote_model_id"]
    ).unwrap();

    pub static ref TOKENS_SPENT: IntCounterVec = register_int_counter_vec!(
        "ai_tokens_total", "Total AI tokens spent", &["model", "mode", "provider"]
    ).unwrap();

    pub static ref AI_REQUESTS: IntCounterVec = register_int_counter_vec!(
        "ai_requests_total", "Total AI requests", &["model", "mode", "provider", "status_code"]
    ).unwrap();

    pub static ref AI_LATENCY: HistogramVec = register_histogram_vec!(
        "ai_request_duration_seconds", "Response time", &["model", "provider"]
    ).unwrap();
}

pub fn update_gateway_status_metrics(status_report: &Value) {
    MODEL_STATUS.reset();
    PROVIDER_STATUS.reset();

    if let Some(providers) = status_report.get("providers").and_then(|p| p.as_array()) {
        for p in providers {
            let p_id = p["provider_id"].as_str().unwrap_or("unknown");
            let p_status = if p["status"] == "online" { 1 } else { 0 };
            PROVIDER_STATUS.with_label_values(&[p_id]).set(p_status);

            if let Some(models) = p.get("checked_models").and_then(|m| m.as_array()) {
                for m in models {
                    let internal = m["internal_name"].as_str().unwrap_or("");
                    let remote = m["remote_model_id"].as_str().unwrap_or("");
                    let m_status = if m["available"].as_bool().unwrap_or(false) { 1 } else { 0 };
                    MODEL_STATUS.with_label_values(&[p_id, internal, remote]).set(m_status);
                }
            }
        }
    }
}