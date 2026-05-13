use utoipa::OpenApi;
use crate::{schemas::*, handlers::process::*, handlers::task_stats::*};
use common_infra::{system_handlers, openapi::SecurityAddon};

#[derive(OpenApi)]
#[openapi(
    paths(
        system_handlers::health, system_handlers::metrics,
        submit, get_results, process,
        get_submitted_students, get_failed_students,
        get_submission_text, get_submission_verdict
    ),
    components(schemas(
        UniversalTaskRequest, TaskPayload, CheckMode,
        SubmitResponse, ResultsRequest, ResultItem, SingleResponse,
        VerdictResponse, SubmissionTextResponse, AutocheckOutput, QuizGenOutput
    )),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;