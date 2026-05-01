use utoipa::OpenApi;
use crate::{schemas::*, handlers::checker::*};
use common_infra::{system_handlers, openapi::SecurityAddon};

#[derive(OpenApi)]
#[openapi(
    paths(
        system_handlers::health,
        submit_check, get_results,
        get_submitted_students, get_failed_students,
        get_submission_text, get_submission_verdict
    ),
    components(schemas(
        AssignmentMessage, SubmitResponse, ResultsRequest, ResultItem,
        VerdictResponse, SubmissionTextResponse, CheckMode
    )),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;