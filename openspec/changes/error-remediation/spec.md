# Spec: Error Remediation — 11 Themes

**Change ID:** error-remediation
**Date:** 2026-06-02
**Status:** spec
**Source:** Verified against source code by scout agent

---

## PR-1: Data Integrity (~50 lines)

### Theme 1: UTF-8 Bytes-vs-Chars

**Root cause:** `str::len()` returns byte count, not Unicode character count. Used in offset tracking, truncation, and character counting.

**Fixes:**

| # | File | Line(s) | Current | Fix | Errors |
|---|------|---------|---------|-----|--------|
| 1.1 | `crates/common/src/lib.rs` | NEW | (no helpers) | Add `pub fn char_len(s: &str) -> usize` and `pub fn char_truncate(s: &str, max: usize) -> String` | — |
| 1.2 | `crates/graph/src/heading_parser.rs` | 17 | `char_offset += line.len() + 1` | `char_offset += line.chars().count() + 1` | C3 |
| 1.3 | `crates/graph/src/heading_parser.rs` | 37 | `char_offset += line.len() + 1` | `char_offset += line.chars().count() + 1` | C3 |
| 1.4 | `crates/ingest/src/validator.rs` | 95-98 | `trimmed[..255].to_string()` | `trimmed.chars().take(255).collect::<String>()` | C2 |
| 1.5 | `crates/ingest/src/extractor.rs` | 37 | `content.len()` | `content.chars().count()` | C5 |
| 1.6 | `crates/ingest/src/extractor.rs` | 75 | `text.len()` (in `extract_pdf_text`) | `text.chars().count()` | C5 |
| 1.7 | `crates/ingest/src/sentence_chunker.rs` | 42 | `current_text.len() < min_chars` | `current_text.chars().count() < min_chars` | C4, H10, H12 |
| 1.8 | `crates/ingest/src/sentence_chunker.rs` | 47 | `current_text.len()` (offset_end) | `current_text.chars().count()` | C4, H11 |
| 1.9 | `crates/ingest/src/sentence_chunker.rs` | 57 | `current_text.len()` (offset_end flush) | `current_text.chars().count()` | C4, H11 |

**Acceptance criteria:**
- [ ] `cargo test` passes
- [ ] New test: `heading_parser` with emoji/CJK input produces correct `char_offset`
- [ ] New test: `validator` with multi-byte filename >255 chars truncates without panic
- [ ] New test: `extractor` with non-ASCII text reports correct `total_chars`
- [ ] New test: `sentence_chunker` with multi-byte text produces correct offsets

**Estimated lines:** ~15 production + ~30 tests = ~45

---

### Theme 2: FK Enforcement

**Root cause:** SQLite disables FK enforcement by default. `PRAGMA foreign_keys=ON` is never executed.

**Fixes:**

| # | File | Line(s) | Current | Fix | Errors |
|---|------|---------|---------|-----|--------|
| 2.1 | `crates/storage/src/lib.rs` | after 41 | (no FK pragma) | Add `conn.pragma_update(None, "foreign_keys", "ON")` with error mapping | C1 |

**Acceptance criteria:**
- [ ] `PRAGMA foreign_keys` returns 1 after `Database::open()`
- [ ] Insert orphan chunk (no parent doc) fails with FK violation
- [ ] `cargo test` passes (existing tests have proper parent rows)

**Estimated lines:** ~5 production + ~10 tests = ~15

---

## PR-2: Security + Onboarding (~40 lines)

### Theme 4: Empty API Key Validation

**Fixes:**

| # | File | Line(s) | Current | Fix | Errors |
|---|------|---------|---------|-----|--------|
| 4.1 | `crates/cli/src/commands/mod.rs` | 94 | `.unwrap_or_default()` | Replace with `.ok_or(CiteError::ConfigError { message: "No API key configured. Set CITE_API_KEY or run `cite setup`." })?` | C7 |
| 4.2 | `crates/providers/src/gemini.rs` | start of `new()` | (no validation) | Add `if api_key.is_empty() { return Err(CiteError::ConfigError { ... }) }` | C7 |
| 4.3 | `crates/providers/src/openai.rs` | start of `new()` | (no validation) | Add `if api_key.is_empty() { return Err(CiteError::ConfigError { ... }) }` | C7 |

**Acceptance criteria:**
- [ ] `cite ingest` without API key → clear error message mentioning `CITE_API_KEY`
- [ ] `cite retrieve` without API key → same clear error
- [ ] Provider with empty key → `CiteError::ConfigError`
- [ ] `cargo test` passes

**Estimated lines:** ~15

---

### Theme 3: Production Mode Guard

**Fixes:**

