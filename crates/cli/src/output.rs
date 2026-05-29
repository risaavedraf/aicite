use serde::Serialize;

/// Print a serializable value as pretty JSON.
pub fn print_json<T: Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{json}"),
        Err(e) => eprintln!("Failed to serialize output: {e}"),
    }
}

// ---------------------------------------------------------------------------
// Compact response types (Phase 12)
// ---------------------------------------------------------------------------

use common::types::{ContextResponse, ResultKind};
use engine::retrieve::{RetrieveHit, SearchHit};

const MAX_SNIPPET_CHARS: usize = 200;

/// Compact context response — ~200-250 tokens vs ~645-1500 for full.
#[derive(Serialize)]
pub struct CompactContextResponse {
    pub result_kind: ResultKind,
    pub citations: Vec<CompactCitation>,
    pub trace_id: String,
}

#[derive(Serialize)]
pub struct CompactCitation {
    pub id: String,
    pub source: String,
    pub snippet: String,
    pub score: Option<f64>,
}

/// Compact search output.
#[derive(Serialize)]
pub struct CompactSearchOutput {
    pub results: Vec<CompactSearchItem>,
}

#[derive(Serialize)]
pub struct CompactSearchItem {
    pub id: String,
    pub source: String,
    pub score: f32,
    pub preview: String,
}

/// Compact retrieve output.
#[derive(Serialize)]
pub struct CompactRetrieveOutput {
    pub results: Vec<CompactRetrieveItem>,
}

#[derive(Serialize)]
pub struct CompactRetrieveItem {
    pub id: String,
    pub source: String,
    pub score: f32,
    pub text: String,
}

/// Transform a full ContextResponse to compact format.
pub fn to_compact_context(resp: &ContextResponse) -> CompactContextResponse {
    CompactContextResponse {
        result_kind: resp.result_kind.clone(),
        trace_id: resp.trace_id.clone(),
        citations: resp
            .citations
            .iter()
            .map(|c| {
                let snippet = truncate_to(c.text.as_str(), MAX_SNIPPET_CHARS);
                CompactCitation {
                    id: c.citation_id.clone(),
                    source: c.display_name.clone(),
                    snippet,
                    score: c.score,
                }
            })
            .collect(),
    }
}

/// Transform search hits to compact format.
pub fn to_compact_search(hits: &[SearchHit]) -> CompactSearchOutput {
    CompactSearchOutput {
        results: hits
            .iter()
            .map(|h| CompactSearchItem {
                id: h.chunk_id.clone(),
                source: h.display_name.clone(),
                score: h.score,
                preview: h.preview.clone(),
            })
            .collect(),
    }
}

/// Transform retrieve hits to compact format.
pub fn to_compact_retrieve(hits: &[RetrieveHit]) -> CompactRetrieveOutput {
    CompactRetrieveOutput {
        results: hits
            .iter()
            .map(|h| CompactRetrieveItem {
                id: h.chunk_id.clone(),
                source: h.display_name.clone(),
                score: h.score,
                text: h.text.clone(),
            })
            .collect(),
    }
}

