use utoipa::OpenApi;
use crate::{schemas::*, handlers::*};
use common_infra::{system_handlers, openapi::SecurityAddon};

#[derive(OpenApi)]
#[openapi(
    info(
        title = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION"),
    ),
    paths(
        system_handlers::health, system_handlers::metrics,
        chat::process_request
    ),
    components(
        schemas(
            RouterRequest, ChatMessage, RouterResponse, SourceMaterial
        )
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;