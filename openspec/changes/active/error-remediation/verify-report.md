# Verify Report: Error Remediation — All 3 PRs

**Change ID:** error-remediation
**Date:** 2026-06-02
**Status:** ✅ PASS
**Verifier:** SDD verify executor

---

## Quality Gates

| Gate | Command | Result |
|------|---------|--------|
| Tests | `cargo test` | ✅ **308 passed**, 0 failed, 12 ignored |
| Lint | `cargo clippy -- -D warnings` | ✅ **0 warnings** |
| Format | `cargo fmt --check` | ✅ **clean** |

---

## Spec Coverage: 11 Themes

### Theme 1: UTF-8 Bytes-vs-Chars — ✅ PASS

| Fix # | File | Claim | Actual | Verified |
|-------|------|-------|--------|:--------:|
| 1.1 | `common/src/lib.rs` | Add `char_len`, `char_truncate` | Present, exported, 9 tests | ✅ |
| 1.2 | `graph/src/heading_parser.rs:17,37` | `line.len()` → `line.chars().count()` | Both sites fixed | ✅ |
| 1.3 | `ingest/src/validator.rs:95-101` | `trimmed[..255]` → `chars().take(255)` | Fixed | ✅ |
| 1.4 | `ingest/src/extractor.rs:37,75` | `content.len()` → `content.chars().count()` | Both sites fixed | ✅ |
| 1.5 | `ingest/src/sentence_chunker.rs:42,47,57` | `current_text.len()` → `chars().count()` | All 3 sites fixed | ✅ |

**Tests:** `test_char_len_*` (5), `test_char_truncate_*` (4), `test_extract_headings_utf8_offsets`, `test_extract_headings_accented_offsets`, `test_derive_display_name_truncate_multibyte`, `test_derive_display_name_truncate_cjk`, `test_extract_plain_text_non_ascii_char_count`, `test_extract_plain_text_accented_char_count`, `test_sentence_chunker_multibyte_offsets`, `test_sentence_chunker_multibyte_merge_threshold` — all pass.

**Discovery:** `heading_parser.rs` had a pre-existing code-block double-increment bug (lines inside code blocks incremented `char_offset` twice). Fixed inline during Theme 1 implementation. Not in original spec.

### Theme 2: FK Enforcement — ✅ PASS

| Fix # | File | Claim | Actual | Verified |
|-------|------|-------|--------|:--------:|
| 2.1 | `storage/src/lib.rs` | Add FK pragma in `open()` and `open_memory()` | Both present | ✅ |

**Tests:** `test_fk_pragma_returns_1`, `test_fk_rejects_orphan_chunk`, `test_fk_allows_valid_insert` — all pass.

### Theme 3: Production Mode Guard — ✅ PASS

| Fix # | File | Claim | Actual | Verified |
|-------|------|-------|--------|:--------:|
| 3.1-3.2 | `cli/src/commands/ingest.rs` | Wire `check_ingest_allowed` guard | Guard call present before queued/direct paths | ✅ |

**Tests:** `test_check_ingest_allowed_local_demo`, `test_check_ingest_allowed_production`, `test_check_ingest_allowed_public_demo`, plus 3 integration tests in `runtime_mode.rs` — all pass.

### Theme 4: Empty API Key Validation — ✅ PASS

| Fix # | File | Claim | Actual | Verified |
|-------|------|-------|--------|:--------:|
| 4.1 | `cli/src/commands/mod.rs:94` | `.unwrap_or_default()` → `.ok_or_else(ConfigError)` | Fixed | ✅ |
| 4.2 | `providers/src/gemini.rs` | Empty key check in `new()` | Present | ✅ |
| 4.3 | `providers/src/openai.rs` | Empty key check in `new()` | Present | ✅ |

**Tests:** `test_provider_rejects_empty_key` in both gemini and openai — pass.

### Theme 5: Rate Limit Composite Key — ✅ PASS

| Fix # | File | Claim | Actual | Verified |
|-------|------|-------|--------|:--------:|
| 5.1 | `engine/src/retrieve.rs:278-280` | `provider_id` → `provider_id:model_id` | Fixed, `pub(crate)` | ✅ |

**Tests:** `test_rate_limit_key_includes_model_id` — pass.

