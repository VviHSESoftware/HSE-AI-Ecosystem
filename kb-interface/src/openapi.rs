use utoipa::OpenApi;
use crate::{schemas::*, handlers::*};
use common_infra::{system_handlers, openapi::SecurityAddon};

#[derive(OpenApi)]
#[openapi(
    paths(
        system_handlers::health, system_handlers::metrics,
        query::semantic_query, query::timed_video_query,
        query::document_page_query, query::full_content_query, query::available_structure,
    ),
    components(
        schemas(
            SemanticQueryReq, TimedVideoQueryReq, DocumentPageQueryReq,
            FullContentQueryReq, QueryChunk, QueryRes,
            AvailableStructureReq, ModuleNode, StructureRes
        )
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;