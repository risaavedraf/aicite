# Apply Progress: Error Remediation — PR-1

**Change ID:** error-remediation
**PR:** PR-1 (Data Integrity)
**Date:** 2026-06-02
**Status:** complete

---

## Completed Tasks

- [x] **Task 1.1:** Add `char_len` and `char_truncate` helpers to `common/src/lib.rs`
- [x] **Task 1.2:** Tests for `char_len` and `char_truncate` (9 tests in common crate)
- [x] **Task 1.3:** Fix `heading_parser.rs` UTF-8 byte/char confusion (`line.len()` → `line.chars().count()`)
- [x] **Task 1.4:** Add UTF-8 tests for `heading_parser` (emoji + accented tests)
- [x] **Task 1.5:** Fix `validator.rs` byte-slice truncation (`trimmed[..255]` → `chars().take(255)`)
- [x] **Task 1.6:** Add UTF-8 tests for `validator` (emoji + CJK truncation tests)
- [x] **Task 1.7:** Fix `extractor.rs` UTF-8 byte/char confusion (`content.len()` → `content.chars().count()`)
- [x] **Task 1.8:** Add UTF-8 test for `extractor` total_chars (Japanese + accented tests)
- [x] **Task 1.9:** Fix `sentence_chunker.rs` UTF-8 byte/char confusion (3 locations: min_chars check + 2x offset_end)
- [x] **Task 1.10:** Add UTF-8 tests for `sentence_chunker` (multibyte offsets + merge threshold)
- [x] **Task 1.11:** Add FK enforcement pragma to `storage/src/lib.rs` (both `open()` and `open_memory()`)
- [x] **Task 1.12:** Add FK enforcement tests (pragma returns 1, orphan rejection, valid insert)
- [x] **Task 1.13:** Full quality gate — all three commands pass

---

## Files Changed

| File | Theme | Changes |
|------|-------|---------|
| `crates/common/src/lib.rs` | 1 (UTF-8) | Added `char_len()`, `char_truncate()`, and 9 unit tests |
| `crates/graph/src/heading_parser.rs` | 1 (UTF-8) | `line.len()` → `line.chars().count()` (2 sites); fixed code block double-increment bug; added 2 UTF-8 tests |
| `crates/ingest/src/validator.rs` | 1 (UTF-8) | `trimmed.len()` → `trimmed.chars().count()` + `chars().take(255)`; added 2 multibyte truncation tests |
| `crates/ingest/src/extractor.rs` | 1 (UTF-8) | `content.len()` → `content.chars().count()` (2 sites: txt + pdf); added 2 non-ASCII char count tests |
| `crates/ingest/src/sentence_chunker.rs` | 1 (UTF-8) | `current_text.len()` → `current_text.chars().count()` (3 sites); added 2 multibyte offset/merge tests |
| `crates/storage/src/lib.rs` | 2 (FK) | Added `PRAGMA foreign_keys = ON` in `open()` and `open_memory()`; added 3 FK enforcement tests |

---

## Test Commands Run

```
cargo test               → 268 passed, 0 failed, 12 ignored
cargo clippy -- -D warnings → 0 warnings
cargo fmt --check         → clean
```

---

## Deviations from Design

### 1. heading_parser code block double-increment (discovered during implementation)

**What:** The original `heading_parser.rs` had `char_offset += line.len() + 1` inside both the code-block `if` block (with `continue`) AND at the bottom of the loop. For code-block lines, this caused double-counting.

**Why it was invisible:** All existing tests use ASCII text where `len == chars().count()`, and the test `test_headings_in_code_blocks_ignored` only checked heading titles, not offsets. The double-count only affected offset tracking for lines inside code blocks.

**Fix:** Removed the `continue` and the first `char_offset` increment inside the code-block block. Used `else if` to make the code-block toggle and heading detection mutually exclusive. Single `char_offset +=` at the end of each loop iteration.

**Risk:** Low — the fix is straightforward and all existing tests pass unchanged.

### 2. Task 1.2 mixed-string character count

**What:** The spec said `char_len("Hello 日本語 🎉 café") == 18`, but the actual count is 16 (the spec double-counted the emoji as 2 chars).

**Fix:** Used the correct value (16) in the test assertion.

### 3. Task 1.4 / 1.10 offset assertions

