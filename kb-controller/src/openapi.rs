use utoipa::OpenApi;
use crate::{schemas::*, handlers::*};
use common_infra::{system_handlers, openapi::SecurityAddon};

#[derive(OpenApi)]
#[openapi(
    paths(
        system_handlers::health, system_handlers::metrics,
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