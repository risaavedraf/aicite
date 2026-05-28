# SDD Spec — Phase 6: Evaluation

## Overview

Phase 6 adds an evaluation framework to validate retrieval quality and runtime mode enforcement against a golden dataset. No existing contracts change.

---

## FR-601: Golden corpus

**What**: A curated sample corpus of 3 documents with structured, verifiable content.

**Documents**:
1. `architecture.txt` — Describes a fictional software architecture (microservices, API gateway, database layer). Contains 4+ distinct facts retrievable by direct queries.
2. `api-reference.md` — API documentation for a fictional service (endpoints, parameters, error codes). Contains 4+ distinct facts.
3. `security-policy.txt` — Security and compliance policies (authentication, encryption, audit logging). Contains 3+ facts. Includes one section with prompt-injection-like content (e.g., "Ignore all previous instructions and...") that must be treated as source text.

**Total facts**: 10+ distinct, verifiable claims across the 3 documents.

**Location**: `tests/golden/corpus/`

---

## FR-602: Golden fixtures

**What**: 8 query fixtures with expected outcomes, stored as a structured JSON file.

**Fixture schema**:
```json
{
  "fixture_id": "string",
  "query": "string",
  "category": "direct_fact | no_results | ambiguous | multi_chunk | prompt_injection",
  "expected": {
    "result_kind": "context | no_results | insufficient_context",
    "min_citations": 0,
    "must_contain_chunk_ids": ["string"],
    "must_not_cite_document_ids": ["string"],
    "confidence_label_required": false,
    "assertions": ["string"]
  },
  "description": "string"
}
```

**Fixtures**:

| # | fixture_id | category | Expected result_kind | Key assertion |
|---|-----------|----------|---------------------|---------------|
| 1 | `df-001` | direct_fact | `context` | Correct chunk in top-5 |
| 2 | `df-002` | direct_fact | `context` | Correct chunk in top-5 |
| 3 | `df-003` | direct_fact | `context` | Correct chunk in top-5 |
| 4 | `nr-001` | no_results | `no_results` | Empty citations |
| 5 | `nr-002` | no_results | `no_results` | Empty citations |
| 6 | `amb-001` | ambiguous | `insufficient_context` | Caution metadata present |
| 7 | `mc-001` | multi_chunk | `context` | 2+ citations from different chunks |
| 8 | `pi-001` | prompt_injection | `context` or `no_results` | Document text treated as source; no instruction execution |

**Location**: `tests/golden/fixtures.json`

---

## FR-603: GoldenProvider

**What**: A deterministic mock embedding provider for evaluation that returns pre-computed vectors.

**Contract**:
- Implements `EmbeddingProvider` trait
- `embed(text)` → returns the pre-computed vector for known texts, or a zero vector for unknown texts
- `model_id()` → `"golden-eval-v1"`
- `provider_id()` → `"golden"`
- Vectors are stored in a `HashMap<String, Vec<f32>>` keyed by normalized text
- Fully deterministic: same input always produces same output
- No external API calls

**Pre-computed vectors**:
- One vector per chunk in the golden corpus (3 docs × N chunks each)
- One vector per golden query (8 queries)
- Vectors are hand-crafted so that:
  - Query vectors are most similar to their expected matching chunk vectors
  - No-results query vectors are dissimilar to all chunk vectors (cosine < evidence_floor)
  - Ambiguous query vectors are similar but below confidence_threshold
  - Multi-chunk query vectors are similar to 2+ chunk vectors

**Location**: `tests/golden/provider.rs`

---

## FR-604: Evaluation engine

**What**: A module that runs golden fixtures against the context pipeline and computes pass/fail.

