use utoipa::OpenApi;
use crate::{schemas::*, handlers::*};

#[derive(OpenApi)]
#[openapi(
    paths(
        system::health, system::metrics,
        modules::add_module_base, modules::add_module_access,
        modules::set_video_type, modules::set_text_type, modules::set_document_type,
        modules::get_module_id, modules::get_system_id, modules::invalidate_module,
        modules::get_supported_file_types,
        admin::create_system
    ),
    components(schemas(
        AddModuleBaseReq, ModuleIdRes, AddModuleAccessReq, SetVideoTypeReq, SetDocumentTypeUpload, SetTextTypeReq,
        GetModuleIdReq, GetSystemIdReq, InvalidateModuleReq, SupportedFileTypesRes, CreateSystemReq, CreateSystemRes
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