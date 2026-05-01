use utoipa::OpenApi;
use crate::{schemas::*, handlers::*};
use common_infra::{system_handlers, openapi::SecurityAddon};

#[derive(OpenApi)]
#[openapi(
    paths(
        system_handlers::health, system_handlers::metrics,
        generation::llm, generation::vlm, generation::embeddings,
        audio::asr,
        admin::reload, admin::status
    ),
    components(schemas(LLMRequest, ChatMessage, LLMResponse, VLMRequest, EmbeddingRequest, EmbeddingResponse, AudioUpload)),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;