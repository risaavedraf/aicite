use serde::{Deserialize, Serialize};

/// CLI exit codes matching the PRD specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum ExitCode {
    /// Success, including context, no_results, and insufficient_context
    Success = 0,
    /// Validation, config, or contract error
    Validation = 1,
    /// Not found or not ready
    NotFound = 2,
    /// Provider or external dependency failure
    Provider = 3,
    /// Runtime mode forbidden
    RuntimeForbidden = 4,
    /// Internal error
    Internal = 5,
    /// Operation in progress / durable lock conflict
    OperationInProgress = 6,
    /// Rate limit exceeded
    RateLimitExceeded = 7,
}