| # | File | Line(s) | Current | Fix | Errors |
|---|------|---------|---------|-----|--------|
| 3.1 | `crates/engine/src/ingest.rs` | start of `ingest()` (line ~48) | (no guard call) | Add `runtime_guard::check_ingest_allowed(&config.runtime.mode)?;` | C6 |
| 3.2 | `crates/engine/src/ingest.rs` | start of `ingest_next()` (line ~75) | (no guard call) | Add `runtime_guard::check_ingest_allowed(&config.runtime.mode)?;` | C6 |
| 3.3 | `crates/engine/src/runtime_guard.rs` | 23-34 | `check_ingest_allowed()` defined, never called | No change needed — function is correct, just needs wiring | C6 |

**Note:** Do NOT rename `production_mode` parameter in this PR — that's a refactor for PR-3. This PR only wires the guard.

**Acceptance criteria:**
- [ ] `cite ingest` in Production mode → `CiteError::RuntimeModeForbidden`
- [ ] `cite ingest` in LocalPrivateDemo → proceeds normally
- [ ] Existing `runtime_guard` tests still pass
- [ ] `cargo test` passes

**Estimated lines:** ~10

---

### Theme 5: Rate Limit Composite Key

**Fixes:**

| # | File | Line(s) | Current | Fix | Errors |
|---|------|---------|---------|-----|--------|
| 5.1 | `crates/engine/src/retrieve.rs` | 278-280 | `provider.provider_id().to_string()` | `format!("{}:{}", provider.provider_id(), provider.model_id())` | C8 |

**Note:** Analysis suggested 4-part composite key `(mode, corpus_id, provider_id, scope)`. Verified source shows the function only receives `provider`. The minimal correct fix is `provider_id:model_id` — this distinguishes same-provider different-model rate limits. Full composite key requires changing the function signature (deferred).

**Acceptance criteria:**
- [ ] Different models on same provider get separate rate limit buckets
- [ ] `cargo test` passes

**Estimated lines:** ~5

---

## PR-3: Config + Defensive + Robustness (~195 lines)

### Theme 6: Config-Disconnect

**Fixes:**

| # | File | Line(s) | Current | Fix | Errors |
|---|------|---------|---------|-----|--------|
| 6.1 | `crates/providers/src/gemini.rs` | 28 | `Duration::from_secs(30)` | Accept `timeout_secs: u64` param, use `Duration::from_secs(timeout_secs)` | H15 |
| 6.2 | `crates/providers/src/openai.rs` | 34 | `Duration::from_secs(30)` | Accept `timeout_secs: u64` param, use `Duration::from_secs(timeout_secs)` | H15 |
| 6.3 | `crates/cli/src/commands/mod.rs` | provider creation | (ignores timeout) | Pass `config.ingest.embedding_timeout_secs` to provider constructors | H15 |
| 6.4 | `crates/config/src/lib.rs` | 93, 108 | `min_chunk_size_chars` + `min_chunk_chars` | Consolidate to single `min_chunk_chars` field. Update all references. | H13 |
| 6.5 | `crates/config/src/lib.rs` | 111 | `max_chunk_chars: 200` | Set default to `1500` (must be > `chunk_size_chars: 1000`) | H13 |

**Acceptance criteria:**
- [ ] `CITE_EMBEDDING_TIMEOUT=60` actually sets 60s timeout
- [ ] No confusing duplicate chunk config fields
- [ ] `max_chunk_chars > chunk_size_chars` in defaults
- [ ] `cargo test` passes

**Estimated lines:** ~50

---

### Theme 7: Silenced Error Elimination

**Fixes:**

| # | File | Line(s) | Current | Fix | Errors |
|---|------|---------|---------|-----|--------|
| 7.1 | `crates/storage/src/snapshots.rs` | 68-73 | `.ok()` | `.optional().map_err(storage_err)?` — only `QueryReturnedNoRows` → `None` | H17 |
| 7.2 | `crates/engine/src/ingest.rs` | 191-193 | `let _ = cleanup_partial(...)` | Log warning on failure: `if let Err(e) = cleanup_partial(...) { eprintln!(...) }` | H6 |
| 7.3 | `crates/engine/src/ingest.rs` | 246-247 | `let _ = db.delete_embeddings/chunks(...)` | Log warnings on failure | H6 |

**Acceptance criteria:**
- [ ] DB errors in `activate_snapshot` surface as `CiteError::StorageError`
- [ ] Cleanup failures logged to stderr
- [ ] `cargo test` passes

**Estimated lines:** ~20

---

### Theme 8: Integer Cast Safety

**Fixes:**

| # | File | Line(s) | Current | Fix | Errors |
|---|------|---------|---------|-----|--------|
| 8.1 | `crates/storage/src/util.rs` | 37, 42, 47, 51 | `as u32` | `u32::try_from(v).map_err(storage_err)?` | H18 |
| 8.2 | `crates/storage/src/embeddings.rs` | 144, 148, 152, 155 | `as u32` | `u32::try_from(v).map_err(storage_err)?` | H18 |
| 8.3 | `crates/storage/src/embeddings.rs` | ~209, 213, 217, 220 | `as u32` (in `list_ready_chunk_embeddings`) | `u32::try_from(v).map_err(storage_err)?` | H18 |

