//! OpenAPI 3.1 specification for the Stadera HTTP API.
//!
//! [`ApiDoc`] is the single source of truth: each handler is annotated
//! with `#[utoipa::path(...)]` (in its own module) and listed here in
//! `paths(...)`. Schemas referenced by responses/request bodies are
//! collected in `components(schemas(...))`.
//!
//! The spec is served as JSON at `/api-docs/openapi.json` and rendered
//! by Swagger UI at `/docs`. The frontend consumes the JSON with
//! `openapi-typescript` to generate a typed client at build time.

use utoipa::Modify;
use utoipa::OpenApi;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Stadera API",
        version = "0.1.0",
        description = "Personal weight tracking and nutrition coaching API.",
        license(name = "MIT")
    ),
    paths(
        crate::routes::health::health,
        crate::routes::me::me,
        crate::routes::today::today,
        crate::routes::trend::trend,
        crate::routes::history::history,
        crate::routes::profile::get_profile,
        crate::routes::profile::put_profile,
    ),
    components(
        schemas(
            crate::error::ErrorBody,
            crate::dto::UserView,
            crate::dto::MeasurementView,
            crate::dto::DailyTargetView,
            crate::dto::ProfileView,
            crate::routes::health::HealthResponse,
            crate::routes::me::MeResponse,
            crate::routes::today::TodayResponse,
            crate::routes::trend::TrendResponse,
            crate::routes::history::HistoryResponse,
            crate::routes::profile::ProfilePayload,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "health", description = "System health"),
        (name = "user", description = "Current authenticated user"),
        (name = "measurements", description = "Weight measurements and aggregate trends"),
        (name = "profile", description = "Metabolic profile (height, sex, activity, goal)")
    )
)]
pub struct ApiDoc;

/// Adds the `session_cookie` security scheme to the spec components.
/// Endpoints opt in via `security(("session_cookie" = []))` on their
/// `#[utoipa::path]` attribute.
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi
            .components
            .as_mut()
            .expect("components are populated by the derive");
        components.add_security_scheme(
            "session_cookie",
            SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new("stadera_session"))),
        );
    }
}