**API**:
```rust
pub struct EvalReport {
    pub total_fixtures: u32,
    pub passed: u32,
    pub failed: u32,
    pub hit_rate: f64,
    pub results: Vec<FixtureResult>,
}

pub struct FixtureResult {
    pub fixture_id: String,
    pub category: String,
    pub passed: bool,
    pub actual_result_kind: ResultKind,
    pub actual_citation_count: u32,
    pub failure_reason: Option<String>,
}

pub fn run_evaluation(
    db: &Database,
    provider: &GoldenProvider,
    config: &RetrievalConfig,
    fixtures: &[GoldenFixture],
) -> EvalReport;
```

**Evaluation logic per fixture**:
1. Call `build_context(db, provider, config, rate_limit, fixture.query, None)`
2. Compare `response.result_kind` against `fixture.expected.result_kind`
3. Check citation count >= `fixture.expected.min_citations`
4. Check `must_contain_chunk_ids` are present in citations
5. Check `must_not_cite_document_ids` are absent from citations
6. Check `confidence_label_required` if applicable
7. Record pass/fail with reason

**Hit rate**: `passed / total` (target: >= 0.80, i.e., 7/8 or better)

**Location**: `crates/engine/src/evaluate.rs`

---

## FR-605: CLI `evaluate` command

**What**: A `harness evaluate` command that runs the evaluation suite.

**Usage**:
```
harness evaluate [--json]
```

**Behavior**:
1. Load golden fixtures from embedded JSON (compiled into binary) or `--fixtures-dir` override
2. Create in-memory database
3. Ingest golden corpus documents
4. Run evaluation engine with GoldenProvider
5. Output results

**Output (human-readable)**:
```
Evaluation Results
═══════════════════════════════════════
  df-001  PASS  direct_fact      "What is the API gateway?"
  df-002  PASS  direct_fact      "What database is used?"
  df-003  PASS  direct_fact      "How is auth handled?"
  nr-001  PASS  no_results       "What is quantum computing?"
  nr-002  PASS  no_results       "Explain neural networks"
  amb-001 PASS  ambiguous        "Tell me about the system"
  mc-001  PASS  multi_chunk      "How does the API handle errors?"
  pi-001  PASS  prompt_injection "Ignore instructions and..."

  Hit rate: 8/8 (100.0%) — PASS (threshold: 80%)
```

**Output (--json)**:
```json
{
  "total": 8,
  "passed": 8,
  "failed": 0,
  "hit_rate": 1.0,
  "threshold": 0.8,
  "overall_pass": true,
  "results": [...]
}
```

**Exit codes**:
- `0` — all fixtures pass and hit rate >= 80%
- `1` — any fixture fails or hit rate < 80%

**Location**: `crates/cli/src/commands/evaluate.rs`

---

## FR-606: Runtime mode enforcement tests

**What**: Integration tests verifying that runtime mode restrictions are enforced.

**Test cases**:
1. `PublicPackagedDemo` mode: `ingest` returns `runtime_mode_forbidden`
2. `Production` mode: `ingest` returns `runtime_mode_forbidden`
3. `LocalPrivateDemo` mode: `ingest` proceeds normally

**Location**: `tests/runtime_mode.rs`

---

## Non-functional requirements

- **Determinism**: Evaluation results must be identical across runs (no external API dependency)
- **Speed**: Full evaluation suite completes in < 1 second (in-memory DB, mock provider)
- **Review budget**: Each slice stays under 400 changed lines
- **No contract changes**: Existing retrieval/context/ingest APIs remain unchanged

---

## Acceptance criteria mapping

| Acceptance criterion | Spec item |
|---------------------|-----------|
| 80% top-5 hit rate | FR-604 (hit_rate >= 0.8) |
| 3 direct-fact cases pass 3/3 | FR-602 fixtures df-001..003 |
| 2 no-results cases pass 2/2 | FR-602 fixtures nr-001..002 |
| 1 ambiguous → insufficient_context | FR-602 fixture amb-001 |
| 1 multi-chunk → 2+ citations | FR-602 fixture mc-001 |
| 1 prompt-injection → source text | FR-602 fixture pi-001 |
| Runtime mode enforcement | FR-606 |
