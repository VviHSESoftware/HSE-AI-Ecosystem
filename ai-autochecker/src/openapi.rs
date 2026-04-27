use utoipa::OpenApi;
use crate::{schemas::*, handlers::checker::*};

#[derive(OpenApi)]
#[openapi(
    paths(
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

struct SecurityAddon;
impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(comp) = openapi.components.as_mut() {
            comp.add_security_scheme(
                "bearerAuth",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::HttpBuilder::new()
                        .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                        .build()
                )
            );
        }
    }
}