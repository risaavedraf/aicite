use crate::exit::ExitCode;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// All errors in the AI Cite system
#[derive(Debug, thiserror::Error)]
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
}
