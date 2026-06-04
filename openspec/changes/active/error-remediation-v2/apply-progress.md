# Apply Progress: error-remediation-v2

## Overall Status

**Status:** ✅ ALL 6 PRs APPLIED — verify gate passed
**Branch:** `refactor/error-remediation-v2-waves-1-2`
**Verify:** `cargo test` 297 pass / 0 fail / 13 ignored | `cargo clippy` clean | `cargo fmt` clean

---

## Wave 1 / PR-1 — CLI DRY and Retrieval Validation

**Status:** applied, verify gate passed.
**Commit:** `06692e6`

### Completed tasks

- 1.1 Added focused RED tests for invalid retrieval scope combinations.
- 1.2 Added canonical `exit_for_error` helper in `commands/mod.rs`.
- 1.3 Added shared `validate_retrieval_scope` helper.
- 1.3a Fixed hierarchy-preservation regression.
- 1.4 Replaced setup API-key prompt `unwrap_or_default()` with explicit error handling.
- 1.5 Kept helpers small: one renderer, one retrieval-scope validator.

### Files changed

- `crates/cli/src/commands/mod.rs`
- `crates/cli/src/commands/{search,retrieve,context,setup,read,get,list,refresh,retry}.rs`

### Test evidence

| Step | Command | Result |
|---|---|---|
| GREEN | `cargo test -p cli` | pass: 20 tests |
| VERIFY | `cargo test` | pass |
| VERIFY | `cargo clippy -- -D warnings` | pass |
| VERIFY | `cargo fmt --check` | pass |

---

## Wave 2 / PR-2a — Canonical Golden Fixtures and Evaluation Provider

**Status:** applied, verify gate passed.
**Commit:** `f6c2a3a`

### Completed tasks

- 2a.1 Inspected golden fixture/provider drift.
- 2a.2 Removed duplicated test-only golden provider.
- 2a.3 Added canonical `engine::evaluate::golden_fixtures()`.
- 2a.3a Engine golden integration fixtures now derive from canonical fixtures.

### Files changed

- `crates/cli/src/commands/evaluate.rs`
- `crates/engine/src/evaluate.rs`
- `crates/engine/tests/golden/{fixtures,provider}.rs`
- `crates/engine/tests/golden_test.rs`

### Test evidence

| Step | Command | Result |
|---|---|---|
| GREEN | `cargo test -p engine -p cli` | pass |
| VERIFY | `cargo test` | pass |
| VERIFY | `cargo clippy -- -D warnings` | pass |

---

## Wave 3 / PR-2b — Deterministic Test Infrastructure and Edge Cases

**Status:** applied, verify gate passed.
**Commit:** `46d88ac`

### Completed tasks

- 2b.1 Marked 2 network-dependent provider tests `#[ignore]` with reason strings.
- 2b.2 Added 10 retrieval edge-case tests (cosine_similarity + rank_by_similarity).
- 2b.3 Added 5 config tests (env precedence, invalid env fallback, TOML loading, missing TOML, PartialEq).
- 2b.4 Verified UTF-8 tests from first pass cover M21/L19 (no changes needed).
- 2b.5 Added explanatory comment on ignored retrieval doctest.

### Files changed

- `crates/providers/src/gemini.rs` — `#[ignore]` on network test
- `crates/providers/src/openai.rs` — `#[ignore]` on network test
- `crates/retrieval/src/lib.rs` — 10 new edge-case tests
- `crates/config/src/lib.rs` — 5 new tests + ENV_MUTEX

### Test evidence

| Step | Command | Result |
|---|---|---|
| VERIFY | `cargo test` | 297 pass, 0 fail, 13 ignored |
| VERIFY | `cargo clippy -- -D warnings` | pass |
| VERIFY | `cargo fmt --check` | pass |

---

## Wave 4 / PR-3 — Cast Safety and Type Consistency

**Status:** applied, verify gate passed.
**Commit:** `f09b06f`

### Completed tasks

- 3.1-3.2 Added `i64_to_u32` and `usize_to_u32` checked conversion helpers in `util.rs`.
- 3.3 Replaced 4 casts in `documents.rs`, 1 in `rate_limits.rs`, 8 in `traces.rs`.
- Zero remaining unchecked `as u32` casts in storage outside tests.

### Files changed

- `crates/storage/src/util.rs` — +14 lines (helpers)
- `crates/storage/src/documents.rs` — 4 casts replaced
- `crates/storage/src/rate_limits.rs` — 1 cast replaced
- `crates/storage/src/traces.rs` — 8 casts replaced

### Test evidence