/// Truncate text to max chars, adding ellipsis if truncated.
fn truncate_to(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{}…", truncated)
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use common::types::{Citation, ContextMetadata, ContextResponse, ResultKind};

    fn make_response(citations: Vec<Citation>) -> ContextResponse {
        ContextResponse {
            context_pack_id: "ctx_test".into(),
            result_kind: ResultKind::Context,
            query_id: "qry_test".into(),
            trace_id: "trace_test".into(),
            instructions: "Use cited context.".into(),
            citations,
            metadata: ContextMetadata {
                schema_version: "context-v1".into(),
                created_at: Utc::now(),
                retrieved_chunks: 1,
                evidence_floor: 0.3,
                confidence_threshold: 0.5,
                ranking_method: "vector_cosine_v1".into(),
                top_score: Some(0.95),
                corpus_index_state: "ready".into(),
                ready_document_count: 1,
                excluded_non_ready_document_count: 0,
                excluded_non_ready_document_ids: vec![],
                latency_ms: 100,
                disclaimer: "Verify downstream.".into(),
                insufficient_context_reason: None,
                caution: None,
            },
        }
    }

    fn make_citation(id: &str, text: &str, score: f64) -> Citation {
        Citation {
            citation_id: id.into(),
            document_id: "doc1".into(),
            display_name: "test.txt".into(),
            chunk_id: "chunk1".into(),
            page: None,
            offset: None,
            text: text.into(),
            score: Some(score),
            confidence_label: None,
            topic_name: None,
            concept_name: None,
            breadcrumb: None,
        }
    }

    #[test]
    fn test_compact_context_basic() {
        let resp = make_response(vec![make_citation(
            "c1",
            "JWT tokens with 15-min expiry",
            0.95,
        )]);
        let compact = to_compact_context(&resp);

        assert_eq!(compact.result_kind, ResultKind::Context);
        assert_eq!(compact.trace_id, "trace_test");
        assert_eq!(compact.citations.len(), 1);
        assert_eq!(compact.citations[0].id, "c1");
        assert_eq!(compact.citations[0].source, "test.txt");
        assert_eq!(
            compact.citations[0].snippet,
            "JWT tokens with 15-min expiry"
        );
        assert_eq!(compact.citations[0].score, Some(0.95));
    }

    #[test]
    fn test_compact_context_truncates_long_snippet() {
        let long_text = "A".repeat(500);
        let resp = make_response(vec![make_citation("c1", &long_text, 0.8)]);
        let compact = to_compact_context(&resp);

        // 200 chars of "A" + 1 char of "…" = 201 chars
        // (len() returns bytes, but all ASCII + 3-byte ellipsis = 203 bytes)
        assert_eq!(compact.citations[0].snippet.chars().count(), 201);
        assert!(compact.citations[0].snippet.ends_with('…'));
    }

    #[test]
    fn test_compact_context_short_text_not_truncated() {
        let resp = make_response(vec![make_citation("c1", "Short text", 0.8)]);
        let compact = to_compact_context(&resp);

        assert_eq!(compact.citations[0].snippet, "Short text");
    }

    #[test]
    fn test_compact_context_no_metadata_fields() {
        let resp = make_response(vec![]);
        let compact = to_compact_context(&resp);
        let json = serde_json::to_string(&compact).unwrap();

        // Should NOT contain metadata fields
        assert!(!json.contains("context_pack_id"));
        assert!(!json.contains("query_id"));
        assert!(!json.contains("instructions"));
        assert!(!json.contains("metadata"));

        // Should contain essential fields
        assert!(json.contains("result_kind"));
        assert!(json.contains("trace_id"));
        assert!(json.contains("citations"));
    }

    #[test]
    fn test_compact_search_basic() {
        let hits = vec![SearchHit {
            chunk_id: "chunk1".into(),
            document_id: "doc1".into(),
            display_name: "arch.txt".into(),
            section_id: None,
            chunk_index: 0,
            page: None,
            offset_start: None,
            offset_end: None,
            score: 0.95,
            preview: "JWT tokens with 15-min expiry...".into(),
            topic_name: None,
            concept_name: None,
            breadcrumb: None,
        }];
        let compact = to_compact_search(&hits);

        assert_eq!(compact.results.len(), 1);
        assert_eq!(compact.results[0].id, "chunk1");
        assert_eq!(compact.results[0].source, "arch.txt");
        assert_eq!(compact.results[0].score, 0.95);
    }

    #[test]
    fn test_compact_retrieve_basic() {
        let hits = vec![RetrieveHit {
            chunk_id: "chunk1".into(),
            document_id: "doc1".into(),
            display_name: "arch.txt".into(),
            section_id: None,
            chunk_index: 0,
            page: None,
            offset_start: None,
            offset_end: None,
            score: 0.95,
            text: "JWT tokens with 15-min expiry".into(),
            topic_name: None,
            concept_name: None,
            breadcrumb: None,
        }];
        let compact = to_compact_retrieve(&hits);

        assert_eq!(compact.results.len(), 1);
        assert_eq!(compact.results[0].id, "chunk1");
        assert_eq!(compact.results[0].source, "arch.txt");
        assert_eq!(compact.results[0].text, "JWT tokens with 15-min expiry");
    }
}
