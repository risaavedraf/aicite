use crate::exit::ExitCode;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// All errors in the AI Harness system
#[derive(Debug, thiserror::Error)]
pub enum HarnessError {
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

impl HarnessError {
    /// Machine-readable error code
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnsupportedFileType { .. } => "unsupported_file_type",
            Self::FileTooLarge { .. } => "file_too_large",
            Self::FileNotFound { .. } => "file_not_found",
            Self::DocumentNotFound { .. } => "document_not_found",
            Self::DocumentNotReady { .. } => "document_not_ready",
            Self::TraceNotFound { .. } => "trace_not_found",
            Self::CitationNotFound { .. } => "citation_not_found",
            Self::ChunkNotFound { .. } => "chunk_not_found",
            Self::ConfigError { .. } => "config_error",
            Self::StorageError { .. } => "storage_error",
            Self::InternalError { .. } => "internal_error",
            Self::QueryTooLong { .. } => "query_too_long",
            Self::InvalidParameter { .. } => "invalid_parameter",
            Self::PathRejected { .. } => "path_rejected",
            Self::RuntimeModeForbidden { .. } => "runtime_mode_forbidden",
            Self::RateLimitExceeded { .. } => "rate_limit_exceeded",
            Self::OperationInProgress { .. } => "operation_in_progress",
            Self::EmbeddingProviderError { .. } => "embedding_provider_error",
            Self::RetrievalTimeout => "retrieval_timeout",
        }
    }

    /// Exit code for the CLI
    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::UnsupportedFileType { .. } => ExitCode::Validation,
            Self::FileTooLarge { .. } => ExitCode::Validation,
            Self::FileNotFound { .. } => ExitCode::NotFound,
            Self::DocumentNotFound { .. } => ExitCode::NotFound,
            Self::DocumentNotReady { .. } => ExitCode::NotFound,
            Self::TraceNotFound { .. } => ExitCode::NotFound,
            Self::CitationNotFound { .. } => ExitCode::NotFound,
            Self::ChunkNotFound { .. } => ExitCode::NotFound,
            Self::ConfigError { .. } => ExitCode::Validation,
            Self::StorageError { .. } => ExitCode::Internal,
            Self::InternalError { .. } => ExitCode::Internal,
            Self::QueryTooLong { .. } => ExitCode::Validation,
            Self::InvalidParameter { .. } => ExitCode::Validation,
            Self::PathRejected { .. } => ExitCode::Validation,
            Self::RuntimeModeForbidden { .. } => ExitCode::RuntimeForbidden,
            Self::RateLimitExceeded { .. } => ExitCode::RateLimitExceeded,
            Self::OperationInProgress { .. } => ExitCode::OperationInProgress,
            Self::EmbeddingProviderError { .. } => ExitCode::Provider,
            Self::RetrievalTimeout => ExitCode::Provider,
        }
    }

    /// Human-readable message
    pub fn message(&self) -> String {
        self.to_string()
    }

    /// Convert to JSON error response
    pub fn to_json_response(&self) -> ErrorResponse {
        ErrorResponse {
            error: ErrorBody {
                code: self.code().to_string(),
                message: self.message(),
                details: None,
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