**Acceptance criteria:**
- [ ] Values > `u32::MAX` produce `StorageError` instead of silent truncation
- [ ] `cargo test` passes

**Estimated lines:** ~25

---

### Theme 9: Provider Unwrap Consistency

**Fixes:**

| # | File | Line(s) | Current | Fix | Errors |
|---|------|---------|---------|-----|--------|
| 9.1 | `crates/cli/src/commands/mod.rs` | NEW | (no helper) | Add `pub fn provider(&self) -> Result<&dyn EmbeddingProvider, CiteError>` on `CommandContext` | H3 |
| 9.2 | `crates/cli/src/commands/context.rs` | 50 | `.unwrap()` | `.provider()?` | H3 |
| 9.3 | `crates/cli/src/commands/ingest.rs` | 66 | `.unwrap()` | `.provider()?` | H3 |
| 9.4 | `crates/cli/src/commands/retrieve.rs` | 80 | `.unwrap()` | `.provider()?` | H3 |

**Acceptance criteria:**
- [ ] No `.unwrap()` on provider access in any command
- [ ] Provider creation failure → graceful `CiteError`, not panic
- [ ] `cargo test` passes

**Estimated lines:** ~15

---

### Theme 10: Graph Parsing Robustness

**Fixes:**

| # | File | Line(s) | Current | Fix | Errors |
|---|------|---------|---------|-----|--------|
| 10.1 | `crates/graph/src/hierarchy.rs` | 130-146 | `.find()` matches first heading by title | Track heading iterator position; consume headings sequentially instead of searching from start | H8 |
| 10.2 | `crates/graph/src/heading_parser.rs` | 14-16 | `starts_with("```")` | Add `trim_start()` before check: `trimmed.starts_with("```")` (already trimmed — verify). Handle 4+ backtick fences. | H9 |

**Acceptance criteria:**
- [ ] Document with duplicate `## Overview` headings → chunks assigned to correct sections
- [ ] Indented code fences → correctly toggled
- [ ] `cargo test` passes

**Estimated lines:** ~30

---

### Theme 11: Misc High-Tier Fixes

**Fixes:**

| # | File | Line(s) | Current | Fix | Errors |
|---|------|---------|---------|-----|--------|
| 11.1 | `crates/cli/src/commands/evaluate.rs` | 250 | `execute(_args, _config, json: bool)` | Use `args.json` instead of separate `json` param. Remove dead param. | H1 |
| 11.2 | `crates/common/src/error.rs` | 7 | `#[derive(Debug, thiserror::Error)]` | Add `PartialEq`: `#[derive(Debug, PartialEq, thiserror::Error)]` | C11 |
| 11.3 | `crates/providers/Cargo.toml` | 8-9 | `tokio` + `tracing` deps | Remove both lines | H16 |
| 11.4 | `crates/retrieval/src/lib.rs` | 40-64 | `ScoredChunk` duplicates fields | Embed `ChunkEmbeddingRecord` or add `From` impl (evaluate scope — may defer) | H19 |

**Acceptance criteria:**
- [ ] `cite evaluate --json` produces JSON output
- [ ] `CiteError` derives `PartialEq`
- [ ] `providers` compiles without tokio/tracing
- [ ] `cargo test` passes
- [ ] `cargo clippy -- -D warnings` passes

**Estimated lines:** ~20

---

## Execution Order

```
PR-1:
  1.1 (common helpers) → 1.2-1.3 (graph) → 1.4-1.9 (ingest) → tests
  2.1 (storage FK) → tests

PR-2:
  4.1-4.3 (API key) → tests
  3.1-3.3 (guard) → tests
  5.1 (rate limit) → tests

PR-3:
  6.1-6.5 (config) → tests
  7.1-7.3 (silenced errors) → tests
  8.1-8.3 (integer casts) → tests
  9.1-9.4 (provider unwrap) → tests
  10.1-10.2 (graph robustness) → tests
  11.1-11.4 (misc) → tests
```

## Cross-Theme Dependencies

```
Theme 1 (UTF-8) must complete before Theme 10 (graph robustness)
  — cursor positions in hierarchy.rs must be char-based

Theme 6 (config field rename) should complete before Theme 1 (sentence_chunker)
  — fix chunker against correct field names
  BUT: Theme 1 is in PR-1, Theme 6 in PR-3
  SOLUTION: In PR-1, fix sentence_chunker using the CURRENT field names
            In PR-3, rename fields and update references

No other cross-PR dependencies.
```

## Test Strategy

- Each theme gets at least 1 targeted test for the specific bug being fixed
- UTF-8 tests use real multi-byte fixtures: `"日本語"`, `"élégant"`, `"🎉🎊"`
- FK test uses in-memory SQLite with intentional FK violation
- API key test verifies error message content
- All tests run via `cargo test` (project-wide)
- Lint via `cargo clippy -- -D warnings`
- Format via `cargo fmt --check`
