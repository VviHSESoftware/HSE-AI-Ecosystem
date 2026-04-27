use utoipa::OpenApi;
use crate::{schemas::*, handlers::*};

#[derive(OpenApi)]
#[openapi(
    paths(
        system::health, system::metrics,
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