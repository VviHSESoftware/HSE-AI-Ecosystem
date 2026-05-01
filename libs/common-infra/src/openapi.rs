#[cfg(feature = "openapi")]
pub struct SecurityAddon;

#[cfg(feature = "openapi")]
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