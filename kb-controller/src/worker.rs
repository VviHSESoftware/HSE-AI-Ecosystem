use graphile_worker::{IntoTaskHandlerResult, TaskHandler, WorkerContext};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info, warn};
use common_infra::AppError;
use crate::state::KbControllerState;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ModuleJobPayload {
    Text {
        module_id: i32,
        text: String,
    },
    VideoUrl {
        module_id: i32,
        video_url: String,
    },
    DocumentFile {
        module_id: i32,
        filename: String,
        temp_path: String,
    },
    VideoFile {
        module_id: i32,
        filename: String,
        temp_path: String,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessModuleTask {
    pub payload: ModuleJobPayload,
}

impl TaskHandler for ProcessModuleTask {
    const IDENTIFIER: &'static str = "process_kb_module";

    async fn run(self, ctx: WorkerContext) -> impl IntoTaskHandlerResult {
        process_task(self, ctx).await
    }
}

fn is_retriable_error(err: &AppError) -> bool {
    let msg = err.to_string().to_lowercase();
    msg.contains("429")
        || msg.contains("rate limit")
        || msg.contains("timeout")
        || msg.contains("502")
        || msg.contains("503")
        || msg.contains("504")
        || msg.contains("connection refused")
        || msg.contains("reset by peer")
}

async fn process_task(task: ProcessModuleTask, ctx: WorkerContext) -> Result<(), String> {
    let state = ctx.get_ext::<Arc<KbControllerState>>()
        .expect("KbControllerState must be provided in worker context");

    info!("Starting background processing of module task: {:?}", match &task.payload {
        ModuleJobPayload::Text { module_id, .. } => format!("Text (module {})", module_id),
        ModuleJobPayload::VideoUrl { module_id, video_url } => format!("Video URL {} (module {})", video_url, module_id),
        ModuleJobPayload::DocumentFile { module_id, filename, .. } => format!("Document {} (module {})", filename, module_id),
        ModuleJobPayload::VideoFile { module_id, filename, .. } => format!("Video File {} (module {})", filename, module_id),
    });

    let res = match &task.payload {
        ModuleJobPayload::Text { module_id, text } => {
            state.module_service.process_text_type(*module_id, text.clone()).await
        }
        ModuleJobPayload::VideoUrl { module_id, video_url } => {
            state.module_service.process_video_type(*module_id, video_url.clone()).await
        }
        ModuleJobPayload::DocumentFile { module_id, filename, temp_path } => {
            let read_res = tokio::fs::read(temp_path).await
                .map_err(|e| AppError::Internal(format!("Failed to read temp doc file: {}", e)));

            let process_res = match read_res {
                Ok(bytes) => state.module_service.process_document_type(*module_id, filename.clone(), bytes).await,
                Err(err) => Err(err),
            };

            if process_res.is_ok() || !is_retriable_error(process_res.as_ref().unwrap_err()) {
                let _ = tokio::fs::remove_file(temp_path).await;
            }
            process_res
        }
        ModuleJobPayload::VideoFile { module_id, filename, temp_path } => {
            let read_res = tokio::fs::read(temp_path).await
                .map_err(|e| AppError::Internal(format!("Failed to read temp video file: {}", e)));

            let process_res = match read_res {
                Ok(bytes) => state.module_service.process_video_file_type(*module_id, filename.clone(), bytes).await,
                Err(err) => Err(err),
            };

            if process_res.is_ok() || !is_retriable_error(process_res.as_ref().unwrap_err()) {
                let _ = tokio::fs::remove_file(temp_path).await;
            }
            process_res
        }
    };

    match res {
        Ok(_) => {
            info!("Module background processing completed successfully");
            Ok(())
        }
        Err(err) if is_retriable_error(&err) => {
            warn!(reason = %err, "Retriable error during module processing. Re-scheduling task...");
            Err(err.to_string())
        }
        Err(err) => {
            error!(reason = %err, "Non-retriable error during module processing. Task dropped.");
            Ok(())
        }
    }
}