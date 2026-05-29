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
    pub confidence_label: Option<String>,
    /// Topic name from hierarchy (Phase 11)
    pub topic_name: Option<String>,
    /// Concept name from hierarchy (Phase 11)
    pub concept_name: Option<String>,
    /// Breadcrumb path: "display_name > topic > concept" (Phase 11)
    pub breadcrumb: Option<String>,
}

/// Offset range in source document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffsetRange {
    pub start: u32,
    pub end: u32,
}

/// Result kind for context pack assembly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResultKind {
    Context,
    NoResults,
    InsufficientContext,
}

impl std::fmt::Display for ResultKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Context => write!(f, "context"),
            Self::NoResults => write!(f, "no_results"),
            Self::InsufficientContext => write!(f, "insufficient_context"),
        }
    }
}

/// Context pack metadata (contract fields).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMetadata {
    pub schema_version: String,
    pub created_at: DateTime<Utc>,
    pub retrieved_chunks: u32,
    pub evidence_floor: f64,
    pub confidence_threshold: f64,
    pub ranking_method: String,
    pub top_score: Option<f32>,
    pub corpus_index_state: String,
    pub ready_document_count: u32,
    pub excluded_non_ready_document_count: u32,
    pub excluded_non_ready_document_ids: Vec<String>,
    pub latency_ms: u64,
    pub disclaimer: String,
    pub insufficient_context_reason: Option<String>,
    pub caution: Option<String>,
}

/// Agent-facing context pack response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextResponse {
    pub context_pack_id: String,
    pub result_kind: ResultKind,
    pub query_id: String,
    pub trace_id: String,
    pub instructions: String,
    pub citations: Vec<Citation>,
    pub metadata: ContextMetadata,
}

/// Read selector modes (mutually exclusive).
#[derive(Debug, Clone)]
pub enum ReadSelector {
    Citation {
        trace_id: String,
        citation_id: String,
    },
    Chunk {
        document_id: String,
        chunk_id: String,
    },
}

/// Response from read command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResponse {
    pub citation_id: Option<String>,
    pub document_id: String,
    pub display_name: Option<String>,
    pub chunk_id: String,
    pub page: Option<u32>,
    pub offset: Option<OffsetRange>,
    pub text: String,
    pub trace_id: Option<String>,
    pub score: Option<f64>,
    pub confidence_label: Option<String>,
}

/// Response from trace command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceResponse {
    pub trace_id: String,
    pub query_id: Option<String>,
    pub context_pack_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub schema_version: String,
    pub embedding_model_registry_id: String,
    pub provider: String,
    pub document_ids: Vec<String>,
    pub citation_ids: Vec<String>,
    pub retrieval_top_k: Option<u32>,
    pub evidence_floor: Option<f64>,
    pub confidence_threshold: Option<f64>,
    pub ranking_method: Option<String>,
    pub source_metadata_state: String,
    pub responsible_owner: Option<String>,
    pub user_visible_disclaimer_shown: bool,
}

/// Input payload to persist trace headers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceHeaderInput {
    pub trace_id: String,
    pub query_id: Option<String>,
    pub context_pack_id: Option<String>,
    pub request_type: String,
    pub document_ids: Option<String>,
    pub citation_ids: Option<String>,
    pub top_k: Option<u32>,
    pub evidence_floor: Option<f64>,
    pub confidence_threshold: Option<f64>,
    pub ranking_method: Option<String>,
    pub latency_ms: Option<u64>,
}

/// Stored trace header row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceHeaderRecord {
    pub trace_id: String,
    pub query_id: Option<String>,
    pub context_pack_id: Option<String>,
    pub request_type: String,
    pub document_ids: Option<String>,
    pub citation_ids: Option<String>,
    pub top_k: Option<u32>,
    pub evidence_floor: Option<f64>,
    pub confidence_threshold: Option<f64>,
    pub ranking_method: Option<String>,
    pub latency_ms: Option<u64>,
    pub created_at: DateTime<Utc>,
}

/// Citation row stored for deterministic scoped lookup.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceCitationRecord {
    pub trace_id: String,
    pub citation_id: String,
    pub document_id: String,
    pub display_name: String,
    pub chunk_id: String,
    pub page: Option<u32>,
    pub offset_start: Option<u32>,
    pub offset_end: Option<u32>,
    pub text: String,
    pub score: Option<f64>,
    pub confidence_label: Option<String>,
}

/// Minimal context metadata scaffold for Slice 1.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextMetadataScaffold {
    pub excluded_non_ready_document_count: u32,
    pub excluded_non_ready_document_ids: Vec<String>,
}

/// Trace output envelope scaffold.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceEnvelope {
    pub header: TraceHeaderRecord,
    pub citations: Vec<TraceCitationRecord>,
    pub context_metadata: ContextMetadataScaffold,
}

/// Result of running a single golden fixture evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureResult {
    pub fixture_id: String,
    pub category: String,
    pub passed: bool,
    pub actual_result_kind: ResultKind,
    pub actual_citation_count: usize,
    pub failure_reason: Option<String>,
}

/// Report from running the full golden dataset evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalReport {
    pub total: u32,
    pub passed: u32,
    pub failed: u32,
    pub hit_rate: f64,
    pub threshold: f64,
    pub overall_pass: bool,
    pub results: Vec<FixtureResult>,
}
