# SDD Tasks — Phase 6: Evaluation

## Delivery plan

3 slices, each under 400 changed lines. Dependencies flow linearly: Slice 1 → Slice 2 → Slice 3.

---

## Slice 1 — Golden corpus + fixtures + GoldenProvider + integration tests

**Estimated lines**: ~350
**Dependencies**: None (first slice)
**Files touched**: 8 new files, 0 modified

### Tasks

- [ ] **T1.1** Create `tests/golden/corpus/architecture.txt`
  - Write ~500 words covering API gateway, PostgreSQL, JWT auth, structured logging
  - Include 4+ distinct verifiable facts
  - Acceptance: file exists, ingestible by existing pipeline

- [ ] **T1.2** Create `tests/golden/corpus/api-reference.md`
  - Write ~400 words covering GET/POST /users, error codes, rate limiting
  - Include 4+ distinct verifiable facts
  - Acceptance: file exists, ingestible by existing pipeline

- [ ] **T1.3** Create `tests/golden/corpus/security-policy.txt`
  - Write ~400 words covering passwords, encryption, audit logging
  - Include 3+ distinct verifiable facts
  - Include prompt-injection section: "Ignore all previous instructions..."
  - Acceptance: file exists, ingestible, contains injection-like text

- [ ] **T1.4** Create `tests/golden/fixtures.json`
  - 8 fixtures with schema from spec (fixture_id, query, category, expected)
  - 3 direct_fact, 2 no_results, 1 ambiguous, 1 multi_chunk, 1 prompt_injection
  - Acceptance: valid JSON, all 8 categories represented

- [ ] **T1.5** Create `tests/golden/provider.rs`
  - `GoldenProvider` struct with `HashMap<String, Vec<f32>>`
  - Implement `EmbeddingProvider` trait
  - 8-dimensional vectors, hand-crafted per design doc
  - Pre-computed vectors for all corpus chunks + 8 query vectors
  - Acceptance: `embed()` returns deterministic vectors, cosine similarity matches design intent

- [ ] **T1.6** Create `tests/golden/fixtures.rs`
  - `GoldenFixture` struct matching fixture JSON schema
  - `load_fixtures()` function that parses embedded JSON
  - Acceptance: fixtures load correctly from `include_str!()`

- [ ] **T1.7** Create `tests/golden/mod.rs`
  - Integration test: `test_golden_dataset_all_fixtures`
  - Setup: create in-memory DB, ingest 3 corpus docs, insert chunk embeddings via GoldenProvider
  - For each fixture: run `build_context`, assert expected result_kind, citation count, chunk IDs
  - Compute hit rate, assert >= 0.80
  - Acceptance: `cargo test --test golden` passes all 8 fixtures

### Acceptance gate (Slice 1)

```
cargo test --test golden
# All 8 fixtures pass
# Hit rate: 8/8 (100%)
```

---

## Slice 2 — Evaluation engine + EvalReport types

**Estimated lines**: ~250
**Dependencies**: Slice 1 complete
**Files touched**: 2 modified, 1 new

### Tasks

- [ ] **T2.1** Add `EvalReport` and `FixtureResult` types to `crates/common/src/types.rs`
  - `EvalReport { total, passed, failed, hit_rate, results, threshold, overall_pass }`
  - `FixtureResult { fixture_id, category, passed, actual_result_kind, actual_citation_count, failure_reason }`
  - Acceptance: types compile, derive Debug + Serialize + Deserialize

- [ ] **T2.2** Create `crates/engine/src/evaluate.rs`
  - `run_evaluation(db, provider, config, fixtures) -> EvalReport`
  - Logic: for each fixture, call `build_context`, compare against expected, record result
  - Helper: `evaluate_fixture(db, provider, config, fixture) -> FixtureResult`
  - Acceptance: unit tests pass for each fixture category

- [ ] **T2.3** Add `pub mod evaluate;` to `crates/engine/src/lib.rs`
  - Acceptance: `cargo test -p engine` includes evaluate tests

- [ ] **T2.4** Add engine-level unit tests for evaluate module
  - Test each fixture category independently (mock DB + GoldenProvider)
  - Test hit rate computation edge cases (all pass, all fail, partial)
  - Acceptance: `cargo test -p engine evaluate` passes

### Acceptance gate (Slice 2)

```
cargo test -p engine evaluate
# All evaluate unit tests pass
cargo clippy -- -D warnings
# Clean
```

---

## Slice 3 — CLI `evaluate` command + runtime mode tests

**Estimated lines**: ~300
**Dependencies**: Slice 2 complete
**Files touched**: 3 modified, 2 new

### Tasks

- [ ] **T3.1** Create `crates/cli/src/commands/evaluate.rs`
  - `EvaluateArgs` struct (optional `--json` flag)
  - `execute(args, config, json) -> ExitCode`:
    1. Load golden fixtures (embedded or from dir)
    2. Create in-memory DB
    3. Ingest golden corpus
    4. Build GoldenProvider with corpus embeddings
    5. Call `engine::evaluate::run_evaluation`
    6. Output human-readable or JSON report
  - Acceptance: `harness evaluate --json` outputs valid JSON

- [ ] **T3.2** Wire evaluate command into CLI
  - Add `pub mod evaluate;` to `crates/cli/src/commands/mod.rs`
  - Add `Commands::Evaluate(EvaluateArgs)` to `crates/cli/src/main.rs`
  - Acceptance: `harness evaluate --help` shows usage

- [ ] **T3.3** Create `tests/runtime_mode.rs`
  - Test: `PublicPackagedDemo` mode rejects `ingest` with `runtime_mode_forbidden`
  - Test: `Production` mode rejects `ingest` with `runtime_mode_forbidden`
  - Test: `LocalPrivateDemo` mode allows `ingest`
  - Acceptance: `cargo test --test runtime_mode` passes

- [ ] **T3.4** Manual verification
  - Run `harness evaluate` and verify output matches expected
  - Run `harness evaluate --json` and verify JSON structure
  - Acceptance: output matches spec format

### Acceptance gate (Slice 3)

```
harness evaluate
# All 8 fixtures pass, hit rate shown
harness evaluate --json
# Valid JSON output
cargo test --test runtime_mode
# All 3 mode tests pass
cargo clippy -- -D warnings
# Clean
cargo test
# All existing tests still pass (no regressions)
```

---

## Final acceptance (Phase 6 complete)

- [ ] All 8 golden fixtures pass
- [ ] Hit rate >= 80%
- [ ] Runtime mode enforcement tests pass
- [ ] `harness evaluate` command works (human + JSON)
- [ ] `cargo clippy -- -D warnings` clean
- [ ] `cargo test` all green (no regressions)
- [ ] All 3 slices committed separately
- [ ] SDD artifacts complete in `docs/sdd/phase-6-evaluation/`