**What:** The task specs had estimated offset values based on incorrect character counts for emoji strings (e.g., `## 🎉` is 4 chars, not 5). Used debug verification to get exact values.

**Fix:** Used precise character counts verified by runtime debug output.

---

## Remaining Tasks

PR-1 is complete. PR-2 and PR-3 tasks are out of scope for this change.

---

## PR Boundary

**PR-1:** Data Integrity (~274 insertions, ~108 deletions across 6 files)
- Theme 1 (UTF-8): 5 production files modified + tests
- Theme 2 (FK): 1 file modified + tests
- Well under 400-line budget

---

## Discovery: Pre-existing Bug Fixed

**heading_parser.rs double-increment of `char_offset` for code-block lines.** This was not a UTF-8 bug — it was an offset accounting error that affected all users equally. The UTF-8 fix made it visible during testing. Fixed inline as it was trivially related to the same code being modified.

---

# Apply Progress: Error Remediation — PR-2

**Change ID:** error-remediation
**PR:** PR-2 (Security + Onboarding)
**Date:** 2026-06-02
**Status:** complete

---

## Completed Tasks

- [x] **Task 2.1:** Add empty API key validation to `GeminiProvider::new` (early return with `ConfigError`)
- [x] **Task 2.2:** Add empty API key validation to `OpenAICompatibleProvider::new` (early return with `ConfigError`)
- [x] **Task 2.3:** Add empty key tests for both providers (`test_provider_rejects_empty_key` in each)
- [x] **Task 2.4:** Replace `.unwrap_or_default()` with `.ok_or_else()` in CLI `create_provider`
- [x] **Task 2.5:** Wire `check_ingest_allowed` guard into CLI ingest command (before queued/direct paths)
- [x] **Task 2.6:** Add guard integration tests in `runtime_guard.rs` (3 tests: LocalDemo OK, Production blocked, PublicDemo blocked)
- [x] **Task 2.7:** Fix rate limit composite key (`provider_id` → `provider_id:model_id`)
- [x] **Task 2.8:** Add rate limit key test (`test_rate_limit_key_includes_model_id`)
- [x] **Task 2.9:** Full quality gate — all three commands pass

---

## Files Changed

| File | Theme | Changes |
|------|-------|---------|
| `crates/providers/src/gemini.rs` | 4 (API Key) | Added `api_key.is_empty()` check + `test_provider_rejects_empty_key` test |
| `crates/providers/src/openai.rs` | 4 (API Key) | Added `api_key.is_empty()` check + `test_provider_rejects_empty_key` test |
| `crates/cli/src/commands/mod.rs` | 4 (API Key) | `.unwrap_or_default()` → `.ok_or_else(ConfigError)` in `create_provider` |
| `crates/cli/src/commands/ingest.rs` | 3 (Guard) | Added `check_ingest_allowed` guard call before queued/direct paths |
| `crates/engine/src/runtime_guard.rs` | 3 (Guard) | Added 3 guard tests (LocalDemo, Production, PublicPackagedDemo) |
| `crates/engine/src/retrieve.rs` | 5 (Rate Limit) | `rate_limit_key` changed to composite `provider_id:model_id`; made `pub(crate)`; added test |

---

## Test Commands Run

```
cargo test               → 285 passed, 0 failed, 13 ignored
cargo clippy -- -D warnings → 0 warnings
cargo fmt --check         → clean
```

---

## Deviations from Design

None. All PR-2 tasks implemented exactly as specified.

---

## Remaining Tasks

PR-2 is complete. PR-3 tasks are out of scope for this change.

---

## PR Boundary

**PR-2:** Security + Onboarding (~35 insertions, ~2 deletions across 6 files)
- Theme 4 (API Key): 3 production files modified + 2 tests
- Theme 3 (Guard): 1 production file modified + 3 tests
- Theme 5 (Rate Limit): 1 production file modified + 1 test
- Well under 400-line budget

---

# Apply Progress: Error Remediation — PR-3

**Change ID:** error-remediation
**PR:** PR-3 (Config + Defensive + Robustness)
**Date:** 2026-06-02
**Status:** complete

---

## Completed Tasks

