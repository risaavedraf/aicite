//! Runtime-mode enforcement guards.
//!
//! Centralises the decision of which operations are allowed in each
//! [`config::RuntimeMode`].

use common::HarnessError;
use config::RuntimeMode;

/// Check whether **ingest** is allowed in the given runtime mode.
///
/// - `LocalPrivateDemo` — allowed.
/// - `PublicPackagedDemo` — forbidden (read-only demo).
/// - `Production` — forbidden (ingest handled through deployment pipeline).
pub fn check_ingest_allowed(mode: &RuntimeMode) -> Result<(), HarnessError> {
    match mode {
        RuntimeMode::LocalPrivateDemo => Ok(()),
        RuntimeMode::PublicPackagedDemo => Err(HarnessError::RuntimeModeForbidden {
            message: "Ingest is not allowed in public_packaged_demo mode".to_string(),
        }),
        RuntimeMode::Production => Err(HarnessError::RuntimeModeForbidden {
            message: "Ingest is not allowed in production mode".to_string(),
        }),
    }
}
