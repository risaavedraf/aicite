//! Common types shared across the AI Cite crate ecosystem.
//!
//! This module defines the core domain types used by retrieval, storage,
//! graph, and ingest crates. Types are designed for serialization round-trips
//! (serde) and SQLite persistence.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::fmt;
use std::ops::Deref;
use std::path::PathBuf;
use std::str::FromStr;

// ---------------------------------------------------------------------------
// Newtype wrappers for strongly-typed identifiers
// ---------------------------------------------------------------------------

macro_rules! string_id_newtype {
    (
        $(#[$meta:meta])*
        pub struct $name:ident;
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }

        impl FromStr for $name {
            type Err = Infallible;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Ok(Self::from(value))
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl Deref for $name {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
}

string_id_newtype! {
    /// Strongly-typed document identifier.
    ///
    /// Wraps a plain `String` to prevent accidental mixing with other ID types
    /// at compile time. It is transparent at serialization and persistence
    /// boundaries, while providing typed Rust APIs internally.
    ///
    /// # Examples
    ///
    /// ```
    /// use common::types::DocumentId;
    ///
    /// let id = DocumentId::from("doc-42");
    /// assert_eq!(id.as_ref(), "doc-42");
    /// assert_eq!(format!("{id}"), "doc-42");
    /// ```
    pub struct DocumentId;
}

string_id_newtype! {
    /// Strongly-typed chunk identifier.
    ///
    /// Wraps a plain `String` to distinguish chunk IDs from document or trace IDs
    /// at compile time. It preserves the external string representation.
    ///
    /// # Examples
    ///
    /// ```
    /// use common::types::ChunkId;
    ///
    /// let id = ChunkId::from("chunk-7");
    /// assert_eq!(id.as_ref(), "chunk-7");
    /// ```
    pub struct ChunkId;
}

string_id_newtype! {
    /// Strongly-typed topic identifier.
    ///
    /// Wraps a plain `String` to distinguish graph topic IDs from document,
    /// chunk, concept, or trace IDs while preserving string-shaped external data.
    ///
    /// # Examples
    ///
    /// ```
    /// use common::types::TopicId;
    ///
    /// let id = TopicId::from("topic-doc-1");
    /// assert_eq!(id.as_ref(), "topic-doc-1");
    /// ```
    pub struct TopicId;
}

string_id_newtype! {
    /// Strongly-typed concept identifier.
    ///
    /// Wraps a plain `String` to distinguish graph concept IDs from topic and
    /// other domain identifiers while preserving string-shaped external data.
    ///
    /// # Examples
    ///
    /// ```
    /// use common::types::ConceptId;
    ///
    /// let id = ConceptId::from("concept-doc-1");
    /// assert_eq!(id.as_ref(), "concept-doc-1");
    /// ```
    pub struct ConceptId;
}

string_id_newtype! {
    /// Strongly-typed trace identifier.
    ///
    /// Wraps a plain `String` to distinguish trace IDs from other identifiers
    /// at compile time. Traces link retrieval requests back to their citations
    /// and metadata while preserving string-shaped external data.
    ///
    /// # Examples
    ///
    /// ```
    /// use common::types::TraceId;
    ///
    /// let id = TraceId::from("trace-abc");
    /// assert_eq!(id.as_ref(), "trace-abc");
    /// ```
    pub struct TraceId;
}

// ---------------------------------------------------------------------------
// Domain enums
// ---------------------------------------------------------------------------

/// Document status in the ingestion pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DocumentStatus {
    Pending,
    Processing,
    Ready,
    Failed,
}

impl fmt::Display for DocumentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Processing => write!(f, "processing"),
            Self::Ready => write!(f, "ready"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

/// Supported source file types for document ingestion.
///
/// Determines which parser is used during the ingest pipeline.
/// Serialized as lowercase strings (`"pdf"`, `"txt"`, `"md"`).
///
/// # Examples
///
/// ```
/// use common::types::FileType;
///
/// assert_eq!(FileType::from_extension("pdf"), Some(FileType::Pdf));
/// assert_eq!(FileType::from_extension("markdown"), Some(FileType::Md));
/// assert_eq!(FileType::from_extension("docx"), None);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FileType {
    /// Adobe PDF documents.
    Pdf,
    /// Plain text files.
    Txt,
    /// Markdown documents.
    Md,
}

impl fmt::Display for FileType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pdf => write!(f, "pdf"),
            Self::Txt => write!(f, "txt"),
            Self::Md => write!(f, "md"),
        }
    }
}

