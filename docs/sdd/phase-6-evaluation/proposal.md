# SDD Proposal — Phase 6: Evaluation

## Problem

The AI Harness CLI has a complete retrieval pipeline (Phases 1–5) but no automated way to verify that retrieval actually returns correct, relevant results for known queries. Without evaluation tooling, we cannot:

1. Confirm the 80% top-5 hit rate acceptance criterion
2. Detect regressions when changing chunking, ranking, or context assembly
3. Validate runtime mode enforcement (public demo blocks uploads, production blocked)
4. Demonstrate retrieval quality to reviewers

## Proposed solution

Add an evaluation framework with three components:

### 1. Golden dataset (corpus + fixtures)
A small, curated sample corpus (3 documents, 10+ facts) with 8 query fixtures covering all acceptance criteria scenarios:
- 3 direct-fact queries (must find correct chunk in top-5)
- 2 no-results queries (must return `no_results` with no citations)
- 1 ambiguous query (must return `insufficient_context` or cautious metadata)
- 1 multi-chunk query (must return 2+ citations)
- 1 prompt-injection query (must treat document text as source, not instructions)

### 2. Deterministic evaluation engine
A `GoldenProvider` (mock embedding provider) that returns pre-computed vectors for the golden corpus, making evaluation fully deterministic and API-free. An evaluation engine loads fixtures, runs each query through the existing context pipeline, and computes pass/fail per fixture plus overall hit rate.

### 3. CLI `evaluate` command
A `harness evaluate` command that runs the evaluation suite and outputs structured results (per-fixture pass/fail, hit rate, summary). Exits 0 on all-pass, 1 on any failure.

## Scope

### In scope
- Golden corpus documents (3 files: 2 TXT, 1 MD)
- Golden fixture definitions (8 queries with expected outcomes)
- `GoldenProvider` with pre-computed vectors
- Integration test module (`tests/golden/`)
- Evaluation engine (`crates/engine/src/evaluate.rs`)
- CLI `evaluate` command
- Runtime mode enforcement tests

### Out of scope
- Changes to existing retrieval/context/ingest contracts
- New embedding providers or provider changes
- ANN/reranking upgrades
- Packaging/release (Phase 7)
- Answer-generation layer
- Changes to existing unit tests

## Delivery strategy

3 slices, each under 400 changed lines:

| Slice | Content | Est. lines |
|-------|---------|------------|
| 1 | Golden corpus + fixtures + GoldenProvider + integration tests | ~350 |
| 2 | Evaluation engine + EvalReport types | ~250 |
| 3 | CLI evaluate command + runtime mode tests | ~300 |

## Dependencies

- Phase 1–5 complete ✅
- Existing `FakeProvider` pattern for test embedding providers
- `Database::open_memory()` for in-memory test databases
- Existing context/retrieve engine APIs

## Risks

| Risk | Mitigation |
|------|-----------|
| Pre-computed vectors break if chunker changes | Golden corpus is small; re-compute in helper when chunker changes |
| Golden dataset too small to be meaningful | 8 fixtures cover all acceptance criteria; expand iteratively |
| Review budget overrun | 3 slices keep each under 400 lines |

## Recommendation

Proceed to spec. The evaluation framework is well-scoped, low-risk, and directly addresses acceptance criteria that cannot be validated any other way.
