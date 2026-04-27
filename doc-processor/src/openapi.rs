use utoipa::OpenApi;
use crate::{schemas::*, handlers::*};

#[derive(OpenApi)]
#[openapi(
    paths(
        system::health, system::metrics,
        chunking::chunk_text, chunking::chunk_partitioned,
        parsing::parse_video, parsing::parse_document, parsing::get_supported_types
    ),
    components(schemas(
        ChunkTextRequest, ChunkTextResponse, PlainChunk,
        ChunkPartitionedTextRequest, ChunkPartitionedTextResponse, PartitionPart, PartitionedChunk,
        ParseVideoRequest, ParseVideoResponse,
        ParseDocumentUpload, ParseDocumentResponse, DocumentPagePart,
        SupportedTypesResponse
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