use common::CiteError;
use config::{RateLimitConfig, RetrievalConfig};
use providers::EmbeddingProvider;
use retrieval::{rank_by_similarity, ScoredChunk};
use std::collections::HashMap;
use storage::Database;

const MAX_QUERY_CHARS: usize = 4000;
const MIN_K: u32 = 1;
const MAX_K: u32 = 10;
const SEARCH_PREVIEW_CHARS: usize = 160;
const SEARCH_ROUTE: &str = "search";
const RETRIEVE_ROUTE: &str = "retrieve";

// ---------------------------------------------------------------------------
// Parameter object
// ---------------------------------------------------------------------------

/// Parameter object for retrieval operations.
pub struct RetrievalRequest<'a> {
    pub db: &'a Database,
    pub provider: &'a dyn EmbeddingProvider,
    pub config: &'a RetrievalConfig,
    pub rate_limit: &'a RateLimitConfig,
    pub query: &'a str,
    pub k_override: Option<u32>,
    pub topic_filter: Option<&'a str>,
    pub concept_filter: Option<&'a str>,
}

// ---------------------------------------------------------------------------
// Unified hit type
// ---------------------------------------------------------------------------

/// Unified hit from retrieval operations.
///
/// Contains the full chunk text. Use [`Hit::preview`] for a truncated preview.
#[derive(Debug, Clone)]
pub struct Hit {
    pub chunk_id: String,
    pub document_id: String,
    pub display_name: String,
    pub section_id: Option<String>,
    pub chunk_index: u32,
    pub page: Option<u32>,
    pub offset_start: Option<u32>,
    pub offset_end: Option<u32>,
    pub score: f32,
    pub text: String,
    /// Topic name from hierarchy (Phase 11)
    pub topic_name: Option<String>,
    /// Concept name from hierarchy (Phase 11)
    pub concept_name: Option<String>,
    /// Breadcrumb path: "display_name > topic > concept" (Phase 11)
    pub breadcrumb: Option<String>,
}

/// Backward-compatible alias for [`Hit`].
pub type SearchHit = Hit;
/// Backward-compatible alias for [`Hit`].
pub type RetrieveHit = Hit;

// ---------------------------------------------------------------------------
// Hierarchy helpers
// ---------------------------------------------------------------------------

/// Build a breadcrumb path from display name and hierarchy metadata.
pub(crate) fn build_breadcrumb(
    display_name: &str,
    topic_name: Option<&str>,
    concept_name: Option<&str>,
) -> String {
    match (topic_name, concept_name) {
        (Some(t), Some(c)) => format!("{} > {} > {}", display_name, t, c),
        (Some(t), None) => format!("{} > {}", display_name, t),
        _ => display_name.to_string(),
    }
}

/// Enrich ranked `ScoredChunk` results with hierarchy metadata from a lookup map.
pub(crate) fn enrich_with_hierarchy(
    ranked: Vec<ScoredChunk>,
    meta: &HierarchyMeta,
) -> Vec<ScoredChunk> {
    ranked
        .into_iter()
        .map(|mut item| {
            if let Some((tid, tname, cid, cname)) = meta.get(&item.chunk_id) {
                item.topic_id = tid.clone();
                item.topic_name = tname.clone();
                item.concept_id = cid.clone();
                item.concept_name = cname.clone();
            }
            item
        })
        .collect()
}

/// Type alias for the hierarchy metadata lookup: chunk_id → (topic_id, topic_name, concept_id, concept_name).
type HierarchyMeta = HashMap<
    String,
    (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ),
>;

/// Fetch candidates using hierarchical path if available, flat path otherwise.
/// Returns (flat_candidates, optional_hierarchy_metadata).
#[allow(clippy::too_many_arguments)]
pub(crate) fn fetch_candidates(
    db: &Database,
    config: &RetrievalConfig,
    topic_filter: Option<&str>,
    concept_filter: Option<&str>,
) -> Result<
    (
        Vec<storage::embeddings::ChunkEmbeddingRecord>,
        Option<HierarchyMeta>,
    ),
    CiteError,
