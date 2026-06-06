use crate::exit::ExitCode;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// All errors in the AI Cite system
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum CiteError {
    #[error("Unsupported file type: {file_type}")]
    UnsupportedFileType { file_type: String },

    #[error("File too large: {size_bytes} bytes (max: {max_bytes})")]
    FileTooLarge { size_bytes: u64, max_bytes: u64 },

    #[error("File not found: {}", path.display())]
    FileNotFound { path: PathBuf },

    #[error("Document not found: {document_id}")]
    DocumentNotFound { document_id: String },

    #[error("Document not ready: {document_id}")]
    DocumentNotReady { document_id: String },

    #[error("Trace not found: {trace_id}")]
    TraceNotFound { trace_id: String },

    #[error("Citation not found: {citation_id}")]
    CitationNotFound { citation_id: String },

    #[error("Chunk not found: {chunk_id}")]
    ChunkNotFound { chunk_id: String },

    #[error("Configuration error: {message}")]
    ConfigError { message: String },

    #[error("Storage error: {message}")]
    StorageError { message: String },

    #[error("Internal error: {message}")]
    InternalError { message: String },

    #[error("Query too long: {length} chars (max: {max})")]
    QueryTooLong { length: usize, max: usize },

    #[error("Invalid parameter: {message}")]
    InvalidParameter { message: String },

    #[error("Path rejected: {message}")]
    PathRejected { message: String },

    #[error("Runtime mode forbidden: {message}")]
    RuntimeModeForbidden { message: String },

    #[error("Rate limit exceeded")]
    RateLimitExceeded { retry_after_seconds: u32 },

    #[error("Operation in progress: {message}")]
    OperationInProgress {
        message: String,
        retry_after_seconds: u32,
        lock_name: Option<String>,
    },

    #[error("Embedding provider error: {message}")]
    EmbeddingProviderError { message: String },

    #[error("Retrieval timeout")]
    RetrievalTimeout,
}

impl CiteError {
    /// Returns both the machine-readable error code and CLI exit code.
    fn code_and_exit(&self) -> (&'static str, ExitCode) {
        match self {
            Self::UnsupportedFileType { .. } => ("unsupported_file_type", ExitCode::Validation),
            Self::FileTooLarge { .. } => ("file_too_large", ExitCode::Validation),
            Self::FileNotFound { .. } => ("file_not_found", ExitCode::NotFound),
            Self::DocumentNotFound { .. } => ("document_not_found", ExitCode::NotFound),
            Self::DocumentNotReady { .. } => ("document_not_ready", ExitCode::NotFound),
            Self::TraceNotFound { .. } => ("trace_not_found", ExitCode::NotFound),
            Self::CitationNotFound { .. } => ("citation_not_found", ExitCode::NotFound),
            Self::ChunkNotFound { .. } => ("chunk_not_found", ExitCode::NotFound),
            Self::ConfigError { .. } => ("config_error", ExitCode::Validation),
            Self::StorageError { .. } => ("storage_error", ExitCode::Internal),
            Self::InternalError { .. } => ("internal_error", ExitCode::Internal),
            Self::QueryTooLong { .. } => ("query_too_long", ExitCode::Validation),
            Self::InvalidParameter { .. } => ("invalid_parameter", ExitCode::Validation),
            Self::PathRejected { .. } => ("path_rejected", ExitCode::Validation),
            Self::RuntimeModeForbidden { .. } => {
                ("runtime_mode_forbidden", ExitCode::RuntimeForbidden)
            }
            Self::RateLimitExceeded { .. } => ("rate_limit_exceeded", ExitCode::RateLimitExceeded),
            Self::OperationInProgress { .. } => {
                ("operation_in_progress", ExitCode::OperationInProgress)
            }
            Self::EmbeddingProviderError { .. } => ("embedding_provider_error", ExitCode::Provider),
            Self::RetrievalTimeout => ("retrieval_timeout", ExitCode::Provider),
        }
    }