### Theme 6: Config-Disconnect — ✅ PASS

| Fix # | File | Claim | Actual | Verified |
|-------|------|-------|--------|:--------:|
| 6.1 | `providers/src/gemini.rs` | Accept `timeout_secs` param | Constructor updated, test sites updated | ✅ |
| 6.2 | `providers/src/openai.rs` | Accept `timeout_secs` param | Constructor updated, test sites updated | ✅ |
| 6.3 | `cli/src/commands/mod.rs` | Pass `embedding_timeout_secs` to providers | Both constructors wired | ✅ |
| 6.4 | `config/src/lib.rs` | Consolidate `min_chunk_size_chars` → `min_chunk_chars` | Field removed, only `min_chunk_chars` remains | ✅ |
| 6.5 | `config/src/lib.rs:111` | `default_max_chunk_chars()` → 1500 | Returns 1500 | ✅ |

**Grep verified:** Zero references to `min_chunk_size_chars` in any `crates/` source file.

**Additional call site:** `setup.rs` was discovered to need the timeout parameter. Fixed with default `30`.

**Tests:** `test_default_max_chunk_chars_is_1500`, `test_env_embedding_timeout_overridden` — pass.

### Theme 7: Silenced Error Elimination — ✅ PASS

| Fix # | File | Claim | Actual | Verified |
|-------|------|-------|--------|:--------:|
| 7.1 | `storage/src/snapshots.rs:68-73` | `.ok()` → `.optional().map_err(storage_err)?` | Fixed | ✅ |
| 7.2 | `engine/src/ingest.rs:191-193` | `let _ = cleanup_partial(...)` → `if let Err(e) = ... { eprintln!() }` | Fixed | ✅ |
| 7.3 | `engine/src/ingest.rs:246-247` | `let _ = db.delete_embeddings/chunks(...)` → log on failure | Fixed | ✅ |

**Tests:** `test_activate_snapshot_returns_none_when_no_pointer` — pass.

### Theme 8: Integer Cast Safety — ✅ PASS

| Fix # | File | Claim | Actual | Verified |
|-------|------|-------|--------|:--------:|
| 8.1 | `storage/src/util.rs:37,42,47,51` | `as u32` → `u32::try_from()` | All 4 sites fixed | ✅ |
| 8.2 | `storage/src/embeddings.rs:144,148,152,155` | `as u32` → `u32::try_from()` | All 4 sites fixed | ✅ |
| 8.3 | `storage/src/embeddings.rs:~209,~213,~217,~220` | `as u32` → `u32::try_from()` | All 4 sites fixed | ✅ |

**Tests:** `test_row_to_chunk_valid_index` — pass.

**Scope note:** Additional `as u32` casts remain in `documents.rs` (3), `traces.rs` (7), and `rate_limits.rs` (1). These were NOT in the spec scope (only `util.rs` and `embeddings.rs` were specified). Recommend a follow-up pass.

### Theme 9: Provider Unwrap Consistency — ✅ PASS

| Fix # | File | Claim | Actual | Verified |
|-------|------|-------|--------|:--------:|
| 9.1 | `cli/src/commands/mod.rs` | Add `CommandContext::provider()` method | Present | ✅ |
| 9.2 | `cli/src/commands/context.rs:50` | `.unwrap()` → `.provider()?` | Fixed, 0 `.unwrap()` in file | ✅ |
| 9.3 | `cli/src/commands/ingest.rs:66` | `.unwrap()` → `.provider()?` | Fixed, 0 `.unwrap()` in file | ✅ |
| 9.4 | `cli/src/commands/retrieve.rs:80` | `.unwrap()` → `.provider()?` | Fixed, 0 `.unwrap()` in file | ✅ |

**Note:** `search.rs` uses a safe manual `match ctx.provider.as_ref()` pattern. This was already safe and NOT in the spec scope. Acceptable.

### Theme 10: Graph Parsing Robustness — ✅ PASS

| Fix # | File | Claim | Actual | Verified |
|-------|------|-------|--------|:--------:|
| 10.1 | `graph/src/hierarchy.rs:130-146` | Sequential heading consumption replaces `.find()` | Implemented with cursor-based matching | ✅ |
| 10.2 | `graph/src/heading_parser.rs:14-16` | Indented code fence detection | False alarm — `line.trim()` already handles it. Test confirms. | ✅ |