impl FileType {
    /// Parse a file type from a lowercase file extension string.
    ///
    /// Recognises `"pdf"`, `"txt"`, `"md"`, and `"markdown"`.
    /// Returns `None` for unrecognised extensions.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "pdf" => Some(Self::Pdf),
            "txt" => Some(Self::Txt),
            "md" | "markdown" => Some(Self::Md),
            _ => None,
        }
    }
}

/// Error information stored with failed documents.
///
/// Carries a machine-readable `code` and a human-readable `message`.
/// Persisted alongside the document so retry logic can inspect the failure reason.
///
/// # Examples
///
/// ```
/// use common::types::ErrorInfo;
///
/// let err = ErrorInfo {
///     code: "PARSE_FAILED".to_string(),
///     message: "Corrupt PDF".to_string(),
/// };
/// assert_eq!(err.code, "PARSE_FAILED");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    /// Machine-readable error code (e.g. `"PARSE_FAILED"`).
    pub code: String,
    /// Human-readable error description.
    pub message: String,
}

/// Core document metadata persisted in SQLite.
///
/// Represents a single ingested file and its current processing state.
/// Fields are grouped logically:
/// - **Identity**: `document_id`, `display_name`, `file_path`, `file_type`, `file_size_bytes`
/// - **Pipeline state**: `status`, `chunk_count`, retry tracking, `error`
/// - **Timestamps**: `created_at`, `updated_at`
/// - **Lifecycle**: `source_hash`, `ingested_at`, `file_modified_at`
///
/// > **Note on sub-structs**: The fields are intentionally kept flat because
/// > splitting into nested structs (e.g. `DocumentIdentity`) would require
/// > changes across every construction site in the codebase (storage, ingest,
/// > CLI). The current layout is a pragmatic trade-off for adoption simplicity.
///
/// # Examples
///
/// ```
/// use chrono::Utc;
/// use std::path::PathBuf;
/// use common::types::{Document, DocumentStatus, FileType};
///
/// let doc = Document {
///     document_id: "doc-1".into(),
///     display_name: "readme.txt".to_string(),
///     file_path: PathBuf::from("/docs/readme.txt"),
///     file_type: FileType::Txt,
///     file_size_bytes: 2048,
///     status: DocumentStatus::Pending,
///     chunk_count: 0,
///     retry_count: 0,
///     max_retry_count: 3,
///     next_retry_at: None,
///     error: None,
///     created_at: Utc::now(),
///     updated_at: Utc::now(),
///     source_hash: None,
///     ingested_at: None,
///     file_modified_at: None,
/// };
/// assert_eq!(doc.status, DocumentStatus::Pending);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Unique document identifier.
    pub document_id: DocumentId,
    /// Human-readable display name (typically the filename).
    pub display_name: String,
    /// Absolute path to the source file on disk.
    pub file_path: PathBuf,
    /// Detected file type governing parser selection.
    pub file_type: FileType,
    /// Size of the source file in bytes.
    pub file_size_bytes: u64,
    /// Current position in the ingestion pipeline.
    pub status: DocumentStatus,
    /// Number of text chunks produced during ingestion.
    pub chunk_count: u32,
    /// How many times processing has been retried after failure.
    pub retry_count: u32,
    /// Maximum retries allowed before permanent failure.
    pub max_retry_count: u32,
    /// Scheduled time for the next retry attempt, if any.
    pub next_retry_at: Option<DateTime<Utc>>,
    /// Error details when `status` is `Failed`.
    pub error: Option<ErrorInfo>,
    /// Timestamp when the document was first ingested.
    pub created_at: DateTime<Utc>,
    /// Timestamp of the most recent status change.
    pub updated_at: DateTime<Utc>,
    /// Hash of source file contents observed at ingest time.
    pub source_hash: Option<String>,
    /// Timestamp when Cite last processed the source content.
    pub ingested_at: Option<DateTime<Utc>>,
    /// Source file modification time observed during ingest, when available.
    pub file_modified_at: Option<DateTime<Utc>>,
}

