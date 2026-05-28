use common::types::{EvalReport, FixtureResult, ResultKind};
use config::{RateLimitConfig, RetrievalConfig};
use providers::EmbeddingProvider;
use storage::Database;

use crate::context::build_context;

/// A single golden-fixture input for evaluation.
#[derive(Debug, Clone)]
pub struct GoldenFixture {
    pub fixture_id: String,
    pub category: String,
    pub query: String,
    pub expected: FixtureExpected,
}

/// Expected outcome for a golden fixture.
#[derive(Debug, Clone)]
pub struct FixtureExpected {
    pub result_kind: ResultKind,
    pub min_citations: usize,
}

/// Run the full golden-dataset evaluation against the context pipeline.
pub fn run_evaluation(
    db: &Database,
    provider: &dyn EmbeddingProvider,
    config: &RetrievalConfig,
    rate_limit: &RateLimitConfig,
    fixtures: &[GoldenFixture],
    threshold: f64,
) -> EvalReport {
    let results: Vec<FixtureResult> = fixtures
        .iter()
        .map(|f| evaluate_fixture(db, provider, config, rate_limit, f))
        .collect();

    let total = results.len() as u32;
    let passed = results.iter().filter(|r| r.passed).count() as u32;
    let failed = total - passed;
    let hit_rate = if total > 0 {
        passed as f64 / total as f64
    } else {
        0.0
    };
    let overall_pass = hit_rate >= threshold;

    EvalReport {
        total,
        passed,
        failed,
        hit_rate,
        threshold,
        overall_pass,
        results,
    }
}

/// Evaluate a single fixture by running `build_context` and comparing expectations.
fn evaluate_fixture(
    db: &Database,
    provider: &dyn EmbeddingProvider,
    config: &RetrievalConfig,
    rate_limit: &RateLimitConfig,
    fixture: &GoldenFixture,
) -> FixtureResult {
    let ctx = build_context(db, provider, config, rate_limit, &fixture.query, None);

    match ctx {
        Ok(response) => {
            let actual_citation_count = response.citations.len();
            let mut passed = true;
            let mut failure_reason = None;

            if response.result_kind != fixture.expected.result_kind {
                passed = false;
                failure_reason = Some(format!(
                    "expected result_kind={}, got={}",
                    fixture.expected.result_kind, response.result_kind
                ));
            } else if actual_citation_count < fixture.expected.min_citations {
                passed = false;
                failure_reason = Some(format!(
                    "expected >={} citations, got {}",
                    fixture.expected.min_citations, actual_citation_count
                ));
            }

            FixtureResult {
                fixture_id: fixture.fixture_id.clone(),
                category: fixture.category.clone(),
                passed,
                actual_result_kind: response.result_kind,
                actual_citation_count,
                failure_reason,
            }
        }
        Err(e) => FixtureResult {
            fixture_id: fixture.fixture_id.clone(),
            category: fixture.category.clone(),
            passed: false,
            actual_result_kind: ResultKind::NoResults,
            actual_citation_count: 0,
            failure_reason: Some(format!("pipeline error: {e}")),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use common::types::{Chunk, Document, DocumentStatus, FileType};
    use common::HarnessError;
    use std::path::PathBuf;

    struct FakeProvider {
        vector: Vec<f32>,
    }
    impl EmbeddingProvider for FakeProvider {
        fn embed(&self, _: &str) -> Result<Vec<f32>, HarnessError> {
            Ok(self.vector.clone())
        }
        fn model_id(&self) -> &str {
            "eval-test-model"
        }
        fn provider_id(&self) -> &str {
            "eval-test-provider"
        }
    }

    fn db() -> Database {
        Database::open_memory().unwrap()
    }
    fn cfg() -> RetrievalConfig {
        RetrievalConfig {
            top_k: 5,
            evidence_floor: 0.3,
            confidence_threshold: 0.5,
        }
    }
    fn rl() -> RateLimitConfig {
        RateLimitConfig {
            max_requests: 100,
            window_seconds: 60,
        }
    }

    fn seed_corpus(db: &Database) {
        let doc = Document {
            document_id: "d1".into(),
            display_name: "d1.txt".into(),
            file_path: PathBuf::from("/docs/d1.txt"),
            file_type: FileType::Txt,
            file_size_bytes: 100,
            status: DocumentStatus::Ready,
            chunk_count: 1,
            retry_count: 0,
            max_retry_count: 3,
            next_retry_at: None,
            error: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        db.insert_document(&doc).unwrap();
        let chunk = Chunk {
            chunk_id: "c1".into(),
            document_id: "d1".into(),
            section_id: Some("s1".into()),
            chunk_index: 0,
            text: "relevant text".into(),
            page: Some(1),
            offset_start: Some(0),
            offset_end: Some(13),
            created_at: Utc::now(),
        };
        db.insert_chunks("d1", &[chunk]).unwrap();
        db.insert_embeddings(&[("c1".into(), vec![1.0, 0.0], "m", "p")])
            .unwrap();
    }

    fn fixture(id: &str, kind: ResultKind, min_c: usize) -> GoldenFixture {
        GoldenFixture {
            fixture_id: id.into(),
            category: "basic".into(),
            query: "relevant".into(),
            expected: FixtureExpected {
                result_kind: kind,
                min_citations: min_c,
            },
        }
    }

    #[test]
    fn test_evaluation_all_pass() {
        let db = db();
        seed_corpus(&db);
        let provider = FakeProvider {
            vector: vec![1.0, 0.0],
        };
        let fixtures = vec![fixture("f1", ResultKind::Context, 1)];
        let report = run_evaluation(&db, &provider, &cfg(), &rl(), &fixtures, 1.0);
        assert_eq!(report.total, 1);
        assert_eq!(report.passed, 1);
        assert_eq!(report.failed, 0);
        assert!((report.hit_rate - 1.0).abs() < f64::EPSILON);
        assert!(report.overall_pass);
        assert!(report.results[0].failure_reason.is_none());
    }

    #[test]
    fn test_evaluation_partial_fail() {
        let db = db();
        seed_corpus(&db);
        let provider = FakeProvider {
            vector: vec![1.0, 0.0],
        };
        let fixtures = vec![
            fixture("f1", ResultKind::Context, 1),
            fixture("f2", ResultKind::Context, 10), // 10 citations impossible → fail
        ];
        let report = run_evaluation(&db, &provider, &cfg(), &rl(), &fixtures, 1.0);
        assert_eq!(report.total, 2);
        assert_eq!(report.passed, 1);
        assert_eq!(report.failed, 1);
        assert!((report.hit_rate - 0.5).abs() < f64::EPSILON);
        assert!(!report.overall_pass);
        assert!(report.results[1].failure_reason.is_some());
    }

    #[test]
    fn test_hit_rate_computation() {
        let db = db();
        seed_corpus(&db);
        let provider = FakeProvider {
            vector: vec![0.0, 1.0], // score=0.0, below floor → NoResults
        };
        let fixtures = vec![
            fixture("f1", ResultKind::NoResults, 0),
            fixture("f2", ResultKind::NoResults, 0),
            fixture("f3", ResultKind::Context, 1), // wrong expectation → fail
        ];
        let report = run_evaluation(&db, &provider, &cfg(), &rl(), &fixtures, 0.5);
        assert_eq!(report.total, 3);
        assert_eq!(report.passed, 2);
        assert_eq!(report.failed, 1);
        assert!((report.hit_rate - 2.0 / 3.0).abs() < f64::EPSILON);
        assert!(report.overall_pass); // 2/3 > 0.5 threshold
    }
}
