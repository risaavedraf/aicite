use common::types::ResultKind;
use engine::evaluate;

/// Expected outcome for a golden fixture.
#[derive(Debug, Clone)]
pub struct FixtureExpected {
    pub result_kind: ResultKind,
    pub min_citations: usize,
    pub must_contain_chunk_texts: &'static [&'static str],
    pub confidence_label_required: bool,
}

/// A golden test fixture with query and expected outcome.
#[derive(Debug, Clone)]
pub struct GoldenFixture {
    pub fixture_id: String,
    pub query: String,
    pub category: String,
    pub expected: FixtureExpected,
    #[allow(dead_code)]
    pub description: &'static str,
}

struct GoldenFixtureOverlay {
    fixture_id: &'static str,
    must_contain_chunk_texts: &'static [&'static str],
    confidence_label_required: bool,
    description: &'static str,
}

const GOLDEN_FIXTURE_OVERLAYS: &[GoldenFixtureOverlay] = &[
    GoldenFixtureOverlay {
        fixture_id: "df-001",
        must_contain_chunk_texts: &["API gateway routes all external requests"],
        confidence_label_required: false,
        description: "Direct fact: API gateway routes requests and validates auth tokens",
    },
    GoldenFixtureOverlay {
        fixture_id: "df-002",
        must_contain_chunk_texts: &["PostgreSQL with read replicas"],
        confidence_label_required: false,
        description: "Direct fact: PostgreSQL with read replicas for high availability",
    },
    GoldenFixtureOverlay {
        fixture_id: "df-003",
        must_contain_chunk_texts: &["minimum 12 characters"],
        confidence_label_required: false,
        description: "Direct fact: Password minimum 12 characters with complexity rules",
    },
    GoldenFixtureOverlay {
        fixture_id: "nr-001",
        must_contain_chunk_texts: &[],
        confidence_label_required: false,
        description: "No results: completely unrelated topic",
    },
    GoldenFixtureOverlay {
        fixture_id: "nr-002",
        must_contain_chunk_texts: &[],
        confidence_label_required: false,
        description: "No results: physics topic outside corpus scope",
    },
    GoldenFixtureOverlay {
        fixture_id: "amb-001",
        must_contain_chunk_texts: &[],
        confidence_label_required: false,
        description: "Ambiguous: broad multi-topic query spanning architecture and security",
    },
    GoldenFixtureOverlay {
        fixture_id: "mc-001",
        must_contain_chunk_texts: &[],
        confidence_label_required: false,
        description: "Multi-chunk: requires citations about both authentication and rate limiting",
    },
    GoldenFixtureOverlay {
        fixture_id: "pi-001",
        must_contain_chunk_texts: &[],
        confidence_label_required: false,
        description:
            "Prompt injection: document text about injection attacks treated as source material",
    },
    GoldenFixtureOverlay {
        fixture_id: "hier-001",
        must_contain_chunk_texts: &["PostgreSQL"],
        confidence_label_required: false,
        description: "Hierarchical retrieval: database query with topic enrichment",
    },
    GoldenFixtureOverlay {
        fixture_id: "hier-002",
        must_contain_chunk_texts: &["password", "characters"],
        confidence_label_required: false,
        description: "Hierarchical retrieval: password validation with topic context",
    },
];

/// Load golden fixtures from the canonical engine fixture source, overlaying only
/// integration-test-specific assertions that are not part of `cite evaluate`.
pub fn load_fixtures() -> Vec<GoldenFixture> {
    evaluate::golden_fixtures()
        .into_iter()
        .map(|fixture| {
            let overlay = GOLDEN_FIXTURE_OVERLAYS
                .iter()
                .find(|overlay| overlay.fixture_id == fixture.fixture_id)
                .unwrap_or_else(|| {
                    panic!("missing golden fixture overlay for {}", fixture.fixture_id)
                });

            GoldenFixture {
                fixture_id: fixture.fixture_id,
                query: fixture.query,
                category: fixture.category,
                expected: FixtureExpected {
                    result_kind: fixture.expected.result_kind,
                    min_citations: fixture.expected.min_citations,
                    must_contain_chunk_texts: overlay.must_contain_chunk_texts,
                    confidence_label_required: overlay.confidence_label_required,
                },
                description: overlay.description,
            }
        })
        .collect()
}