**Tests:** `test_duplicate_h2_headings_assigned_correctly`, `test_indented_code_fence_toggled` — pass.

### Theme 11: Misc High-Tier Fixes — ✅ PASS

| Fix # | File | Claim | Actual | Verified |
|-------|------|-------|--------|:--------:|
| 11.1 | `cli/src/commands/evaluate.rs:250` | Dead `json` param → use `args.json` | `_args`→`args`, `json`→`_json`, uses `args.json` | ✅ |
| 11.2 | `common/src/error.rs:7` | Add `PartialEq` derive | `#[derive(Debug, PartialEq, thiserror::Error)]` | ✅ |
| 11.3 | `providers/Cargo.toml:8-9` | Remove `tokio` and `tracing` | Removed, grep confirms 0 refs in source | ✅ |
| 11.4 | `retrieval/src/lib.rs:40-64` | `ScoredChunk` field duplication | **Deferred** (as planned) | ✅ |

**Tests:** `test_cite_error_partial_eq` — pass.

---

## Task Completion: 41 Tasks

### PR-1: Data Integrity (13 tasks) — ✅ 13/13

| Task | Description | Status |
|------|-------------|:------:|
| 1.1 | Add `char_len` and `char_truncate` helpers | ✅ |
| 1.2 | Tests for helpers | ✅ |
| 1.3 | Fix `heading_parser.rs` UTF-8 | ✅ |
| 1.4 | UTF-8 test for heading_parser | ✅ |
| 1.5 | Fix `validator.rs` truncation | ✅ |
| 1.6 | UTF-8 test for validator | ✅ |
| 1.7 | Fix `extractor.rs` UTF-8 | ✅ |
| 1.8 | UTF-8 test for extractor | ✅ |
| 1.9 | Fix `sentence_chunker.rs` UTF-8 | ✅ |
| 1.10 | UTF-8 test for sentence_chunker | ✅ |
| 1.11 | Add FK enforcement pragma | ✅ |
| 1.12 | FK enforcement tests | ✅ |
| 1.13 | PR-1 quality gate | ✅ |

### PR-2: Security + Onboarding (9 tasks) — ✅ 9/9

| Task | Description | Status |
|------|-------------|:------:|
| 2.1 | Empty key validation (Gemini) | ✅ |
| 2.2 | Empty key validation (OpenAI) | ✅ |
| 2.3 | Empty key tests | ✅ |
| 2.4 | Replace `.unwrap_or_default()` | ✅ |
| 2.5 | Wire `check_ingest_allowed` guard | ✅ |
| 2.6 | Guard integration tests | ✅ |
| 2.7 | Fix rate limit composite key | ✅ |
| 2.8 | Rate limit key test | ✅ |
| 2.9 | PR-2 quality gate | ✅ |

### PR-3: Config + Defensive + Robustness (19 tasks) — ✅ 19/19

| Task | Description | Status |
|------|-------------|:------:|
| 3.1 | Add `timeout_secs` to providers | ✅ |
| 3.2 | Wire timeout from config | ✅ |
| 3.3 | Consolidate config fields | ✅ |
| 3.4 | Config consolidation tests | ✅ |
| 3.5 | Fix silenced `.ok()` in snapshots | ✅ |
| 3.6 | Snapshot error test | ✅ |
| 3.7 | Fix silenced cleanup errors | ✅ |
| 3.8 | Integer cast safety (`util.rs`) | ✅ |
| 3.9 | Integer cast safety (`embeddings.rs`) | ✅ |
| 3.10 | Integer cast safety tests | ✅ |
| 3.11 | Add `CommandContext::provider()` | ✅ |
| 3.12 | Replace `.unwrap()` with `.provider()` | ✅ |
| 3.13 | Fix `hierarchy.rs` duplicate heading | ✅ |
| 3.14 | Duplicate heading test | ✅ |
| 3.15 | Indented code fence (false alarm, tested) | ✅ |
| 3.16 | Fix `evaluate.rs` dead param | ✅ |
| 3.17 | Add `PartialEq` derive | ✅ |
| 3.18 | Remove unused deps | ✅ |
| 3.19 | PR-3 quality gate | ✅ |