/// Text chunk extracted from a document during ingestion.
///
/// Each chunk represents a contiguous segment of the source document,
/// optionally anchored to a page number and character offset range.
/// Chunks are the unit of embedding and retrieval.
///
/// # Examples
///
/// ```
/// use chrono::Utc;
/// use common::types::Chunk;
///
/// let chunk = Chunk {
///     chunk_id: "chunk-1".into(),
///     document_id: "doc-1".into(),
///     section_id: None,
///     chunk_index: 0,
///     text: "Hello, world!".to_string(),
///     page: Some(1),
///     offset_start: Some(0),
///     offset_end: Some(13),
///     created_at: Utc::now(),
/// };
/// assert_eq!(chunk.chunk_index, 0);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// Unique chunk identifier.
    pub chunk_id: ChunkId,
    /// Identifier of the parent document.
    pub document_id: DocumentId,
    /// Optional section identifier from the hierarchy builder.
    pub section_id: Option<String>,
    /// Zero-based ordering index within the document.
    pub chunk_index: u32,
    /// The extracted text content.
    pub text: String,
    /// Page number in the source document (1-indexed), if applicable.
    pub page: Option<u32>,
    /// Character offset where this chunk starts in the source.
    pub offset_start: Option<u32>,
    /// Character offset where this chunk ends in the source.
    pub offset_end: Option<u32>,
    /// Timestamp when the chunk was created.
    pub created_at: DateTime<Utc>,
}

/// Citation reference returned in retrieval results.
///
/// Links a ranked chunk back to its source document with display-ready fields
/// for agent consumption. The `breadcrumb` field provides a human-readable
/// navigation path (e.g. `"doc.pdf > API Gateway > Routing"`).
///
/// # Examples
///
/// ```
/// use common::types::Citation;
///
/// let c = Citation {
///     citation_id: "cite-1".to_string(),
///     document_id: "doc-1".into(),
///     display_name: "readme.txt".to_string(),
///     chunk_id: "chunk-1".into(),
///     page: Some(1),
///     offset: None,
///     text: "relevant excerpt".to_string(),
///     score: Some(0.92),
///     confidence_label: Some("high".to_string()),
///     topic_name: None,
///     concept_name: None,
///     breadcrumb: None,
/// };
/// assert!(c.score.unwrap() > 0.9);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    /// Unique citation identifier.
    pub citation_id: String,
    /// Source document identifier.
    pub document_id: DocumentId,
    /// Human-readable document name.
    pub display_name: String,
    /// Chunk that this citation references.
    pub chunk_id: ChunkId,
    /// Page number in the source document.
    pub page: Option<u32>,
    /// Character offset range in the source.
    pub offset: Option<OffsetRange>,
    /// The cited text content.
    pub text: String,
    /// Retrieval relevance score.
    pub score: Option<f64>,
    /// Human-readable confidence tier (e.g. `"high"`, `"medium"`, `"low"`).
    pub confidence_label: Option<String>,
    /// Topic name from the document hierarchy.
    pub topic_name: Option<String>,
    /// Concept name from the document hierarchy.
    pub concept_name: Option<String>,
    /// Breadcrumb path: `"display_name > topic > concept"`.
    pub breadcrumb: Option<String>,
}

/// Character offset range within a source document.
///
/// Used to locate the cited text fragment in the original file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffsetRange {
    /// Start offset (inclusive), zero-indexed.
    pub start: u32,
    /// End offset (exclusive), zero-indexed.
    pub end: u32,
}

/// Classification of a retrieval result for context pack assembly.
///
/// Determines how the agent should interpret the response:
/// - [`Context`](ResultKind::Context) — sufficient evidence was found.
/// - [`NoResults`](ResultKind::NoResults) — no matching chunks.
/// - [`InsufficientContext`](ResultKind::InsufficientContext) — chunks found
///   but below the confidence threshold.
///
/// # Examples
///
/// ```
/// use common::types::ResultKind;
///
/// let kind = ResultKind::Context;
/// assert_eq!(kind.to_string(), "context");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResultKind {
    Context,
    NoResults,
    InsufficientContext,
}