> {
    let use_hierarchy = config.use_hierarchy && db.has_hierarchy_data().unwrap_or(false);

    if use_hierarchy {
        let hier = db.list_chunk_embeddings_hierarchical(topic_filter, concept_filter)?;
        let meta: HierarchyMeta = hier
            .iter()
            .map(|h| {
                (
                    h.chunk.chunk_id.clone(),
                    (
                        h.topic_id.clone(),
                        h.topic_name.clone(),
                        h.concept_id.clone(),
                        h.concept_name.clone(),
                    ),
                )
            })
            .collect();
        let flat: Vec<storage::embeddings::ChunkEmbeddingRecord> =
            hier.into_iter().map(|h| h.chunk).collect();
        Ok((flat, Some(meta)))
    } else {
        Ok((db.list_ready_chunk_embeddings()?, None))
    }
}

// ---------------------------------------------------------------------------
// Shared retrieval pipeline
// ---------------------------------------------------------------------------

/// Shared retrieval pipeline: validate query, enforce rate limit, embed,
/// fetch candidates, rank by similarity, and enrich with hierarchy metadata.
pub fn ranked_candidates(
    req: &RetrievalRequest<'_>,
    route: &str,
) -> Result<Vec<ScoredChunk>, CiteError> {
    let k = resolve_k(req.config, req.k_override)?;
    validate_query(req.query)?;
    enforce_rate_limit(req.db, req.provider, req.rate_limit, route)?;
    let query_vector = req.provider.embed(req.query)?;
    let (candidates, hierarchy_meta) =
        fetch_candidates(req.db, req.config, req.topic_filter, req.concept_filter)?;
    let ranked = rank_by_similarity(&query_vector, &candidates, k as usize);
    Ok(if let Some(ref meta) = hierarchy_meta {
        enrich_with_hierarchy(ranked, meta)
    } else {
        ranked
    })
}

// ---------------------------------------------------------------------------
// Hit conversion
// ---------------------------------------------------------------------------

impl Hit {
    /// Create a [`Hit`] from a ranked [`ScoredChunk`], computing the breadcrumb
    /// path when hierarchy metadata is present.
    fn from_scored_chunk(item: ScoredChunk) -> Self {
        let breadcrumb = if item.topic_name.is_some() || item.concept_name.is_some() {
            Some(build_breadcrumb(
                &item.display_name,
                item.topic_name.as_deref(),
                item.concept_name.as_deref(),
            ))
        } else {
            None
        };
        Hit {
            chunk_id: item.chunk_id,
            document_id: item.document_id,
            display_name: item.display_name,
            section_id: item.section_id,
            chunk_index: item.chunk_index,
            page: item.page,
            offset_start: item.offset_start,
            offset_end: item.offset_end,
            score: item.score,
            text: item.text,
            topic_name: item.topic_name,
            concept_name: item.concept_name,
            breadcrumb,
        }
    }

    /// Returns a truncated preview of the chunk text (~160 characters).
    pub fn preview(&self) -> String {
        make_preview(&self.text)
    }
}

// ---------------------------------------------------------------------------
// Public API: search & retrieve
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub fn search(
    db: &Database,
    provider: &dyn EmbeddingProvider,
    config: &RetrievalConfig,
    rate_limit: &RateLimitConfig,
    query: &str,
    k_override: Option<u32>,
    topic_filter: Option<&str>,
    concept_filter: Option<&str>,
) -> Result<Vec<Hit>, CiteError> {
    let req = RetrievalRequest {
        db,
        provider,
        config,
        rate_limit,
        query,
        k_override,
        topic_filter,
        concept_filter,
    };
    let ranked = ranked_candidates(&req, SEARCH_ROUTE)?;
    Ok(ranked.into_iter().map(Hit::from_scored_chunk).collect())
}

