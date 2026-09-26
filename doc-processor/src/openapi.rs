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
        chunking::chunk_text, chunking::chunk_partitioned,
        parsing::parse_video_file, parsing::parse_audio_file,
        parsing::parse_video, parsing::parse_document,
        parsing::parse_file, parsing::get_supported_types
    ),
    components(schemas(
        ChunkTextRequest, ChunkTextResponse, PlainChunk,
        ChunkPartitionedTextRequest, ChunkPartitionedTextResponse, PartitionPart, PartitionedChunk,
        ParseVideoRequest, ParseVideoResponse,
        ParseVideoFileUpload, ParseAudioFileUpload,
        ParseDocumentUpload, ParseDocumentResponse, DocumentPagePart,
        SupportedTypesResponse, ParseFileResponse
    )),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;