impl fmt::Display for ResultKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Context => write!(f, "context"),
            Self::NoResults => write!(f, "no_results"),
            Self::InsufficientContext => write!(f, "insufficient_context"),
        }
    }
}

/// Metadata envelope for a context pack response.
///
/// Contains provenance, ranking parameters, and diagnostic fields that
/// allow the agent to reason about retrieval quality. Contract fields are
/// versioned via `schema_version`.
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

/// Top-level context pack response returned to the calling agent.
///
/// Wraps the ranked citations, retrieval metadata, and the assembled
/// instructions block that the agent uses as context for answering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextResponse {
    pub context_pack_id: String,
    pub result_kind: ResultKind,
    pub query_id: String,
    pub trace_id: TraceId,
    pub instructions: String,
    pub citations: Vec<Citation>,
    pub metadata: ContextMetadata,
}

/// Selector for the `read` command.
///
/// Determines whether a read targets a citation from a previous trace or
/// a specific chunk by its identifiers. The two modes are mutually exclusive.
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

/// Response payload from the `read` command.
///
/// Returns the full text of a single chunk along with its provenance
/// metadata and optional retrieval score.
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

/// Response payload from the `trace` command.
///
/// Provides full provenance for a retrieval request, including the
/// embedding model, provider, ranking parameters, and associated
/// document/citation identifiers.
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

/// Input payload for persisting trace headers to the trace store.
///
/// Captures the request-level parameters of a retrieval invocation
/// so they can be replayed or audited later.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceHeaderInput {
    pub trace_id: TraceId,
    pub query_id: Option<String>,
    pub context_pack_id: Option<String>,
    pub request_type: String,
    pub document_ids: Option<String>,
    pub citation_ids: Option<String>,
    pub top_k: Option<u32>,
    pub evidence_floor: Option<f64>,
    pub confidence_threshold: Option<f64>,
    pub ranking_method: Option<String>,
    pub embedding_model_registry_id: Option<String>,
    pub provider: Option<String>,
    pub latency_ms: Option<u64>,
}

/// Persisted trace header row read back from the trace store.
///
/// Extends [`TraceHeaderInput`] with a `created_at` timestamp.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceHeaderRecord {
    pub trace_id: TraceId,
    pub query_id: Option<String>,
    pub context_pack_id: Option<String>,
    pub request_type: String,
    pub document_ids: Option<String>,
    pub citation_ids: Option<String>,
    pub top_k: Option<u32>,
    pub evidence_floor: Option<f64>,
    pub confidence_threshold: Option<f64>,
    pub ranking_method: Option<String>,
    pub embedding_model_registry_id: Option<String>,
    pub provider: Option<String>,
    pub latency_ms: Option<u64>,
    pub created_at: DateTime<Utc>,
}

/// Citation row stored in the trace store for deterministic scoped lookup.
///
/// Stores the denormalised citation data so a `trace` command can reconstruct
/// the original context pack without re-querying the embedding index.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceCitationRecord {
    pub trace_id: TraceId,
    pub citation_id: String,
    pub document_id: DocumentId,
    pub display_name: String,
    pub chunk_id: ChunkId,
    pub page: Option<u32>,
    pub offset_start: Option<u32>,
    pub offset_end: Option<u32>,
    pub text: String,
    pub score: Option<f64>,
    pub confidence_label: Option<String>,
}

/// Minimal context metadata scaffold used during early pipeline slices.
///
/// Tracks which documents were excluded from retrieval because they were
/// not in `Ready` status at query time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextMetadataScaffold {
    pub excluded_non_ready_document_count: u32,
    pub excluded_non_ready_document_ids: Vec<String>,
}

/// Complete trace output envelope combining header, citations, and
/// context metadata for a single retrieval invocation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceEnvelope {
    pub header: TraceHeaderRecord,
    pub citations: Vec<TraceCitationRecord>,
    pub context_metadata: ContextMetadataScaffold,
}

