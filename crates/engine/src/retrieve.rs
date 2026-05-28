use common::CiteError;
use config::{RateLimitConfig, RetrievalConfig};
use providers::EmbeddingProvider;
use retrieval::rank_by_similarity;
use storage::Database;

const MAX_QUERY_CHARS: usize = 4000;
const MIN_K: u32 = 1;
const MAX_K: u32 = 10;
const SEARCH_PREVIEW_CHARS: usize = 160;
const SEARCH_ROUTE: &str = "search";
const RETRIEVE_ROUTE: &str = "retrieve";

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub chunk_id: String,
    pub document_id: String,
    pub display_name: String,
    pub section_id: Option<String>,
    pub chunk_index: u32,
    pub page: Option<u32>,
    pub offset_start: Option<u32>,
    pub offset_end: Option<u32>,
    pub score: f32,
    pub preview: String,
}

#[derive(Debug, Clone)]
pub struct RetrieveHit {
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
}

pub fn search(
    db: &Database,
    provider: &dyn EmbeddingProvider,
    config: &RetrievalConfig,
    rate_limit: &RateLimitConfig,
    query: &str,
    k_override: Option<u32>,
) -> Result<Vec<SearchHit>, CiteError> {
    let k = resolve_k(config, k_override)?;
    validate_query(query)?;
    enforce_rate_limit(db, provider, rate_limit, SEARCH_ROUTE)?;

    let query_vector = provider.embed(query)?;
    let candidates = db.list_ready_chunk_embeddings()?;
    let ranked = rank_by_similarity(&query_vector, &candidates, k as usize);

    Ok(ranked
        .into_iter()
        .map(|item| SearchHit {
            chunk_id: item.chunk_id,
            document_id: item.document_id,
            display_name: item.display_name,
            section_id: item.section_id,
            chunk_index: item.chunk_index,
            page: item.page,
            offset_start: item.offset_start,
            offset_end: item.offset_end,
            score: item.score,
            preview: make_preview(&item.text),
        })
        .collect())
}

pub fn retrieve(
    db: &Database,
    provider: &dyn EmbeddingProvider,
    config: &RetrievalConfig,
    rate_limit: &RateLimitConfig,
    query: &str,
    k_override: Option<u32>,
) -> Result<Vec<RetrieveHit>, CiteError> {
    let k = resolve_k(config, k_override)?;
    validate_query(query)?;
    enforce_rate_limit(db, provider, rate_limit, RETRIEVE_ROUTE)?;

    let query_vector = provider.embed(query)?;
    let candidates = db.list_ready_chunk_embeddings()?;
    let ranked = rank_by_similarity(&query_vector, &candidates, k as usize);

    Ok(ranked
        .into_iter()
        .map(|item| RetrieveHit {
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
        })
        .collect())
}

fn rate_limit_key(provider: &dyn EmbeddingProvider) -> String {
    provider.provider_id().to_string()
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
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = normalized.chars();
    let preview: String = chars.by_ref().take(SEARCH_PREVIEW_CHARS).collect();
    if chars.next().is_some() {
        format!("{preview}…")
    } else {
        preview
    }
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
        };

        let err = search(&db, &provider, &cfg, &rl_cfg(), "   ", None).unwrap_err();
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
        };

        let err = search(&db, &provider, &cfg, &rl_cfg(), "hello", None).unwrap_err();
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
        };

        let results = search(&db, &provider, &cfg, &rl_cfg(), "query", None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "doc-ready");
        assert!(results[0].preview.contains("ready text"));
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
        };

        let results = retrieve(&db, &provider, &cfg, &rl_cfg(), "query", Some(1)).unwrap();
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
        };

        let results = search(&db, &provider, &cfg, &rl_cfg(), "query", None).unwrap();
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
        };

        let query = "a".repeat(4001);
        let err = search(&db, &provider, &cfg, &rl_cfg(), &query, None).unwrap_err();
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
        };
        let rl = RateLimitConfig {
            max_requests: 1,
            window_seconds: 60,
        };

        assert!(search(&db, &provider, &cfg, &rl, "query", None).is_ok());
        let err = search(&db, &provider, &cfg, &rl, "query", None).unwrap_err();
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
        };
        let rl = RateLimitConfig {
            max_requests: 1,
            window_seconds: 60,
        };

        assert!(retrieve(&db, &provider, &cfg, &rl, "query", None).is_ok());
        let err = retrieve(&db, &provider, &cfg, &rl, "query", None).unwrap_err();
        assert!(matches!(
            err,
            CiteError::RateLimitExceeded {
                retry_after_seconds: _
            }
        ));
    }
}
