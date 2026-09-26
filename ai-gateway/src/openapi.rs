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
        generation::llm, generation::vlm, generation::embeddings,
        openai::openai_chat, openai::list_models,
        audio::asr,
        admin::reload, admin::status
    ),
    components(schemas(LLMRequest, ChatMessage, LLMResponse, VLMRequest, EmbeddingRequest, EmbeddingResponse, AudioUpload)),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;