#[allow(clippy::too_many_arguments)]
pub fn retrieve(
    db: &Database,
    provider: &dyn EmbeddingProvider,
    config: &RetrievalConfig,
    rate_limit: &RateLimitConfig,
    query: &str,
    k_override: Option<u32>,
    topic_filter: Option<&str>,
    concept_filter: Option<&str>,
) -> Result<Vec<Hit>, CiteError> {
    let req = RetrievalRequest {
        db,
        provider,
        config,
        rate_limit,
        query,
        k_override,
        topic_filter,
        concept_filter,
    };
    let ranked = ranked_candidates(&req, RETRIEVE_ROUTE)?;
    Ok(ranked.into_iter().map(Hit::from_scored_chunk).collect())
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

pub(crate) fn rate_limit_key(provider: &dyn EmbeddingProvider) -> String {
    format!("{}:{}", provider.provider_id(), provider.model_id())
}

fn enforce_rate_limit(
    db: &Database,
    provider: &dyn EmbeddingProvider,
    rate_limit: &RateLimitConfig,
    route: &str,
) -> Result<(), CiteError> {
    let key = rate_limit_key(provider);
    match db.check_and_increment_rate_limit(
        route,
        &key,
        rate_limit.max_requests,
        rate_limit.window_seconds,
    )? {
        storage::rate_limits::RateLimitDecision::Allowed => Ok(()),
        storage::rate_limits::RateLimitDecision::Blocked {
            retry_after_seconds,
        } => Err(CiteError::RateLimitExceeded {
            retry_after_seconds,
        }),
    }
}

pub(crate) fn resolve_k(
    config: &RetrievalConfig,
    k_override: Option<u32>,
) -> Result<u32, CiteError> {
    let k = k_override.unwrap_or(config.top_k);
    if !(MIN_K..=MAX_K).contains(&k) {
        return Err(CiteError::InvalidParameter {
            message: format!("top-k must be between {MIN_K} and {MAX_K}, got {k}"),
        });
    }
    Ok(k)
}

pub(crate) fn validate_query(query: &str) -> Result<(), CiteError> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Err(CiteError::InvalidParameter {
            message: "query must not be empty".to_string(),
        });
    }

    // Reject queries that consist only of punctuation or control characters
    if !trimmed.chars().any(|c| c.is_alphanumeric()) {
        return Err(CiteError::InvalidParameter {
            message: "query must contain at least one alphanumeric character".to_string(),
        });
    }

    let len = trimmed.chars().count();
    if len > MAX_QUERY_CHARS {
        return Err(CiteError::QueryTooLong {
            length: len,
            max: MAX_QUERY_CHARS,
        });
    }

    Ok(())
}

