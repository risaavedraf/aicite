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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_code_values() {
        assert_eq!(ExitCode::Success as i32, 0);
        assert_eq!(ExitCode::Validation as i32, 1);
        assert_eq!(ExitCode::NotFound as i32, 2);
        assert_eq!(ExitCode::Provider as i32, 3);
        assert_eq!(ExitCode::RuntimeForbidden as i32, 4);
        assert_eq!(ExitCode::Internal as i32, 5);
        assert_eq!(ExitCode::OperationInProgress as i32, 6);
        assert_eq!(ExitCode::RateLimitExceeded as i32, 7);
    }

    #[test]
    fn exit_code_partial_eq() {
        assert_eq!(ExitCode::Success, ExitCode::Success);
        assert_ne!(ExitCode::Success, ExitCode::Validation);
        assert_ne!(ExitCode::NotFound, ExitCode::Internal);
    }

    #[test]
    fn exit_code_copy_and_debug() {
        let code = ExitCode::Success;
        let copied = code; // Copy, not move
        assert_eq!(code, copied);
        assert_eq!(format!("{code:?}"), "Success");
    }

    #[test]
    fn exit_code_serde_roundtrip() {
        let variants = [
            ExitCode::Success,
            ExitCode::Validation,
            ExitCode::NotFound,
            ExitCode::Provider,
            ExitCode::RuntimeForbidden,
            ExitCode::Internal,
            ExitCode::OperationInProgress,
            ExitCode::RateLimitExceeded,
        ];
        for v in &variants {
            let json = serde_json::to_string(v).unwrap();
            let deserialized: ExitCode = serde_json::from_str(&json).unwrap();
            assert_eq!(*v, deserialized);
        }
    }
}