- [x] **Task 3.1:** Add `timeout_secs` parameter to provider constructors (`GeminiProvider::new`, `OpenAICompatibleProvider::new`)
- [x] **Task 3.2:** Wire `config.ingest.embedding_timeout_secs` to provider creation in CLI `create_provider()`
- [x] **Task 3.3:** Consolidate config field names — removed `min_chunk_size_chars`, consolidated to `min_chunk_chars`, changed `default_max_chunk_chars()` from 200 to 1500
- [x] **Task 3.4:** Add config consolidation tests (`test_default_max_chunk_chars_is_1500`, `test_env_embedding_timeout_overridden`)
- [x] **Task 3.5:** Fix silenced `.ok()` in `snapshots.rs` → `.optional().map_err(storage_err)?`
- [x] **Task 3.6:** Add test for silenced snapshot error (`test_activate_snapshot_returns_none_when_no_pointer`)
- [x] **Task 3.7:** Fix silenced cleanup errors in `engine/ingest.rs` — `let _ =` → `if let Err(e) = ... { eprintln!() }`
- [x] **Task 3.8:** Fix integer cast safety in `storage/src/util.rs` — `as u32` → `u32::try_from()` with error mapping (4 locations)
- [x] **Task 3.9:** Fix integer cast safety in `storage/src/embeddings.rs` — `as u32` → `u32::try_from()` (8 locations across 2 functions)
- [x] **Task 3.10:** Add integer cast safety test (`test_row_to_chunk_valid_index`)
- [x] **Task 3.11:** Add `CommandContext::provider()` helper method returning `Result<&dyn EmbeddingProvider, CiteError>`
- [x] **Task 3.12:** Replace `.unwrap()` with `.provider()` in `context.rs`, `ingest.rs`, `retrieve.rs` (3 sites)
- [x] **Task 3.13:** Fix `hierarchy.rs` duplicate heading assignment — sequential heading consumption with cursor
- [x] **Task 3.14:** Add duplicate heading test (`test_duplicate_h2_headings_assigned_correctly`)
- [x] **Task 3.15:** Verified indented code fence detection — false alarm; `line.trim()` already handles leading whitespace. Added test `test_indented_code_fence_toggled` confirming correct behavior.
- [x] **Task 3.16:** Fix `evaluate.rs` dead `json` parameter — `_args` → `args`, `json` → `_json`, uses `args.json`
- [x] **Task 3.17:** Add `PartialEq` derive to `CiteError` + test `test_cite_error_partial_eq`
- [x] **Task 3.18:** Remove unused `tokio` and `tracing` deps from `providers/Cargo.toml`
- [x] **Task 3.19:** Full quality gate — all three commands pass

---

## Files Changed

| File | Theme | Changes |
|------|-------|--------|
| `crates/providers/src/gemini.rs` | 6 (Config) | Added `timeout_secs: u64` param to `new()`; updated 4 test call sites |
| `crates/providers/src/openai.rs` | 6 (Config) | Added `timeout_secs: u64` param to `new()`; updated 8 test call sites |
| `crates/cli/src/commands/mod.rs` | 6,9 (Config,Unwrap) | Wired `embedding_timeout_secs` to providers; added `CommandContext::provider()` method |
| `crates/cli/src/commands/setup.rs` | 6 (Config) | Updated 2 provider constructor calls with timeout param |
| `crates/cli/src/commands/context.rs` | 9 (Unwrap) | Replaced `.unwrap()` with `ctx.provider()` + error handling |
| `crates/cli/src/commands/ingest.rs` | 9 (Unwrap) | Replaced `.unwrap()` with `ctx.provider()` + error handling |
| `crates/cli/src/commands/retrieve.rs` | 9 (Unwrap) | Replaced `.unwrap()` with `ctx.provider()` + error handling |
| `crates/cli/src/commands/evaluate.rs` | 11 (Misc) | Fixed dead `json` param: uses `args.json` instead |
| `crates/config/src/lib.rs` | 6 (Config) | Removed `min_chunk_size_chars` field; changed `default_max_chunk_chars()` to 1500; added 2 tests |
| `crates/engine/src/ingest.rs` | 6,7 (Config,Silenced) | `config.min_chunk_size_chars` → `config.min_chunk_chars`; logged cleanup errors |
| `crates/ingest/src/lib.rs` | 6 (Config) | `config.min_chunk_size_chars` → `config.min_chunk_chars` |
| `crates/ingest/src/chunker.rs` | 6 (Config) | Renamed param `min_chunk_size_chars` → `min_chunk_chars`; fixed `trimmed.len()` → `trimmed.chars().count()` |
| `crates/ingest/tests/ingest_e2e.rs` | 6 (Config) | Updated 2 `config.min_chunk_size_chars` references |
| `crates/storage/src/snapshots.rs` | 7 (Silenced) | `.ok()` → `.optional().map_err(storage_err)?`; added 1 test |
| `crates/storage/src/util.rs` | 8 (Casts) | `as u32` → `u32::try_from()` with error mapping (4 sites) |
| `crates/storage/src/embeddings.rs` | 8 (Casts) | `as u32` → `u32::try_from()` with error mapping (8 sites) |
| `crates/storage/src/lib.rs` | 8 (Casts) | Added `test_row_to_chunk_valid_index` test |
| `crates/graph/src/hierarchy.rs` | 10 (Graph) | Sequential heading consumption; added duplicate heading test |
| `crates/graph/src/heading_parser.rs` | 10 (Graph) | Added `test_indented_code_fence_toggled` (verified false alarm) |
| `crates/common/src/error.rs` | 11 (Misc) | Added `PartialEq` derive + test |
| `crates/providers/Cargo.toml` | 11 (Misc) | Removed `tokio` and `tracing` unused dependencies |