    /// Machine-readable error code
    pub fn code(&self) -> &'static str {
        self.code_and_exit().0
    }

    /// Exit code for the CLI
    pub fn exit_code(&self) -> ExitCode {
        self.code_and_exit().1
    }

    /// Human-readable message.
    ///
    /// Prefer using the `Display` impl directly (e.g. `format!("{err}")`) over
    /// calling this method, to avoid the intermediate `String` allocation when
    /// the caller only needs to display the error.
    pub fn message(&self) -> String {
        self.to_string()
    }

    /// Convert to JSON error response
    pub fn to_json_response(&self) -> ErrorResponse {
        let details = match self {
            Self::RateLimitExceeded {
                retry_after_seconds,
            } => Some(serde_json::json!({
                "retry_after_seconds": retry_after_seconds,
            })),
            Self::OperationInProgress {
                retry_after_seconds,
                lock_name,
                ..
            } => Some(serde_json::json!({
                "retry_after_seconds": retry_after_seconds,
                "lock_name": lock_name,
            })),
            _ => None,
        };

        ErrorResponse {
            error: ErrorBody {
                code: self.code().to_string(),
                message: self.message(),
                details,
            },
        }
    }
}

/// JSON error response format
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorBody,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_in_progress_json_contains_retry_and_lock() {
        let err = CiteError::OperationInProgress {
            message: "busy".to_string(),
            retry_after_seconds: 5,
            lock_name: Some("ingest_pipeline".to_string()),
        };

        let response = err.to_json_response();
        let details = response.error.details.expect("details are required");

        assert_eq!(details["retry_after_seconds"], 5);
        assert_eq!(details["lock_name"], "ingest_pipeline");
    }

    #[test]
    fn test_cite_error_partial_eq() {
        let a = CiteError::ConfigError {
            message: "no key".to_string(),
        };
        let b = CiteError::ConfigError {
            message: "no key".to_string(),
        };
        assert_eq!(a, b);

        let c = CiteError::ConfigError {
            message: "different".to_string(),
        };
        assert_ne!(a, c);
    }

    /// Helper: assert that an error variant satisfies the public API contract.
    fn assert_error_contract(err: &CiteError) {
        // code() returns non-empty
        let code = err.code();
        assert!(!code.is_empty(), "code() must be non-empty for {err:?}");

        // message() returns non-empty
        let msg = err.message();
        assert!(!msg.is_empty(), "message() must be non-empty for {err:?}");

        // to_json_response() produces valid JSON with required fields
        let resp = err.to_json_response();
        let json = serde_json::to_value(&resp).expect("ErrorResponse must serialize");
        assert!(json.get("error").is_some(), "JSON must have 'error' key");
        let body = &json["error"];
        assert_eq!(body["code"], code, "JSON code must match code()");
        assert!(
            body["message"].as_str().is_some(),
            "JSON message must be a string"
        );
    }

    #[test]
    fn test_cite_error_matrix_all_variants() {
        let variants: Vec<CiteError> = vec![
            CiteError::UnsupportedFileType {
                file_type: "docx".to_string(),
            },
            CiteError::FileTooLarge {
                size_bytes: 999_999,
                max_bytes: 100_000,
            },
            CiteError::FileNotFound {
                path: "/tmp/missing.txt".into(),
            },
            CiteError::DocumentNotFound {
                document_id: "doc-x".to_string(),
            },
            CiteError::DocumentNotReady {
                document_id: "doc-y".to_string(),
            },
            CiteError::TraceNotFound {
                trace_id: "tr-1".to_string(),
            },
            CiteError::CitationNotFound {
                citation_id: "cite-1".to_string(),
            },
            CiteError::ChunkNotFound {
                chunk_id: "chk-1".to_string(),
            },
            CiteError::ConfigError {
                message: "bad config".to_string(),
            },
            CiteError::StorageError {
                message: "disk full".to_string(),
            },
            CiteError::InternalError {
                message: "oops".to_string(),
            },
            CiteError::QueryTooLong {
                length: 5000,
                max: 2000,
            },
            CiteError::InvalidParameter {
                message: "bad k".to_string(),
            },
            CiteError::PathRejected {
                message: "traversal".to_string(),
            },
            CiteError::RuntimeModeForbidden {
                message: "read-only".to_string(),
            },
            CiteError::RateLimitExceeded {
                retry_after_seconds: 60,
            },
            CiteError::OperationInProgress {
                message: "busy".to_string(),
                retry_after_seconds: 5,
                lock_name: Some("ingest".to_string()),
            },
            CiteError::EmbeddingProviderError {
                message: "api down".to_string(),
            },
            CiteError::RetrievalTimeout,
        ];

        assert_eq!(variants.len(), 19, "must cover all 19 CiteError variants");

        for err in &variants {
            assert_error_contract(err);
        }
    }

    #[test]
    fn test_cite_error_exit_codes() {
        use ExitCode::*;

        let cases: Vec<(CiteError, ExitCode)> = vec![
            (
                CiteError::UnsupportedFileType {
                    file_type: "x".into(),
                },
                Validation,
            ),
            (
                CiteError::FileTooLarge {
                    size_bytes: 1,
                    max_bytes: 0,
                },
                Validation,
            ),
            (CiteError::FileNotFound { path: "/a".into() }, NotFound),
            (
                CiteError::DocumentNotFound {
                    document_id: "d".into(),
                },
                NotFound,
            ),
            (
                CiteError::DocumentNotReady {
                    document_id: "d".into(),
                },
                NotFound,
            ),
            (
                CiteError::TraceNotFound {
                    trace_id: "t".into(),
                },
                NotFound,
            ),
            (
                CiteError::CitationNotFound {
                    citation_id: "c".into(),
                },
                NotFound,
            ),
            (
                CiteError::ChunkNotFound {
                    chunk_id: "c".into(),
                },
                NotFound,
            ),
            (
                CiteError::ConfigError {
                    message: "e".into(),
                },
                Validation,
            ),
            (
                CiteError::StorageError {
                    message: "e".into(),
                },
                Internal,
            ),
            (
                CiteError::InternalError {
                    message: "e".into(),
                },
                Internal,
            ),
            (CiteError::QueryTooLong { length: 1, max: 0 }, Validation),
            (
                CiteError::InvalidParameter {
                    message: "e".into(),
                },
                Validation,
            ),
            (
                CiteError::PathRejected {
                    message: "e".into(),
                },
                Validation,
            ),
            (
                CiteError::RuntimeModeForbidden {
                    message: "e".into(),
                },
                RuntimeForbidden,
            ),
            (
                CiteError::RateLimitExceeded {
                    retry_after_seconds: 1,
                },
                RateLimitExceeded,
            ),
            (
                CiteError::OperationInProgress {
                    message: "e".into(),
                    retry_after_seconds: 1,
                    lock_name: None,
                },
                OperationInProgress,
            ),
            (
                CiteError::EmbeddingProviderError {
                    message: "e".into(),
                },
                Provider,
            ),
            (CiteError::RetrievalTimeout, Provider),
        ];

        for (err, expected_code) in &cases {
            assert_eq!(
                err.exit_code(),
                *expected_code,
                "exit_code mismatch for {:?}",
                err
            );
        }
    }

    #[test]
    fn test_cite_error_codes_unique() {
        let variants: Vec<CiteError> = vec![
            CiteError::UnsupportedFileType {
                file_type: "x".into(),
            },
            CiteError::FileTooLarge {
                size_bytes: 1,
                max_bytes: 0,
            },
            CiteError::FileNotFound { path: "/a".into() },
            CiteError::DocumentNotFound {
                document_id: "d".into(),
            },
            CiteError::DocumentNotReady {
                document_id: "d".into(),
            },
            CiteError::TraceNotFound {
                trace_id: "t".into(),
            },
            CiteError::CitationNotFound {
                citation_id: "c".into(),
            },
            CiteError::ChunkNotFound {
                chunk_id: "c".into(),
            },
            CiteError::ConfigError {
                message: "e".into(),
            },
            CiteError::StorageError {
                message: "e".into(),
            },
            CiteError::InternalError {
                message: "e".into(),
            },
            CiteError::QueryTooLong { length: 1, max: 0 },
            CiteError::InvalidParameter {
                message: "e".into(),
            },
            CiteError::PathRejected {
                message: "e".into(),
            },
            CiteError::RuntimeModeForbidden {
                message: "e".into(),
            },
            CiteError::RateLimitExceeded {
                retry_after_seconds: 1,
            },
            CiteError::OperationInProgress {
                message: "e".into(),
                retry_after_seconds: 1,
                lock_name: None,
            },
            CiteError::EmbeddingProviderError {
                message: "e".into(),
            },
            CiteError::RetrievalTimeout,
        ];

        let codes: Vec<&str> = variants.iter().map(|e| e.code()).collect();
        let unique: std::collections::HashSet<&str> = codes.iter().copied().collect();
        assert_eq!(codes.len(), unique.len(), "error codes must be unique");
    }
}