/// Result of evaluating a single golden fixture against the retrieval pipeline.
///
/// Used by the eval harness to track pass/fail status and compare actual
/// against expected retrieval outcomes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureResult {
    pub fixture_id: String,
    pub category: String,
    pub passed: bool,
    pub actual_result_kind: ResultKind,
    pub actual_citation_count: usize,
    pub failure_reason: Option<String>,
}

/// Aggregate report from running the full golden dataset evaluation.
///
/// Summarises pass/fail counts, hit rate, and whether the overall
/// evaluation meets the configured threshold.
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

#[cfg(test)]
mod tests {
    use super::{ChunkId, ConceptId, DocumentId, FileType, TopicId, TraceId};
    use std::str::FromStr;

    fn assert_string_id_contract<T>(sample: &str, expected_debug_name: &str)
    where
        T: From<String>
            + From<&'static str>
            + FromStr<Err = std::convert::Infallible>
            + AsRef<str>
            + Clone
            + std::fmt::Debug
            + std::fmt::Display
            + std::ops::Deref<Target = str>
            + PartialEq
            + serde::Serialize
            + for<'de> serde::Deserialize<'de>,
    {
        let from_string = T::from(sample.to_owned());
        let from_str_literal = T::from("literal-id");
        let parsed = T::from_str(sample).expect("string-transparent IDs parse infallibly");

        assert_eq!(from_string, parsed);
        assert_eq!(from_string.clone(), parsed);
        assert_eq!(from_string.as_ref(), sample);
        assert_eq!(&*from_string, sample);
        assert_eq!(from_string.to_string(), sample);
        assert_eq!(from_str_literal.as_ref(), "literal-id");
        assert!(format!("{from_string:?}").contains(expected_debug_name));

        let json = serde_json::to_string(&from_string).expect("ID serializes as JSON string");
        assert_eq!(json, format!("\"{sample}\""));

        let deserialized: T =
            serde_json::from_str(&json).expect("ID deserializes from JSON string");
        assert_eq!(deserialized, from_string);
    }

    #[test]
    fn document_id_has_string_transparent_foundation_traits() {
        assert_string_id_contract::<DocumentId>("doc-42", "DocumentId");
    }

    #[test]
    fn chunk_id_has_string_transparent_foundation_traits() {
        assert_string_id_contract::<ChunkId>("chunk-7", "ChunkId");
    }

    #[test]
    fn topic_id_has_string_transparent_foundation_traits() {
        assert_string_id_contract::<TopicId>("topic-doc-1", "TopicId");
    }

    #[test]
    fn concept_id_has_string_transparent_foundation_traits() {
        assert_string_id_contract::<ConceptId>("concept-doc-1", "ConceptId");
    }

    #[test]
    fn trace_id_has_string_transparent_foundation_traits() {
        assert_string_id_contract::<TraceId>("trace-abc", "TraceId");
    }

    // -----------------------------------------------------------------------
    // FileType::from_extension()
    // -----------------------------------------------------------------------

    #[test]
    fn from_extension_pdf() {
        assert_eq!(FileType::from_extension("pdf"), Some(FileType::Pdf));
    }

    #[test]
    fn from_extension_txt() {
        assert_eq!(FileType::from_extension("txt"), Some(FileType::Txt));
    }

    #[test]
    fn from_extension_md() {
        assert_eq!(FileType::from_extension("md"), Some(FileType::Md));
    }

    #[test]
    fn from_extension_markdown() {
        assert_eq!(FileType::from_extension("markdown"), Some(FileType::Md));
    }

    #[test]
    fn from_extension_unknown() {
        assert_eq!(FileType::from_extension("docx"), None);
        assert_eq!(FileType::from_extension("xlsx"), None);
        assert_eq!(FileType::from_extension(""), None);
    }

    #[test]
    fn from_extension_case_insensitive() {
        assert_eq!(FileType::from_extension("PDF"), Some(FileType::Pdf));
        assert_eq!(FileType::from_extension("TXT"), Some(FileType::Txt));
        assert_eq!(FileType::from_extension("MD"), Some(FileType::Md));
        assert_eq!(FileType::from_extension("Markdown"), Some(FileType::Md));
    }
}
