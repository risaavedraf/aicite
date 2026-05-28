# SDD Explore Notes — Phase 6: Evaluation

## Goal

Define the implementation path for Phase 6 evaluation: golden dataset, sample corpus, retrieval quality metrics, and acceptance testing on top of the completed Phase 1–5 engine.

## Boundary with completed Phase 5

Phases 1–5 are fully implemented and committed:
- Phase 1: scaffold, config, storage, CLI skeleton
- Phase 2: ingest pipeline (extract, chunk, embed, document lifecycle)
- Phase 3: retrieval pipeline (vector search, cosine similarity, search/retrieve commands)
- Phase 4: context packs + citations (context/read/trace commands, result-kind logic)
- Phase 5: durability (locks, backlog, rate limits, retry, refresh, recovery)

Phase 6 must **validate correctness and retrieval quality** without changing any existing output contracts.

---

## Current-state findings

### 1) Test infrastructure

Current state:
- Unit tests exist inline in:
  - `crates/engine/src/context.rs` (14 tests: result kinds, rate limits, traces, read modes)
  - `crates/engine/src/retrieve.rs` (7 tests: search/retrieve happy paths, rate limits, validation)
  - `crates/retrieval/src/lib.rs` (4 tests: cosine similarity, ranking)
  - `crates/config/src/lib.rs` (1 test: defaults)
- E2E test: `crates/ingest/tests/ingest_e2e.rs` (7 tests: validate/extract/chunk pipeline)
- All tests use in-memory SQLite (`Database::open_memory()`)
- All tests use inline `FakeProvider` with fixed vectors
- `cargo test` passes clean (1 warning: unused variable in snapshots.rs)

Gap:
- No golden dataset or corpus-level evaluation tests
- No integration test that exercises ingest → retrieve → context end-to-end with real content
- No retrieval quality metrics (hit rate, precision@k)
- No runtime mode enforcement tests

### 2) Test fixtures

Current state:
- `crates/ingest/tests/fixtures/sample.txt` — minimal text file
- `crates/ingest/tests/fixtures/sample.md` — minimal markdown file
- No structured sample corpus with known facts

Gap:
- Need a sample corpus with 3+ documents containing 10+ verifiable facts
- Need golden queries with known expected answers (ground truth)

### 3) Embedding provider for evaluation

Current state:
- `EmbeddingProvider` trait in `crates/providers/src/lib.rs`: `embed()`, `model_id()`, `provider_id()`
- Two real providers: `openai.rs`, `gemini.rs`
- Tests use `FakeProvider` returning fixed vectors — not suitable for semantic similarity testing

Gap:
- Need a deterministic mock embedding provider that produces vectors preserving semantic similarity for known content
- Options:
  a. **Hash-based mock**: hash text → deterministic vector, similar texts get similar vectors (complex)
  b. **Pre-computed golden vectors**: store expected vectors for golden corpus chunks + queries (simple, deterministic, correct for evaluation)
  c. **TF-IDF-like mock**: simple term-frequency vectorization (reasonable middle ground)

Recommendation: **Option (b)** — pre-computed golden vectors. For evaluation we control both corpus and queries, so we can pre-compute and store expected embeddings. This is fully deterministic, requires no external API, and tests the actual retrieval pipeline (cosine similarity, ranking, context assembly) without depending on embedding quality.

### 4) CLI evaluation command

Current state:
- No `evaluate` or `eval` command in CLI (`crates/cli/src/main.rs`)
- CLI has 11 commands: health, ingest, list, get, retry, search, retrieve, context, read, trace, refresh

Gap:
- Need `harness evaluate` command that runs golden dataset against current corpus
- Should output pass/fail per fixture, overall hit rate, and summary

### 5) Runtime mode enforcement

Current state:
- `RuntimeMode` enum: `PublicPackagedDemo`, `LocalPrivateDemo`, `Production`
- Config loads mode from `HARNESS_RUNTIME_MODE` env var
- `engine::ingest` checks mode for upload restrictions
- No tests verify mode-specific behavior

Gap:
- Need tests that verify:
  - Public packaged demo rejects uploads
  - Production mode blocks uploads
  - Local/private mode allows imports

### 6) Acceptance criteria coverage

From `docs/prd/10-acceptance-criteria.md`, Phase 6 directly addresses:

