use utoipa::OpenApi;
use crate::{schemas::*, handlers::*};
use common_infra::{system_handlers, openapi::SecurityAddon};

#[derive(OpenApi)]
#[openapi(
    paths(
        system_handlers::health, system_handlers::metrics,
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