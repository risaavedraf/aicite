use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Document status in the ingestion pipeline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DocumentStatus {
    Pending,
    Processing,
    Ready,
    Failed,
}

impl std::fmt::Display for DocumentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Processing => write!(f, "processing"),
            Self::Ready => write!(f, "ready"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

/// Supported file types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FileType {
    Pdf,
    Txt,
    Md,
}

impl std::fmt::Display for FileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pdf => write!(f, "pdf"),
            Self::Txt => write!(f, "txt"),
            Self::Md => write!(f, "md"),
        }
    }
}

impl FileType {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "pdf" => Some(Self::Pdf),
            "txt" => Some(Self::Txt),
            "md" | "markdown" => Some(Self::Md),
            _ => None,
        }
    }
}

/// Error information stored with failed documents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub code: String,
    pub message: String,
}

/// Document metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub document_id: String,
    pub display_name: String,
    pub file_path: PathBuf,
    pub file_type: FileType,
    pub file_size_bytes: u64,
    pub status: DocumentStatus,
    pub chunk_count: u32,
    pub retry_count: u32,
    pub max_retry_count: u32,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub error: Option<ErrorInfo>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Text chunk from a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub chunk_id: String,
    pub document_id: String,
    pub section_id: Option<String>,
    pub chunk_index: u32,
    pub text: String,
    pub page: Option<u32>,
    pub offset_start: Option<u32>,
    pub offset_end: Option<u32>,
    pub created_at: DateTime<Utc>,
}

/// Citation reference for retrieval results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub citation_id: String,
    pub document_id: String,
    pub display_name: String,
    pub chunk_id: String,
    pub page: Option<u32>,
    pub offset: Option<OffsetRange>,
    pub text: String,
    pub score: Option<f64>,
}

/// Offset range in source document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffsetRange {
    pub start: u32,
    pub end: u32,
}