- [FR-102] Retrieval returns relevant chunks; 80% top-5 hit rate on golden queries
- Golden dataset includes: 3 direct-fact, 2 no-results, 1 ambiguous, 1 multi-chunk, 1 prompt-injection
- Direct-fact cases pass 3/3
- No-results cases pass 2/2
- Prompt-injection fixture confirms document text treated as source, not instructions
- Ambiguous case returns `insufficient_context` or cautious metadata
- Multi-chunk fixture includes 2+ citations

---

## Risks

1. **Pre-computed vector brittleness**
   If chunking logic changes (chunk boundaries, overlap), pre-computed vectors for chunks become stale.
   Mitigation: golden corpus is small and versioned; re-compute vectors in a helper test or build script when chunker changes.

2. **Evaluation determinism**
   If evaluation depends on external APIs (real embedding providers), results vary across runs.
   Mitigation: use deterministic mock provider only; real-provider smoke tests are a separate concern.

3. **Golden dataset scope creep**
   Temptation to add too many fixtures beyond the 8 minimum.
   Mitigation: ship 8 fixtures first, add more iteratively.

4. **Review budget**
   Sample corpus documents + golden dataset + eval command + tests could exceed 400 lines.
   Mitigation: split into slices (corpus/fixtures, eval engine, CLI command).

---

## Non-goals

- No changes to existing retrieval/context/ingest contracts
- No new embedding providers
- No ANN/reranking upgrades
- No packaging/release work (Phase 7)
- No answer-generation layer
- No changes to existing unit tests

---

## Recommended delivery slices (<= 400 changed lines each)

### Slice 1 — Golden corpus + ground truth fixtures
Scope:
- Create `tests/golden/corpus/` with 3 sample documents (TXT/MD) containing 10+ verifiable facts
- Create `tests/golden/fixtures.json` with 8 query fixtures:
  - 3 direct-fact (expected: `context`, specific chunk IDs in top-5)
  - 2 no-results (expected: `no_results`, empty citations)
  - 1 ambiguous (expected: `insufficient_context` or cautious metadata)
  - 1 multi-chunk (expected: `context`, 2+ citations)
  - 1 prompt-injection (expected: document text treated as source)
- Create `tests/golden/ground_truth.rs` — test module that loads fixtures and validates against in-memory DB
- Pre-computed embedding vectors for golden corpus chunks and queries
- Deterministic `GoldenProvider` that returns stored vectors
Target files:
- `tests/golden/corpus/*.txt`, `tests/golden/corpus/*.md` (new)
- `tests/golden/fixtures.json` (new)
- `tests/golden/mod.rs` (new test module)
- `tests/golden/provider.rs` (GoldenProvider)
- `Cargo.toml` (add integration test harness)

### Slice 2 — Retrieval quality metrics + evaluation engine
Scope:
- `crates/engine/src/evaluate.rs` — evaluation engine:
  - load golden fixtures
  - run each query through context pipeline with GoldenProvider
  - compute per-fixture pass/fail
  - compute overall hit rate (top-5)
  - return `EvalReport` struct
- Unit tests for evaluation logic
Target files:
- `crates/engine/src/evaluate.rs` (new)
- `crates/engine/src/lib.rs` (add module)
- `crates/common/src/types.rs` (add EvalReport types if needed)

### Slice 3 — CLI `evaluate` command + runtime mode tests
Scope:
- `harness evaluate [--golden-dir <path>] [--json]` command
- Output: per-fixture pass/fail, overall hit rate, summary
- Exit code 0 on all pass, 1 on any failure
- Runtime mode enforcement tests (public demo blocks uploads, production blocks, local allows)
Target files:
- `crates/cli/src/commands/evaluate.rs` (new)
- `crates/cli/src/commands/mod.rs` (add module)
- `crates/cli/src/main.rs` (add command)
- `tests/runtime_mode.rs` (new integration test)

---

## Suggested acceptance checks for Slice 1 (first implementation slice)

- Golden corpus documents are valid and ingestible by the existing pipeline
- All 8 fixtures have valid expected outcomes
- GoldenProvider returns deterministic vectors
- `cargo test --test golden` passes with all 8 fixtures green
- Hit rate metric computed correctly (8/8 or documented failures)