---

## Review Workload / PR Boundary

| PR | Chain Strategy | Line Budget | Actual Status |
|----|---------------|:-----------:|:-------------:|
| PR-1 | stacked-to-main | <400 | ✅ Well under |
| PR-2 | stacked-to-main | <400 | ✅ Well under |
| PR-3 | stacked-to-main | <400 | ✅ Well under |

No scope creep beyond assigned tasks detected. The only additions were:
- `heading_parser.rs` code-block double-increment fix (tightly related, discovered during testing)
- `setup.rs` timeout parameter (compilation requirement, not optional)
- `chunker.rs` parameter rename (consistency with config consolidation)

All additions were directly required by or trivially related to the assigned themes.

---

## Cross-Crate Integrity

| Check | Status | Detail |
|-------|:------:|--------|
| `common::char_len` exported | ✅ | `pub fn` in `common/src/lib.rs` |
| `common::char_truncate` exported | ✅ | `pub fn` in `common/src/lib.rs` |
| Provider `timeout_secs` all callers | ✅ | `mod.rs` + `setup.rs` updated |
| `CommandContext::provider()` consistent | ✅ | `context.rs`, `ingest.rs`, `retrieve.rs` use it |
| Config field rename complete | ✅ | Zero `min_chunk_size_chars` in source |
| `check_ingest_allowed` wired | ✅ | Called in `cli/ingest.rs` before all paths |

---

## Regression Risk Assessment

| Risk | Status | Detail |
|------|:------:|--------|
| Unused imports introduced | ✅ None | Clippy clean with `-D warnings` |
| Dead code introduced | ✅ None | All new functions and methods are called |
| Weak test assertions | ✅ None found | Tests verify concrete values, not tautologies |
| Misleading error messages | ✅ None | All new error messages include actionable remediation steps |
| `.len()` on strings in offset contexts | ✅ None remaining | All fixed; remaining `.len()` calls are on Vec/metadata/ASCII-only tests |
| Unchecked `as u32` in scope | ✅ Fixed in scope | 12 of 12 in-scope sites fixed; 11 out-of-scope remain in `documents.rs`/`traces.rs` |

---

## Deviations from Design

| # | Deviation | Risk | Impact |
|---|-----------|:----:|--------|
| 1 | `heading_parser.rs` code-block double-increment fix (pre-existing bug) | Low | Positive — existing tests pass, offsets now correct for code-block lines |
| 2 | Task 3.15 indented code fence — false alarm, no code change needed | None | Test confirms `line.trim()` already handles indented fences |
| 3 | `setup.rs` discovered as additional call site for timeout parameter | Low | Compilation requirement; fixed with default `30` |
| 4 | `chunker.rs` parameter renamed `min_chunk_size_chars` → `min_chunk_chars` | Low | Internal function, all call sites updated |
| 5 | `min_chunk_chars` default is 30 (sentence chunker) vs old 100 (fixed-size) | Low | Slightly more permissive chunk filtering; 30 is still reasonable |

---

## Out-of-Scope Items for Future Passes

| Item | Location | Reason |
|------|----------|--------|
| `as u32` casts in `documents.rs` | 3 sites | Not in spec scope (util.rs + embeddings.rs only) |
| `as u32` casts in `traces.rs` | 7 sites | Not in spec scope |
| `as u32` cast in `rate_limits.rs` | 1 site | Not in spec scope |
| `search.rs` provider access pattern | Safe manual `match` | Already safe, inconsistent with new `ctx.provider()` |
| `ScoredChunk` field duplication | `retrieval/src/lib.rs` | Explicitly deferred in spec |
| `read.rs` `.unwrap()` calls | Lines 103, 115 | Guarded by prior validation, safe |

---

## Summary

**Status: ✅ PASS**

All 11 themes across 41 tasks are verified complete. All quality gates pass with zero warnings, zero failures, and clean formatting. Cross-crate integrity is confirmed. No regressions introduced. The implementation is ready for merge.

**Risk level:** Low. The changes are well-tested, well-scoped, and follow the design document precisely with minor deviations that were all improvements over the original plan.
