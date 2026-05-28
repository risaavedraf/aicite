//! Runtime-mode enforcement guards.
//!
//! Centralises the decision of which operations are allowed in each
//! [`config::RuntimeMode`].

use common::CiteError;
use config::RuntimeMode;

/// Check whether the configured embedding provider is a "real" external provider
/// (as opposed to eval/golden/mock test providers).
///
/// Returns `true` for providers that send data to external services
/// (e.g., `openai-compatible`, `gemini`).
pub fn is_real_provider(provider_id: &str) -> bool {
    !matches!(provider_id, "eval" | "golden" | "mock" | "test")
}

/// Check whether **ingest** is allowed in the given runtime mode.
///
/// - `LocalPrivateDemo` — allowed.
/// - `PublicPackagedDemo` — forbidden (read-only demo).
/// - `Production` — forbidden (ingest handled through deployment pipeline).
pub fn check_ingest_allowed(mode: &RuntimeMode) -> Result<(), CiteError> {
    match mode {
        RuntimeMode::LocalPrivateDemo => Ok(()),
        RuntimeMode::PublicPackagedDemo => Err(CiteError::RuntimeModeForbidden {
            message: "Ingest is not allowed in public_packaged_demo mode".to_string(),
        }),
        RuntimeMode::Production => Err(CiteError::RuntimeModeForbidden {
            message: "Ingest is not allowed in production mode".to_string(),
        }),
    }
}
