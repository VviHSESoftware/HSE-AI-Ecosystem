use graphile_worker::{IntoTaskHandlerResult, TaskHandler, WorkerContext};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::schemas::UniversalTaskRequest;
use crate::state::AppState;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UniversalTask {
    pub task_id: Uuid,
    pub req: UniversalTaskRequest,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NormalTask(pub UniversalTask);

impl TaskHandler for NormalTask {
    const IDENTIFIER: &'static str = "ai_normal";
    async fn run(self, ctx: WorkerContext) -> impl IntoTaskHandlerResult {
        process_any_task(self.0, ctx).await
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PreciseTask(pub UniversalTask);

impl TaskHandler for PreciseTask {
    const IDENTIFIER: &'static str = "ai_precise";
    async fn run(self, ctx: WorkerContext) -> impl IntoTaskHandlerResult {
        process_any_task(self.0, ctx).await
    }
}

async fn process_any_task(data: UniversalTask, ctx: WorkerContext) -> Result<(), String> {
    let state = ctx.get_ext::<Arc<AppState>>()
        .expect("AppState must be provided in worker context");

    let result = crate::engine::run_managed_task(state, data.task_id, &data.req.payload, &data.req.mode).await;

    match result {
        Ok(_) => Ok(()),
        Err(err) if err.is_retriable() => Err(err.to_string()),
        Err(_) => Ok(())
    }
}