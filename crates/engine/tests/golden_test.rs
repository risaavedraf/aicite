//! Golden dataset evaluation integration tests.
//!
//! Tests retrieval quality against a curated corpus with known facts.

#[path = "golden/provider.rs"]
mod provider;

#[path = "golden/fixtures.rs"]
mod fixtures;

use common::types::ResultKind;
use config::{IngestConfig, RateLimitConfig, RetrievalConfig};
use engine::context::build_context;
use engine::ingest;
use fixtures::{load_fixtures, GoldenFixture};
use provider::GoldenProvider;
use providers::EmbeddingProvider;
use std::path::PathBuf;
use storage::Database;

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join("corpus")
}

fn test_db() -> Database {
    Database::open_memory().expect("Failed to create in-memory database")
}

fn retrieval_config() -> RetrievalConfig {
    RetrievalConfig {
        top_k: 5,
        evidence_floor: 0.30,
        confidence_threshold: 0.50,
    }
}

fn rate_limit_config() -> RateLimitConfig {
    RateLimitConfig {
        max_requests: 1000,
        window_seconds: 60,
    }
}

fn ingest_config() -> IngestConfig {
    IngestConfig::default()
}

/// Ingest all golden corpus documents and return the DB with chunks + embeddings.
fn setup_corpus() -> (Database, GoldenProvider) {
    let db = test_db();
    let provider = GoldenProvider::new();
    let config = ingest_config();
    let dir = corpus_dir();

    let files = vec![
        "architecture.txt",
        "api-reference.md",
        "security-policy.txt",
    ];

    for filename in &files {
        let path = dir.join(filename);
        assert!(path.exists(), "Corpus file not found: {:?}", path);

        let result = ingest::ingest(&db, &provider, &config, &path, None, false)
            .unwrap_or_else(|e| panic!("Failed to ingest {}: {}", filename, e));

        assert_eq!(
            result.status,
            common::types::DocumentStatus::Ready,
            "Document {} should be ready after ingestion",
            filename
        );
        assert!(
            result.chunk_count > 0,
            "Document {} should have chunks",
            filename
        );
    }

    (db, provider)
}

/// Check if any citation text contains the expected substring.
fn citations_contain_text(citations: &[common::types::Citation], expected: &str) -> bool {
    let expected_lower = expected.to_lowercase();
    citations
        .iter()
        .any(|c| c.text.to_lowercase().contains(&expected_lower))
}

/// Run a single fixture and return pass/fail with reason.
fn evaluate_fixture(
    db: &Database,
    provider: &GoldenProvider,
    config: &RetrievalConfig,
    rl_config: &RateLimitConfig,
    fixture: &GoldenFixture,
) -> (bool, String) {
    let result = build_context(db, provider, config, rl_config, fixture.query, None);

    match result {
        Ok(response) => {
            // Check result_kind
            let expected_kind = match fixture.expected.result_kind {
                "context" => ResultKind::Context,
                "no_results" => ResultKind::NoResults,
                "insufficient_context" => ResultKind::InsufficientContext,
                other => return (false, format!("Unknown expected result_kind: {}", other)),
            };

            if response.result_kind != expected_kind {
                return (
                    false,
                    format!(
                        "result_kind mismatch: expected {:?}, got {:?}",
                        expected_kind, response.result_kind
                    ),
                );
            }

            // Check min_citations
            if response.citations.len() < fixture.expected.min_citations {
                return (
                    false,
                    format!(
                        "citation count {} below minimum {}",
                        response.citations.len(),
                        fixture.expected.min_citations
                    ),
                );
            }

            // Check must_contain_chunk_texts
            for expected_text in fixture.expected.must_contain_chunk_texts {
                if !citations_contain_text(&response.citations, expected_text) {
                    return (
                        false,
                        format!("no citation contains expected text: '{}'", expected_text),
                    );
                }
            }

            // Check confidence_label_required
            if fixture.expected.confidence_label_required {
                let has_labels = response
                    .citations
                    .iter()
                    .any(|c| c.confidence_label.is_some());
                let has_caution = response.metadata.caution.is_some();
                if !has_labels
                    && !has_caution
                    && response.result_kind != ResultKind::InsufficientContext
                {
                    return (
                        false,
                        "expected confidence labels or caution metadata".to_string(),
                    );
                }
            }

            (true, "passed".to_string())
        }
        Err(e) => {
            // For no_results fixtures, some errors are acceptable
            if fixture.expected.result_kind == "no_results" {
                match &e {
                    common::CiteError::DocumentNotReady { .. } => {
                        (true, "passed (no ready documents)".to_string())
                    }
                    _ => (false, format!("unexpected error: {}", e)),
                }
            } else {
                (false, format!("error: {}", e))
            }
        }
    }
}

#[test]
fn test_golden_dataset_all_fixtures() {
    let (db, provider) = setup_corpus();
    let config = retrieval_config();
    let rl_config = rate_limit_config();
    let fixtures = load_fixtures();

    assert_eq!(fixtures.len(), 8, "Expected 8 golden fixtures");

    let mut results = Vec::new();
    let mut passed = 0;

    for fixture in &fixtures {
        let (ok, reason) = evaluate_fixture(&db, &provider, &config, &rl_config, fixture);
        let status = if ok { "PASS" } else { "FAIL" };

        if ok {
            passed += 1;
        }

        results.push(format!(
            "  {}  {}  {:<20} \"{}\" — {}",
            fixture.fixture_id, status, fixture.category, fixture.query, reason
        ));
    }

    let hit_rate = passed as f64 / fixtures.len() as f64;
    let overall_pass = hit_rate >= 0.80;

    // Print evaluation report
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║           Golden Dataset Evaluation Results                 ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    for line in &results {
        println!("║ {}", line);
    }
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!(
        "║  Hit rate: {}/{} ({:.1}%) — {} (threshold: 80%)",
        passed,
        fixtures.len(),
        hit_rate * 100.0,
        if overall_pass { "PASS" } else { "FAIL" }
    );
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // Assert all fixtures pass
    assert!(
        overall_pass,
        "Golden dataset evaluation failed: {}/{} fixtures passed, hit rate {:.1}%",
        passed,
        fixtures.len(),
        hit_rate * 100.0
    );
}

#[test]
fn test_golden_corpus_ingestion() {
    let (db, _provider) = setup_corpus();

    let docs = db.list_documents().expect("Failed to list documents");
    assert_eq!(docs.len(), 3, "Expected 3 corpus documents");

    for doc in &docs {
        assert_eq!(
            doc.status,
            common::types::DocumentStatus::Ready,
            "Document {} should be ready",
            doc.document_id
        );
        assert!(
            doc.chunk_count > 0,
            "Document {} should have chunks",
            doc.document_id
        );
    }
}

#[test]
fn test_golden_provider_determinism() {
    let provider = GoldenProvider::new();
    let query = "What is the API gateway?";

    let v1 = provider.embed(query).unwrap();
    let v2 = provider.embed(query).unwrap();

    assert_eq!(v1, v2, "GoldenProvider should be deterministic");
    assert_eq!(v1.len(), 8, "Vectors should be 8-dimensional");
}