| Step | Command | Result |
|---|---|---|
| VERIFY | `cargo test` | pass |
| VERIFY | `grep -rn "as u32" crates/storage/src/` | 0 matches outside tests |
| VERIFY | `cargo clippy -- -D warnings` | pass |

---

## Wave 5 / PR-4 — Storage and Engine Correctness Cleanup

**Status:** applied, verify gate passed.
**Commit:** `48b0ffc`

### Completed tasks

- 4.1 Added `prune_stale_rate_limits` with best-effort inline pruning.
- 4.2 Extracted `row_to_chunk_embedding` helper, eliminating ~60 lines of duplication.
- 4.3 Changed `decode_vector_blob` from `Option` to `Result` for corrupt blobs.
- 4.4 Fixed usize cast in `engine/refresh.rs` with `try_from`.
- 4.5 Added `From<ChunkEmbeddingRecord> for ScoredChunk`.

### Files changed

- `crates/storage/src/rate_limits.rs` — +90 lines (pruning method + tests)
- `crates/storage/src/embeddings.rs` — net -17 lines (helper extraction + corrupt blob test)
- `crates/engine/src/refresh.rs` — +2 lines (safe usize cast)
- `crates/retrieval/src/lib.rs` — +15 lines (From impl, simplified rank fn)

### Test evidence

| Step | Command | Result |
|---|---|---|
| VERIFY | `cargo test` | pass |
| VERIFY | `cargo clippy -- -D warnings` | pass |

---

## Wave 6 / PR-5 — Dead Code, Placeholders, and Low-Risk Cleanup

**Status:** applied, verify gate passed.
**Commit:** `213ee99`

### Completed tasks

- Removed empty `Engine` struct (no methods, no callers).
- Removed empty `Graph` struct and unused `SemanticLink` type.
- Removed 3 dead `into_compact_*` functions with `#[allow(dead_code)]`.
- Removed unused `tracing` dependency from engine crate.
- `SemanticLinkRow` in storage preserved (different purpose).

### Files changed

- `crates/engine/src/lib.rs` — -3 lines
- `crates/engine/Cargo.toml` — -1 line
- `crates/graph/src/lib.rs` — -14 lines
- `crates/graph/src/types.rs` — -10 lines
- `crates/cli/src/output.rs` — -60 lines

### Test evidence

| Step | Command | Result |
|---|---|---|
| VERIFY | `cargo test` | pass |
| VERIFY | `cargo clippy -- -D warnings` | pass |

---

## Wave 7 / PR-6 — Naming, Documentation, Setup/Health UX

**Status:** applied, verify gate passed.
**Commit:** `c329610`

### Completed tasks

- 6.1 Setup now saves tested model to config (was hardcoded, not persisted).
- 6.2 Added doc comment on health execute documenting network behavior.
- 6.3 Improved `CITE_API_KEY` deprecation warning.
- 6.4 Added doc comments to config structs.
- 6.5 Added test documenting invalid env values silently fall back to defaults.

### Files changed

- `crates/cli/src/commands/setup.rs` — model save fix
- `crates/cli/src/commands/health.rs` — doc comment
- `crates/config/src/lib.rs` — deprecation warning, doc comments, new test

### Test evidence

| Step | Command | Result |
|---|---|---|
| VERIFY | `cargo test` | 297 pass, 0 fail, 13 ignored |
| VERIFY | `cargo clippy -- -D warnings` | pass |
| VERIFY | `cargo fmt --check` | pass |

---

## Final Verify Report

| Check | Result |
|-------|--------|
| `cargo test` | ✅ 297 pass, 0 fail, 13 ignored |
| `cargo clippy -- -D warnings` | ✅ clean |
| `cargo fmt --check` | ✅ clean |
| `grep -rn "as u32" crates/storage/src/` | ✅ 0 outside tests |
| Remaining `as u32` in tests | 1 (safe: `window as u32` in test helper) |

### Deferred items

| Item | Reason |
|------|--------|
| C9/M33 newtype migration | ~50 files, separate SDD `id-newtype-migration` |
| Snapshot pointer `updated_at` | No column exists, migration out of scope |
| H7 snapshot activation rollback | Architecture change, separate SDD |
| H19 ScoredChunk full dedup | `From` impl added, full API redesign deferred |

### Commits in chain

```
c329610 fix: setup saves model, improve config docs and deprecation warning
213ee99 chore: remove dead code, unused structs, and stale dependency
48b0ffc fix(storage): rate-limit pruning, shared row mapper, corrupt blob errors
f09b06f fix(storage): replace unchecked as u32 casts with checked helpers
46d88ac test: add deterministic edge-case tests and ignore network tests
06692e6 fix(cli): move tests after command helpers
f6c2a3a refactor(error-remediation): apply v2 remediation waves
```