fn make_preview(text: &str) -> String {
    // Single-pass: normalize whitespace and truncate to SEARCH_PREVIEW_CHARS.
    let mut out = String::new();
    let mut char_count = 0usize;
    let mut prev_was_space = true; // treat start as after-space to skip leading ws
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !prev_was_space {
                prev_was_space = true;
            }
        } else {
            if prev_was_space && !out.is_empty() {
                if char_count >= SEARCH_PREVIEW_CHARS {
                    return format!("{out}…");
                }
                out.push(' ');
            }
            if char_count >= SEARCH_PREVIEW_CHARS {
                return format!("{out}…");
            }
            out.push(ch);
            char_count += 1;
            prev_was_space = false;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use common::types::{Chunk, Document, DocumentStatus, FileType};
    use config::RateLimitConfig;

    struct FakeProvider {
        vector: Vec<f32>,
    }

    impl EmbeddingProvider for FakeProvider {
        fn embed(&self, _text: &str) -> Result<Vec<f32>, CiteError> {
            Ok(self.vector.clone())
        }

        fn model_id(&self) -> &str {
            "fake-model"
        }

        fn provider_id(&self) -> &str {
            "fake"
        }
    }

    fn test_db() -> Database {
        Database::open_memory().unwrap()
    }

    fn rl_cfg() -> RateLimitConfig {
        RateLimitConfig {
            max_requests: 20,
            window_seconds: 60,
        }
    }

    fn insert_doc(db: &Database, id: &str, status: DocumentStatus) {
        let now = Utc::now();
        let doc = Document {
            document_id: id.to_string(),
            display_name: format!("{id}.txt"),
            file_path: std::path::Path::new("/tmp/test.txt").to_path_buf(),
            file_type: FileType::Txt,
            file_size_bytes: 100,
            status,
            chunk_count: 0,
            retry_count: 0,
            max_retry_count: 3,
            next_retry_at: None,
            error: None,
            created_at: now,
            updated_at: now,
        };
        db.insert_document(&doc).unwrap();
    }

    fn insert_chunk_with_embedding(
        db: &Database,
        doc_id: &str,
        chunk_id: &str,
        text: &str,
        vec: Vec<f32>,
    ) {
        let chunk = Chunk {
            chunk_id: chunk_id.to_string(),
            document_id: doc_id.to_string(),
            section_id: Some("section-a".to_string()),
            chunk_index: 0,
            text: text.to_string(),
            page: Some(1),
            offset_start: Some(0),
            offset_end: Some(text.len() as u32),
            created_at: Utc::now(),
        };
        db.insert_chunks(doc_id, &[chunk]).unwrap();
        db.insert_embeddings(&[(chunk_id.to_string(), vec, "m", "p")])
            .unwrap();
    }

    #[test]
    fn test_search_rejects_empty_query() {
        let db = test_db();
        let provider = FakeProvider {
            vector: vec![1.0, 0.0],
        };
        let cfg = RetrievalConfig {
            top_k: 5,
            evidence_floor: 0.5,
            confidence_threshold: 0.7,
            use_hierarchy: true,
        };

        let err = search(&db, &provider, &cfg, &rl_cfg(), "   ", None, None, None).unwrap_err();
        assert!(matches!(err, CiteError::InvalidParameter { .. }));
    }

    #[test]
    fn test_search_rejects_punctuation_only_query() {
        let db = test_db();
        let provider = FakeProvider {
            vector: vec![1.0, 0.0],
        };
        let cfg = RetrievalConfig {
            top_k: 5,
            evidence_floor: 0.5,
            confidence_threshold: 0.7,
            use_hierarchy: true,
        };

        let err = search(&db, &provider, &cfg, &rl_cfg(), "???", None, None, None).unwrap_err();
        assert!(matches!(err, CiteError::InvalidParameter { .. }));

        let err = search(&db, &provider, &cfg, &rl_cfg(), "...", None, None, None).unwrap_err();
        assert!(matches!(err, CiteError::InvalidParameter { .. }));
    }

    #[test]
    fn test_search_rejects_invalid_k() {
        let db = test_db();
        let provider = FakeProvider {
            vector: vec![1.0, 0.0],
        };
        let cfg = RetrievalConfig {
            top_k: 0,
            evidence_floor: 0.5,
            confidence_threshold: 0.7,
            use_hierarchy: true,
        };

        let err = search(&db, &provider, &cfg, &rl_cfg(), "hello", None, None, None).unwrap_err();
        assert!(matches!(err, CiteError::InvalidParameter { .. }));
    }

    #[test]
    fn test_search_returns_ready_documents_only() {
        let db = test_db();
        insert_doc(&db, "doc-ready", DocumentStatus::Ready);
        insert_doc(&db, "doc-failed", DocumentStatus::Failed);

        insert_chunk_with_embedding(
            &db,
            "doc-ready",
            "chunk-ready",
            "ready text",
            vec![1.0, 0.0],
        );
        insert_chunk_with_embedding(
            &db,
            "doc-failed",
            "chunk-failed",
            "failed text",
            vec![1.0, 0.0],
        );

        let provider = FakeProvider {
            vector: vec![1.0, 0.0],
        };
        let cfg = RetrievalConfig {
            top_k: 5,
            evidence_floor: 0.5,
            confidence_threshold: 0.7,
            use_hierarchy: true,
        };

        let results = search(&db, &provider, &cfg, &rl_cfg(), "query", None, None, None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "doc-ready");
        assert!(results[0].preview().contains("ready text"));
    }

    #[test]
    fn test_retrieve_returns_full_text() {
        let db = test_db();
        insert_doc(&db, "doc-ready", DocumentStatus::Ready);
        insert_chunk_with_embedding(
            &db,
            "doc-ready",
            "chunk-ready",
            "this is the full chunk text",
            vec![1.0, 0.0],
        );

        let provider = FakeProvider {
            vector: vec![1.0, 0.0],
        };
        let cfg = RetrievalConfig {
            top_k: 5,
            evidence_floor: 0.5,
            confidence_threshold: 0.7,
            use_hierarchy: true,
        };

        let results = retrieve(
            &db,
            &provider,
            &cfg,
            &rl_cfg(),
            "query",
            Some(1),
            None,
            None,
        )
        .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].text, "this is the full chunk text");
        assert_eq!(results[0].chunk_id, "chunk-ready");
    }

    #[test]
    fn test_search_empty_corpus_returns_empty_results() {
        let db = test_db();
        let provider = FakeProvider {
            vector: vec![1.0, 0.0],
        };
        let cfg = RetrievalConfig {
            top_k: 5,
            evidence_floor: 0.5,
            confidence_threshold: 0.7,
            use_hierarchy: true,
        };

        let results = search(&db, &provider, &cfg, &rl_cfg(), "query", None, None, None).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_rejects_too_long_query() {
        let db = test_db();
        let provider = FakeProvider {
            vector: vec![1.0, 0.0],
        };
        let cfg = RetrievalConfig {
            top_k: 5,
            evidence_floor: 0.5,
            confidence_threshold: 0.7,
            use_hierarchy: true,
        };

        let query = "a".repeat(4001);
        let err = search(&db, &provider, &cfg, &rl_cfg(), &query, None, None, None).unwrap_err();
        assert!(matches!(err, CiteError::QueryTooLong { .. }));
    }

    #[test]
    fn test_search_enforces_rate_limit() {
        let db = test_db();
        insert_doc(&db, "doc-ready", DocumentStatus::Ready);
        insert_chunk_with_embedding(
            &db,
            "doc-ready",
            "chunk-ready",
            "ready text",
            vec![1.0, 0.0],
        );

        let provider = FakeProvider {
            vector: vec![1.0, 0.0],
        };
        let cfg = RetrievalConfig {
            top_k: 5,
            evidence_floor: 0.5,
            confidence_threshold: 0.7,
            use_hierarchy: true,
        };
        let rl = RateLimitConfig {
            max_requests: 1,
            window_seconds: 60,
        };

        assert!(search(&db, &provider, &cfg, &rl, "query", None, None, None).is_ok());
        let err = search(&db, &provider, &cfg, &rl, "query", None, None, None).unwrap_err();
        assert!(matches!(
            err,
            CiteError::RateLimitExceeded {
                retry_after_seconds: _
            }
        ));
    }

    #[test]
    fn test_retrieve_enforces_rate_limit() {
        let db = test_db();
        insert_doc(&db, "doc-ready", DocumentStatus::Ready);
        insert_chunk_with_embedding(
            &db,
            "doc-ready",
            "chunk-ready",
            "ready text",
            vec![1.0, 0.0],
        );

        let provider = FakeProvider {
            vector: vec![1.0, 0.0],
        };
        let cfg = RetrievalConfig {
            top_k: 5,
            evidence_floor: 0.5,
            confidence_threshold: 0.7,
            use_hierarchy: true,
        };
        let rl = RateLimitConfig {
            max_requests: 1,
            window_seconds: 60,
        };

        assert!(retrieve(&db, &provider, &cfg, &rl, "query", None, None, None).is_ok());
        let err = retrieve(&db, &provider, &cfg, &rl, "query", None, None, None).unwrap_err();
        assert!(matches!(
            err,
            CiteError::RateLimitExceeded {
                retry_after_seconds: _
            }
        ));
    }

    #[test]
    fn test_rate_limit_key_includes_model_id() {
        let provider = FakeProvider {
            vector: vec![1.0, 0.0],
        };
        // FakeProvider has provider_id "fake" and model_id "fake-model"
        let key = rate_limit_key(&provider);
        assert_eq!(key, "fake:fake-model");
    }

    // -----------------------------------------------------------------------
    // Phase 11 — Hierarchical retrieval tests
    // -----------------------------------------------------------------------

    /// Helper: set up a doc with hierarchy (topic + concept) and chunks with embeddings.
    fn setup_hierarchy(db: &Database) {
        insert_doc(db, "doc-hier", DocumentStatus::Ready);
        insert_chunk_with_embedding(
            db,
            "doc-hier",
            "c-hier-0",
            "JWT tokens with 15-min expiry",
            vec![1.0, 0.0],
        );
        insert_chunk_with_embedding(
            db,
            "doc-hier",
            "c-hier-1",
            "Refresh tokens valid for 7 days",
            vec![0.9, 0.1],
        );
        insert_chunk_with_embedding(
            db,
            "doc-hier",
            "c-hier-2",
            "Unrelated chunk about logging",
            vec![0.0, 1.0],
        );

        db.insert_topic("t-auth", "doc-hier", "Authentication", None)
            .unwrap();
        db.insert_concept("c-jwt", "t-auth", "JWT Tokens", None)
            .unwrap();

        db.set_chunk_hierarchy("c-hier-0", "t-auth", Some("c-jwt"))
            .unwrap();
        db.set_chunk_hierarchy("c-hier-1", "t-auth", Some("c-jwt"))
            .unwrap();
        db.set_chunk_hierarchy("c-hier-2", "t-auth", None).unwrap();
    }

    #[test]
    fn test_search_hierarchical_with_breadcrumb() {
        let db = test_db();
        setup_hierarchy(&db);

        let provider = FakeProvider {
            vector: vec![1.0, 0.0],
        };
        let cfg = RetrievalConfig {
            top_k: 5,
            evidence_floor: 0.3,
            confidence_threshold: 0.5,
            use_hierarchy: true,
        };

        let results = search(
            &db,
            &provider,
            &cfg,
            &rl_cfg(),
            "JWT expiry",
            None,
            None,
            None,
        )
        .unwrap();
        assert!(!results.is_empty());

        // First result should have hierarchy fields populated
        let first = &results[0];
        assert_eq!(first.topic_name.as_deref(), Some("Authentication"));
        assert_eq!(first.concept_name.as_deref(), Some("JWT Tokens"));
        assert!(first.breadcrumb.is_some());
        let bc = first.breadcrumb.as_ref().unwrap();
        assert!(bc.contains("Authentication"));
        assert!(bc.contains("JWT Tokens"));
    }

    #[test]
    fn test_search_flat_fallback_no_hierarchy() {
        let db = test_db();
        insert_doc(&db, "doc-flat", DocumentStatus::Ready);
        insert_chunk_with_embedding(&db, "doc-flat", "c-flat-0", "some text", vec![1.0, 0.0]);

        let provider = FakeProvider {
            vector: vec![1.0, 0.0],
        };
        let cfg = RetrievalConfig {
            top_k: 5,
            evidence_floor: 0.3,
            confidence_threshold: 0.5,
            use_hierarchy: true,
        };

        let results = search(&db, &provider, &cfg, &rl_cfg(), "query", None, None, None).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].topic_name.is_none());
        assert!(results[0].concept_name.is_none());
        assert!(results[0].breadcrumb.is_none());
    }

    #[test]
    fn test_search_flat_flag_returns_no_breadcrumb() {
        let db = test_db();
        setup_hierarchy(&db);

        let provider = FakeProvider {
            vector: vec![1.0, 0.0],
        };
        let cfg = RetrievalConfig {
            top_k: 5,
            evidence_floor: 0.3,
            confidence_threshold: 0.5,
            use_hierarchy: false,
        };

        let results = search(
            &db,
            &provider,
            &cfg,
            &rl_cfg(),
            "JWT expiry",
            None,
            None,
            None,
        )
        .unwrap();
        assert!(!results.is_empty());
        // Even though hierarchy data exists, use_hierarchy=false forces flat path
        assert!(results[0].breadcrumb.is_none());
        assert!(results[0].topic_name.is_none());
        assert!(results[0].concept_name.is_none());
    }

    #[test]
    fn test_search_hierarchical_auto_fallback() {
        let db = test_db();
        // No hierarchy data inserted — just a plain doc
        insert_doc(&db, "doc-plain", DocumentStatus::Ready);
        insert_chunk_with_embedding(&db, "doc-plain", "c-plain-0", "plain text", vec![1.0, 0.0]);

        let provider = FakeProvider {
            vector: vec![1.0, 0.0],
        };
        let cfg = RetrievalConfig {
            top_k: 5,
            evidence_floor: 0.3,
            confidence_threshold: 0.5,
            use_hierarchy: true, // wants hierarchy, but no data exists
        };

        // Should auto-fallback to flat and still return results
        let results = search(&db, &provider, &cfg, &rl_cfg(), "query", None, None, None).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].breadcrumb.is_none());
    }

    #[test]
    fn test_search_with_topic_filter() {
        let db = test_db();

        // Doc with two topics
        insert_doc(&db, "doc-multi", DocumentStatus::Ready);
        insert_chunk_with_embedding(
            &db,
            "doc-multi",
            "c-auth-0",
            "JWT token expiry",
            vec![1.0, 0.0],
        );
        insert_chunk_with_embedding(
            &db,
            "doc-multi",
            "c-log-0",
            "Logging with ELK",
            vec![0.0, 1.0],
        );

        db.insert_topic("t-auth", "doc-multi", "Authentication", None)
            .unwrap();
        db.insert_topic("t-log", "doc-multi", "Logging", None)
            .unwrap();

        db.set_chunk_hierarchy("c-auth-0", "t-auth", None).unwrap();
        db.set_chunk_hierarchy("c-log-0", "t-log", None).unwrap();

        let provider = FakeProvider {
            vector: vec![1.0, 0.0],
        };
        let cfg = RetrievalConfig {
            top_k: 5,
            evidence_floor: 0.3,
            confidence_threshold: 0.5,
            use_hierarchy: true,
        };

        // Filter by topic t-auth — should only return the auth chunk
        let results = search(
            &db,
            &provider,
            &cfg,
            &rl_cfg(),
            "JWT",
            None,
            Some("t-auth"),
            None,
        )
        .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].chunk_id, "c-auth-0");
        assert_eq!(results[0].topic_name.as_deref(), Some("Authentication"));
    }

    #[test]
    fn test_search_with_concept_filter() {
        let db = test_db();

        insert_doc(&db, "doc-concepts", DocumentStatus::Ready);
        insert_chunk_with_embedding(
            &db,
            "doc-concepts",
            "c-jwt-0",
            "JWT expiry 15 min",
            vec![1.0, 0.0],
        );
        insert_chunk_with_embedding(
            &db,
            "doc-concepts",
            "c-pw-0",
            "Password min 12 chars",
            vec![0.8, 0.2],
        );

        db.insert_topic("t-auth", "doc-concepts", "Authentication", None)
            .unwrap();
        db.insert_concept("c-jwt", "t-auth", "JWT Tokens", None)
            .unwrap();
        db.insert_concept("c-pw", "t-auth", "Password Policy", None)
            .unwrap();

        db.set_chunk_hierarchy("c-jwt-0", "t-auth", Some("c-jwt"))
            .unwrap();
        db.set_chunk_hierarchy("c-pw-0", "t-auth", Some("c-pw"))
            .unwrap();

        let provider = FakeProvider {
            vector: vec![1.0, 0.0],
        };
        let cfg = RetrievalConfig {
            top_k: 5,
            evidence_floor: 0.3,
            confidence_threshold: 0.5,
            use_hierarchy: true,
        };

        // Filter by concept c-jwt — should only return the JWT chunk
        let results = search(
            &db,
            &provider,
            &cfg,
            &rl_cfg(),
            "token",
            None,
            None,
            Some("c-jwt"),
        )
        .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].chunk_id, "c-jwt-0");
        assert_eq!(results[0].concept_name.as_deref(), Some("JWT Tokens"));
    }
}