---

## Test Commands Run

```
cargo test               → 308 passed, 0 failed, 12 ignored
cargo clippy -- -D warnings → 0 warnings
cargo fmt --check         → clean
```

---

## Deviations from Design

### 1. Task 3.15 indented code fence (false alarm confirmed)

**What:** The spec suggested changing `trimmed.starts_with("```")` to `line.trim_start().starts_with("```")` to handle indented code fences.

**Why it was unnecessary:** `trimmed` is already `line.trim()`, which strips ALL leading whitespace. So `    \`\`\`\`rust` becomes `` ```rust `` after trimming, and `trimmed.starts_with("```")` already works.

**Verification:** Added `test_indented_code_fence_toggled` test with 4-space indented fences. Test passes, confirming no code change needed.

### 2. Task 3.3 removed `min_chunk_size_chars` field entirely

**What:** The design said to rename `min_chunk_size_chars` to `min_chunk_chars`, but `min_chunk_chars` already existed as a separate field (for sentence chunker). The consolidation removed `min_chunk_size_chars` entirely, keeping `min_chunk_chars` as the single canonical field.

**Impact:** `CITE_MIN_CHUNK_SIZE_CHARS` env var is no longer supported. Users must use `CITE_MIN_CHUNK_CHARS`. The fixed-size chunker (`chunker::chunk_text`) now receives `config.min_chunk_chars` (default 30) instead of the old `config.min_chunk_size_chars` (default 100). This is a smaller threshold — chunk filtering will be slightly more permissive.

**Risk:** Low — the old default of 100 was already conservative, and 30 is still a reasonable minimum chunk size.

### 3. Additional call site found: `setup.rs`

**What:** The design didn't list `setup.rs` as a file to update, but it had two provider constructor calls that needed the new `timeout_secs` parameter.

**Fix:** Added `30` (default timeout) to both calls in `setup.rs`.

### 4. Chunker parameter rename

**What:** The task said to rename the `IngestConfig` field but didn't mention renaming the `chunker::chunk_text` function parameter from `min_chunk_size_chars` to `min_chunk_chars`. I renamed it for consistency and fixed a latent `.len()` → `.chars().count()` issue in the min-size filtering.

**Risk:** Low — the function signature change is internal (not a public API), and all call sites were already updated.

---

## Remaining Tasks

All PR-3 tasks are complete. No remaining work within this change.

---

## PR Boundary

**PR-3:** Config + Defensive + Robustness
- Theme 6 (Config): 6 files modified + 2 tests
- Theme 7 (Silenced): 2 files modified + 1 test
- Theme 8 (Casts): 3 files modified + 1 test
- Theme 9 (Unwrap): 4 files modified
- Theme 10 (Graph): 2 files modified + 2 tests
- Theme 11 (Misc): 3 files modified + 1 test
- Well under 400-line budget

---

## Discovery: `setup.rs` provider constructor calls

**`setup.rs` had two provider constructor calls** (`GeminiProvider::new` and `OpenAICompatibleProvider::new`) that were not listed in the task spec but needed the new `timeout_secs` parameter. Discovered via compilation error. Fixed by adding default `30` timeout.
