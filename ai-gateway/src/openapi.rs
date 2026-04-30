use utoipa::OpenApi;
use crate::{schemas::*, handlers::*};

#[derive(OpenApi)]
#[openapi(
    paths(
        system::health, system::metrics,
        generation::llm, generation::vlm, generation::embeddings,
        audio::asr,
        admin::reload, admin::status
    ),
    components(schemas(LLMRequest, ChatMessage, LLMResponse, VLMRequest, EmbeddingRequest, EmbeddingResponse, AudioUpload)),
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