# Tasks: Error Remediation — 11 Themes, 3 PRs

**Change ID:** error-remediation
**Date:** 2026-06-02
**Status:** tasks
**Inputs:** proposal.md, spec.md, design.md, source-verification.md

---

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~251 (101 prod + 150 tests) |
| 400-line budget risk | Low |
| Chained PRs recommended | Yes |
| Suggested split | PR-1 → PR-2 → PR-3 |
| Delivery strategy | auto-chain |
| Chain strategy | stacked-to-main |

```text
Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: stacked-to-main
400-line budget risk: Low
```

---

## Execution Order

```
PR-1: Theme 1 (UTF-8) → Theme 2 (FK)
PR-2: Theme 4 (API Key) → Theme 3 (Guard) → Theme 5 (Rate Limit)
PR-3: Theme 6 (Config) → Theme 7 (Silenced) → Theme 8 (Casts) → Theme 9 (Unwrap) → Theme 10 (Graph) → Theme 11 (Misc)
```

Cross-PR dependency: Theme 1 (PR-1) must be merged before Theme 10 (PR-3) so that `hierarchy.rs` cursor positions are already char-based.

---

## PR-1: Data Integrity (~82 lines)

### Task 1.1: Add `char_len` and `char_truncate` helpers to `common` crate
- **PR:** PR-1
- **Theme:** 1
- **Errors:** — (foundation for C2–C5, H10–H12)
- **Files:** `crates/common/src/lib.rs`
- **Action:** Add two public utility functions to `common/src/lib.rs` (or a new `common/src/str_util.rs` module re-exported from `lib.rs`):
  ```rust
  /// Count Unicode characters (not bytes) in a string.
  pub fn char_len(s: &str) -> usize {
      s.chars().count()
  }

  /// Truncate a string to at most `max_chars` Unicode characters.
  /// Returns an owned String to avoid lifetime issues.
  pub fn char_truncate(s: &str, max_chars: usize) -> String {
      match s.char_indices().nth(max_chars) {
          Some((idx, _)) => s[..idx].to_string(),
          None => s.to_string(),
      }
  }
  ```
  If using a new module, add `pub mod str_util;` to `lib.rs` and `pub use str_util::{char_len, char_truncate};`.
- **Verify:** `cargo test -p common` — existing tests still pass. New tests added in Task 1.2.
- **Depends on:** (none)

---

### Task 1.2: Add tests for `char_len` and `char_truncate`
- **PR:** PR-1
- **Theme:** 1
- **Errors:** —
- **Files:** `crates/common/src/lib.rs` (or `crates/common/src/str_util.rs`)
- **Action:** Add `#[cfg(test)] mod tests` block with:
  - `test_char_len_ascii`: `char_len("hello") == 5`
  - `test_char_len_cjk`: `char_len("日本語") == 3`
  - `test_char_len_emoji`: `char_len("🎉🎊") == 2`
  - `test_char_len_mixed`: `char_len("Hello 日本語 🎉 café") == 18`
  - `test_char_len_empty`: `char_len("") == 0`
  - `test_char_truncate_cjk`: `char_truncate("日本語テスト", 3) == "日本語"`
  - `test_char_truncate_empty`: `char_truncate("", 100) == ""`
  - `test_char_truncate_exact`: string exactly at boundary returns full string
  - `test_char_truncate_ascii_noop`: short ASCII string returns unchanged
- **Verify:** `cargo test -p common`
- **Depends on:** Task 1.1

---

### Task 1.3: Fix `heading_parser.rs` UTF-8 byte/char confusion
- **PR:** PR-1
- **Theme:** 1
- **Errors:** C3
- **Files:** `crates/graph/src/heading_parser.rs:17,37`
- **Action:** Replace `line.len()` with `line.chars().count()` in both occurrences:
  - **Line 17** (code-block toggle): `char_offset += line.chars().count() + 1;`
  - **Line 37** (after heading processing): `char_offset += line.chars().count() + 1;`
- **Verify:** `cargo test -p graph` — existing `test_char_offsets` still passes (ASCII-only, so `len == chars().count()`).
- **Depends on:** (none — parallel with Task 1.1, but logical foundation for 1.4)

---

### Task 1.4: Add UTF-8 test for `heading_parser`
- **PR:** PR-1
- **Theme:** 1
- **Errors:** C3
- **Files:** `crates/graph/src/heading_parser.rs` (test module)
- **Action:** Add test `test_extract_headings_utf8_offsets`:
  ```rust
  let md = "## 🎉\n\nSome text\n\n## 日本語";
  let headings = extract_headings(md);
  assert_eq!(headings.len(), 2);
  assert_eq!(headings[0].title, "🎉");
  assert_eq!(headings[0].char_offset, 0);
  // "🎉" is 2 bytes but 1 char; "Some text" starts after blank lines
  assert_eq!(headings[1].title, "日本語");
  // Verify offset reflects char positions, not byte positions
  ```
  Also add `test_extract_headings_accented_offsets` with `"## Café\n\ncontent\n\n## Résumé"`.
- **Verify:** `cargo test -p graph`
- **Depends on:** Task 1.3

---

### Task 1.5: Fix `validator.rs` byte-slice truncation panic
- **PR:** PR-1
- **Theme:** 1
- **Errors:** C2
- **Files:** `crates/ingest/src/validator.rs:95-101`
- **Action:** Replace the byte-slice truncation:
  ```rust
  // Before:
  if trimmed.len() > 255 {
      trimmed[..255].to_string()
  } else {
      trimmed.to_string()
  }
  
  // After:
  if trimmed.chars().count() > 255 {
      trimmed.chars().take(255).collect::<String>()
  } else {
      trimmed.to_string()
  }
  ```
- **Verify:** `cargo test -p ingest` — existing tests pass (ASCII-only data, identical behavior).
- **Depends on:** (none)

---

### Task 1.6: Add UTF-8 test for `validator` truncation
- **PR:** PR-1
- **Theme:** 1
- **Errors:** C2
- **Files:** `crates/ingest/src/validator.rs` (test module)
- **Action:** Add test `test_derive_display_name_truncate_multibyte`:
  ```rust
  // 300 emoji characters — byte length is 1200, char length is 300
  let long_emoji = "🎉".repeat(300);
  let result = derive_display_name(&long_emoji, false);
  assert_eq!(result.chars().count(), 255);
  assert!(std::str::from_utf8(result.as_bytes()).is_ok());
  ```
  Also add `test_derive_display_name_truncate_cjk` with 260 CJK characters.
- **Verify:** `cargo test -p ingest`
- **Depends on:** Task 1.5

---

### Task 1.7: Fix `extractor.rs` UTF-8 byte/char confusion
- **PR:** PR-1
- **Theme:** 1
- **Errors:** C5
- **Files:** `crates/ingest/src/extractor.rs:37,75`
- **Action:** Replace `.len()` with `.chars().count()`:
  - **Line 37:** `let total_chars = content.chars().count();`
  - **Line 75:** `total_chars += text.chars().count();`
- **Verify:** `cargo test -p ingest` — existing tests pass (ASCII data).
- **Depends on:** (none)

---

### Task 1.8: Add UTF-8 test for `extractor` total_chars
- **PR:** PR-1
- **Theme:** 1
- **Errors:** C5
- **Files:** `crates/ingest/src/extractor.rs` (test module)
- **Action:** Add test `test_extract_plain_text_non_ascii_char_count`:
  - Create a temp file with `"日本語テスト"` (6 chars, 18 bytes).
  - Assert `result.total_chars == 6`.
  - Also test with accented text: `"élégant café"` (12 chars).
- **Verify:** `cargo test -p ingest`
- **Depends on:** Task 1.7

---

### Task 1.9: Fix `sentence_chunker.rs` UTF-8 byte/char confusion
- **PR:** PR-1
- **Theme:** 1
- **Errors:** C4, H10, H11, H12
- **Files:** `crates/ingest/src/sentence_chunker.rs:42,47,57`
- **Action:** Replace `current_text.len()` with `current_text.chars().count()` in all three locations:
  - **Line 42:** `} else if current_text.chars().count() < min_chars {`
  - **Line 47:** `let offset_end = current_offset_start + current_text.chars().count();`
  - **Line 57:** `let offset_end = current_offset_start + current_text.chars().count();`
- **Verify:** `cargo test -p ingest` — existing tests pass (all test data is ASCII where `len == chars().count()`).
- **Depends on:** (none)

---

### Task 1.10: Add UTF-8 test for `sentence_chunker` offsets
- **PR:** PR-1
- **Theme:** 1
- **Errors:** C4, H10, H11, H12
- **Files:** `crates/ingest/src/sentence_chunker.rs` (test module)
- **Action:** Add test `test_sentence_chunker_multibyte_offsets`:
  ```rust
  let text = "Café con leche. Más café.";
  let chunks = chunk_by_sentence(text, 5);
  // Verify offsets are character-based, not byte-based
  for chunk in &chunks {
      assert!(chunk.offset_end <= text.chars().count(),
              "offset_end {} exceeds char count {}", chunk.offset_end, text.chars().count());
  }
  ```
  Also add `test_sentence_chunker_multibyte_merge_threshold` with short multi-byte sentences to verify merge logic uses character count.
- **Verify:** `cargo test -p ingest`
- **Depends on:** Task 1.9

---

### Task 1.11: Add FK enforcement pragma to `storage`
- **PR:** PR-1
- **Theme:** 2
- **Errors:** C1
- **Files:** `crates/storage/src/lib.rs:41` (after `busy_timeout` pragma)
- **Action:** Add FK pragma after the `busy_timeout` block (line 41) in `Database::open()`:
  ```rust
  conn.pragma_update(None, "foreign_keys", "ON")
      .map_err(|e| CiteError::StorageError {
          message: format!("Failed to enable foreign keys: {e}"),
      })?;
  ```
  Also add the same pragma in `Database::open_memory()` after the `Self { conn }` construction (before `run_migrations`).
- **Verify:** `cargo test -p storage` — existing tests pass (all existing inserts have valid FKs).
- **Depends on:** (none)

---

### Task 1.12: Add FK enforcement tests
- **PR:** PR-1
- **Theme:** 2
- **Errors:** C1
- **Files:** `crates/storage/src/lib.rs` (test module)
- **Action:** Add tests:
  - `test_fk_pragma_returns_1`: Open in-memory DB, query `PRAGMA foreign_keys`, assert value is 1.
  - `test_fk_rejects_orphan_chunk`: Insert chunk with non-existent `document_id`, assert `Err(CiteError::StorageError)` containing "FOREIGN KEY".
  - `test_fk_allows_valid_insert`: Insert document first, then chunk referencing it, assert `Ok(())`.
- **Verify:** `cargo test -p storage`
- **Depends on:** Task 1.11

---

### Task 1.13: PR-1 quality gate
- **PR:** PR-1
- **Theme:** —
- **Errors:** —
- **Files:** —
- **Action:** Run full quality gate:
  ```bash
  cargo test
  cargo clippy -- -D warnings
  cargo fmt --check
  ```
  Verify that PR-1 changed lines are under 400.
- **Verify:** All three commands pass.
- **Depends on:** All PR-1 tasks (1.1–1.12)

---

## PR-2: Security + Onboarding (~41 lines)

### Task 2.1: Add empty API key validation to `GeminiProvider::new`
- **PR:** PR-2
- **Theme:** 4
- **Errors:** C7
- **Files:** `crates/providers/src/gemini.rs:24` (start of `new()`)
- **Action:** Add validation at the top of `new()`, before the endpoint format:
  ```rust
  if api_key.is_empty() {
      return Err(CiteError::ConfigError {
          message: "API key must not be empty. Set the CITE_API_KEY environment variable or add api_key to config.".to_string(),
      });
  }
  ```
- **Verify:** `cargo test -p providers` — existing `test_provider_creation` uses non-empty key, still passes.
- **Depends on:** (none)

---

### Task 2.2: Add empty API key validation to `OpenAICompatibleProvider::new`
- **PR:** PR-2
- **Theme:** 4
- **Errors:** C7
- **Files:** `crates/providers/src/openai.rs:29` (start of `new()`, before HTTPS check)
- **Action:** Add validation at the top of `new()`, before the `endpoint.starts_with("https://")` check:
  ```rust
  if api_key.is_empty() {
      return Err(CiteError::ConfigError {
          message: "API key must not be empty. Set the CITE_API_KEY environment variable or add api_key to config.".to_string(),
      });
  }
  ```
- **Verify:** `cargo test -p providers` — existing tests use non-empty keys, pass.
- **Depends on:** (none)

---

### Task 2.3: Add empty key tests for both providers
- **PR:** PR-2
- **Theme:** 4
- **Errors:** C7
- **Files:** `crates/providers/src/gemini.rs` (test module), `crates/providers/src/openai.rs` (test module)
- **Action:** Add tests:
  - In `gemini.rs`: `test_provider_rejects_empty_key` — `GeminiProvider::new("model", "")` → `Err(CiteError::ConfigError)`.
  - In `openai.rs`: `test_provider_rejects_empty_key` — `OpenAICompatibleProvider::new("https://x", "model", "")` → `Err(CiteError::ConfigError)`.
- **Verify:** `cargo test -p providers`
- **Depends on:** Tasks 2.1, 2.2

---

### Task 2.4: Replace `.unwrap_or_default()` with `.ok_or()` in CLI API key resolution
- **PR:** PR-2
- **Theme:** 4
- **Errors:** C7
- **Files:** `crates/cli/src/commands/mod.rs:94`
- **Action:** Replace:
  ```rust
  // Before:
  let api_key = resolve_api_key(config).unwrap_or_default();
  
  // After:
  let api_key = resolve_api_key(config).ok_or_else(|| common::CiteError::ConfigError {
      message: "No API key configured. Set the CITE_API_KEY environment variable or run `cite setup`.".to_string(),
  })?;
  ```
  This requires the function to return `Result`, which `create_provider` already does.
- **Verify:** `cargo test -p cli` — existing tests pass. Manual test: run `cite ingest` without API key set, confirm clear error message.
- **Depends on:** (none)

---

### Task 2.5: Wire `check_ingest_allowed` guard into CLI ingest command
- **PR:** PR-2
- **Theme:** 3
- **Errors:** C6
- **Files:** `crates/cli/src/commands/ingest.rs:65-67` (after `CommandContext::open`, before provider access)
- **Action:** Add guard call after the context is opened and before provider is used. Insert after line 63 (`Err(code) => return code`):
  ```rust
  if let Err(e) = engine::runtime_guard::check_ingest_allowed(&config.runtime.mode) {
      if json {
          print_json(&e.to_json_response());
      } else {
          eprintln!("Error: {e}");
      }
      return e.exit_code() as i32;
  }
  ```
  **Important:** Do NOT rename `production_mode` — that's a PR-3 refactor. Just wire the guard.
  Note: The guard must be placed before both the `args.queued` path and the direct ingest path. The simplest placement is immediately after `CommandContext::open()` succeeds.
- **Verify:** `cargo test` — existing tests pass. Confirm guard is exercised by the test in `runtime_guard.rs`.
- **Depends on:** (none)

---

### Task 2.6: Add guard integration test
- **PR:** PR-2
- **Theme:** 3
- **Errors:** C6
- **Files:** `crates/engine/src/runtime_guard.rs` (test module — tests already exist for the function itself)
- **Action:** Verify existing tests cover `RuntimeMode::Production` → `Err(RuntimeModeForbidden)` and `RuntimeMode::LocalPrivateDemo` → `Ok(())`. If not, add them. The function is already tested — this task is a verification checkpoint.
- **Verify:** `cargo test -p engine` — `check_ingest_allowed` tests pass.
- **Depends on:** Task 2.5

---

### Task 2.7: Fix rate limit composite key
- **PR:** PR-2
- **Theme:** 5
- **Errors:** C8
- **Files:** `crates/engine/src/retrieve.rs:278-280`
- **Action:** Replace `rate_limit_key` function body:
  ```rust
  // Before:
  fn rate_limit_key(provider: &dyn EmbeddingProvider) -> String {
      provider.provider_id().to_string()
  }
  
  // After:
  fn rate_limit_key(provider: &dyn EmbeddingProvider) -> String {
      format!("{}:{}", provider.provider_id(), provider.model_id())
  }
  ```
- **Verify:** `cargo test -p engine` — existing tests pass. Add test in Task 2.8.
- **Depends on:** (none)

---

### Task 2.8: Add rate limit key test
- **PR:** PR-2
- **Theme:** 5
- **Errors:** C8
- **Files:** `crates/engine/src/retrieve.rs` (test module)
- **Action:** Add test `test_rate_limit_key_includes_model_id`:
  ```rust
  // Use a mock provider or the existing TestProvider if available
  // Verify key format is "provider_id:model_id"
  ```
  If `rate_limit_key` is not `pub`, make it `pub(crate)` or test indirectly through `enforce_rate_limit`.
- **Verify:** `cargo test -p engine`
- **Depends on:** Task 2.7

---

### Task 2.9: PR-2 quality gate
- **PR:** PR-2
- **Theme:** —
- **Errors:** —
- **Files:** —
- **Action:** Run full quality gate:
  ```bash
  cargo test
  cargo clippy -- -D warnings
  cargo fmt --check
  ```
  Verify that PR-2 changed lines are under 400.
- **Verify:** All three commands pass.
- **Depends on:** All PR-2 tasks (2.1–2.8)

---

## PR-3: Config + Defensive + Robustness (~128 lines)

### Task 3.1: Add `timeout_secs` parameter to provider constructors
- **PR:** PR-3
- **Theme:** 6
- **Errors:** H15
- **Files:** `crates/providers/src/gemini.rs:24`, `crates/providers/src/openai.rs:29`
- **Action:** Change constructor signatures and use the parameter:
  - **`gemini.rs`:** `pub fn new(model: &str, api_key: &str, timeout_secs: u64) -> Result<Self, CiteError>` — replace `.timeout(std::time::Duration::from_secs(30))` with `.timeout(std::time::Duration::from_secs(timeout_secs))` on line 35.
  - **`openai.rs`:** `pub fn new(endpoint: &str, model: &str, api_key: &str, timeout_secs: u64) -> Result<Self, CiteError>` — replace `.timeout(std::time::Duration::from_secs(30))` with `.timeout(std::time::Duration::from_secs(timeout_secs))` on line 41.
- **Verify:** `cargo test -p providers` — update existing test call sites to pass `30` as the timeout parameter. All tests compile and pass.
- **Depends on:** (none)

---

### Task 3.2: Wire timeout from config to provider creation in CLI
- **PR:** PR-3
- **Theme:** 6
- **Errors:** H15
- **Files:** `crates/cli/src/commands/mod.rs:98,110` (the two provider creation lines)
- **Action:** Pass `config.ingest.embedding_timeout_secs` to both constructors:
  ```rust
  // Gemini:
  GeminiProvider::new(&config.embedding.model, &api_key, config.ingest.embedding_timeout_secs)?
  
  // OpenAI:
  OpenAICompatibleProvider::new(endpoint, &config.embedding.model, &api_key, config.ingest.embedding_timeout_secs)?
  ```
- **Verify:** `cargo test -p cli` — existing tests pass.
- **Depends on:** Task 3.1

---

### Task 3.3: Consolidate config field names
- **PR:** PR-3
- **Theme:** 6
- **Errors:** H13
- **Files:** `crates/config/src/lib.rs:93,108,111`, `crates/engine/src/ingest.rs:221`
- **Action:**
  1. Remove `min_chunk_size_chars` field from `IngestConfig` struct (line 93).
  2. Update `Default` impl: remove the `min_chunk_size_chars: 100` line.
  3. In `crates/engine/src/ingest.rs:221` (`run_pipeline`), replace `config.min_chunk_size_chars` with `config.min_chunk_chars` in the `chunker::chunk_text()` call.
  4. Update any env var mapping for `CITE_MIN_CHUNK_SIZE_CHARS` to point to `min_chunk_chars` instead.
  5. Change `default_max_chunk_chars()` return value from `200` to `1500` (line ~119).
- **Verify:** `cargo test` — all existing tests pass. Confirm no references to `min_chunk_size_chars` remain in the codebase.
- **Depends on:** (none)

---

### Task 3.4: Add config consolidation tests
- **PR:** PR-3
- **Theme:** 6
- **Errors:** H13
- **Files:** `crates/config/src/lib.rs` (test module)
- **Action:** Add tests:
  - `test_default_max_chunk_chars_is_1500`: Assert `IngestConfig::default().max_chunk_chars == 1500`.
  - `test_env_embedding_timeout_overridden`: Set `CITE_EMBEDDING_TIMEOUT=60`, load config, assert `embedding_timeout_secs == 60`.
- **Verify:** `cargo test -p config`
- **Depends on:** Task 3.3

---

### Task 3.5: Fix silenced `.ok()` in `snapshots.rs`
- **PR:** PR-3
- **Theme:** 7
- **Errors:** H17
- **Files:** `crates/storage/src/snapshots.rs:81-89`
- **Action:** Replace `.ok()` with `.optional().map_err(storage_err)?`:
  ```rust
  // Before:
  let previous_snapshot_id: Option<String> = tx
      .query_row(
          "SELECT active_snapshot_id FROM snapshot_pointer WHERE id = 1",
          [],
          |row| row.get(0),
      )
      .ok();
  
  // After:
  let previous_snapshot_id: Option<String> = tx
      .query_row(
          "SELECT active_snapshot_id FROM snapshot_pointer WHERE id = 1",
          [],
          |row| row.get(0),
      )
      .optional()
      .map_err(storage_err)?;
  ```
  `OptionalExtension` is already imported on line 4.
- **Verify:** `cargo test -p storage` — existing tests pass (they use valid DBs, so no unexpected errors).
- **Depends on:** (none)

---

### Task 3.6: Add test for silenced snapshot error
- **PR:** PR-3
- **Theme:** 7
- **Errors:** H17
- **Files:** `crates/storage/src/snapshots.rs` (test module)
- **Action:** Add test `test_activate_snapshot_returns_none_when_no_pointer`:
  - Open in-memory DB, don't insert any snapshot_pointer row.
  - Attempt to read the active pointer using the same query pattern.
  - Assert `Ok(None)`.
- **Verify:** `cargo test -p storage`
- **Depends on:** Task 3.5

---

### Task 3.7: Fix silenced cleanup errors in `engine/ingest.rs`
- **PR:** PR-3
- **Theme:** 7
- **Errors:** H6
- **Files:** `crates/engine/src/ingest.rs:191-193,246-247`
- **Action:** Replace `let _ =` with log-and-continue:
  - **Line 191:** `let _ = cleanup_partial(db, &document_id);` →
    ```rust
    if let Err(e) = cleanup_partial(db, &document_id) {
        eprintln!("Warning: cleanup failed for {document_id}: {e}");
    }
    ```
  - **Lines 246-247** (inside `cleanup_partial`):
    ```rust
    // Before:
    let _ = db.delete_embeddings_for_document(document_id);
    let _ = db.delete_chunks_for_document(document_id);
    
    // After:
    if let Err(e) = db.delete_embeddings_for_document(document_id) {
        eprintln!("Warning: failed to delete embeddings for {document_id}: {e}");
    }
    if let Err(e) = db.delete_chunks_for_document(document_id) {
        eprintln!("Warning: failed to delete chunks for {document_id}: {e}");
    }
    ```
- **Verify:** `cargo test -p engine` — existing tests pass.
- **Depends on:** (none)

---

### Task 3.8: Fix integer cast safety in `storage/src/util.rs`
- **PR:** PR-3
- **Theme:** 8
- **Errors:** H18
- **Files:** `crates/storage/src/util.rs:37,42,47,51`
- **Action:** Replace `as u32` with `u32::try_from(...)`:
  ```rust
  // Line 37 (chunk_index — non-optional):
  chunk_index: u32::try_from(row.get::<_, i64>("chunk_index").map_err(storage_err)?)
      .map_err(|e| storage_err(format!("chunk_index overflow: {e}")))?,
  
  // Lines 42, 47, 51 (page, offset_start, offset_end — optional):
  .map(|v| u32::try_from(v).map_err(|e| storage_err(format!("page overflow: {e}"))))
  .transpose()?,
  ```
  Repeat with appropriate field names for `offset_start` and `offset_end`.
- **Verify:** `cargo test -p storage` — existing tests pass.
- **Depends on:** (none)

---

### Task 3.9: Fix integer cast safety in `storage/src/embeddings.rs`
- **PR:** PR-3
- **Theme:** 8
- **Errors:** H18
- **Files:** `crates/storage/src/embeddings.rs:144,148,152,155` (in `list_chunk_embeddings_hierarchical`), lines ~209,~213,~217,~220 (in `list_ready_chunk_embeddings`)
- **Action:** Apply the same `u32::try_from()` pattern as Task 3.8 to all `as u32` casts in both functions. There are 8 total cast sites (4 per function).
- **Verify:** `cargo test -p storage`
- **Depends on:** (none)

---

### Task 3.10: Add integer cast safety tests
- **PR:** PR-3
- **Theme:** 8
- **Errors:** H18
- **Files:** `crates/storage/src/util.rs` or `crates/storage/src/lib.rs` (test module)
- **Action:** Add test `test_row_to_chunk_valid_index`:
  - Insert a document and a chunk with `chunk_index = 42`, read it back, assert `chunk.chunk_index == 42`.
  - Note: Testing the overflow case requires inserting a value > `u32::MAX` into SQLite, which may be impractical. Document the `try_from` guard in a code comment instead.
- **Verify:** `cargo test -p storage`
- **Depends on:** Tasks 3.8, 3.9

---

### Task 3.11: Add `CommandContext::provider()` helper method
- **PR:** PR-3
- **Theme:** 9
- **Errors:** H3
- **Files:** `crates/cli/src/commands/context.rs` (the `CommandContext` impl block)
- **Action:** Add method:
  ```rust
  /// Get the embedding provider, returning an error if none was configured.
  pub fn provider(&self) -> Result<&dyn EmbeddingProvider, CiteError> {
      self.provider
          .as_deref()
          .ok_or_else(|| CiteError::ConfigError {
              message: "No embedding provider configured".to_string(),
          })
  }
  ```
  This requires `EmbeddingProvider` to be in scope (it already is via `use super::...`).
- **Verify:** `cargo test -p cli` — existing tests pass.
- **Depends on:** (none)

---

### Task 3.12: Replace `.unwrap()` with `.provider()?` in CLI commands
- **PR:** PR-3
- **Theme:** 9
- **Errors:** H3
- **Files:** `crates/cli/src/commands/context.rs:50`, `crates/cli/src/commands/ingest.rs:66`, `crates/cli/src/commands/retrieve.rs:80`
- **Action:** Replace all three occurrences:
  ```rust
  // Before:
  let provider = ctx.provider.as_ref().unwrap();
  
  // After:
  let provider = ctx.provider()?;
  ```
  The return type change from `&Box<dyn EmbeddingProvider>` to `&dyn EmbeddingProvider` is handled by `as_deref()` in the helper. Ensure downstream code works with `&dyn EmbeddingProvider` (it already does, since `.as_ref()` on `Box<dyn T>` gives `&Box<dyn T>`, and existing calls use `provider.as_ref()` to get `&dyn T` — verify this carefully).
  
  **Note:** In `ingest.rs:66` and `retrieve.rs:80`, the existing code does `ctx.provider.as_ref().unwrap()` which gives `&Box<dyn EmbeddingProvider>`. The callers then pass `provider.as_ref()` which gives `&dyn EmbeddingProvider`. After the change, `ctx.provider()` returns `Result<&dyn EmbeddingProvider, _>`, so the callers should use `provider` directly (no `.as_ref()` needed). Update all downstream usages.
- **Verify:** `cargo test -p cli`
- **Depends on:** Task 3.11

---

### Task 3.13: Fix `hierarchy.rs` duplicate heading assignment
- **PR:** PR-3
- **Theme:** 10
- **Errors:** H8
- **Files:** `crates/graph/src/hierarchy.rs:130-146`
- **Action:** Replace `.find()`-based matching with sequential heading consumption:
  ```rust
  let mut heading_idx = 0usize;
  for (t_idx, topic_with_concepts) in topics.iter().enumerate() {
      // Advance heading_idx to find the next H2 matching this topic
      while heading_idx < headings.len() {
          let h = &headings[heading_idx];
          if h.level == 2 && h.title == topic_with_concepts.topic.name {
              boundaries.push((h.char_offset, t_idx, None));
              heading_idx += 1;
              break;
          }
          heading_idx += 1;
      }
      // Match concepts (H3) from current position
      let mut concept_heading_idx = heading_idx;
      for (c_idx, concept_with_chunks) in topic_with_concepts.concepts.iter().enumerate() {
          while concept_heading_idx < headings.len() {
              let h = &headings[concept_heading_idx];
              if h.level == 3 && h.title == concept_with_chunks.concept.name {
                  boundaries.push((h.char_offset, t_idx, Some(c_idx)));
                  concept_heading_idx += 1;
                  break;
              }
              concept_heading_idx += 1;
          }
      }
  }
  ```
  Ensure the resulting `boundaries` are still sorted by `char_offset` before use.
- **Verify:** `cargo test -p graph` — existing tests pass.
- **Depends on:** (none — but Theme 1 (PR-1) must be merged first so char_offset is correct)

---

### Task 3.14: Add duplicate heading test
- **PR:** PR-3
- **Theme:** 10
- **Errors:** H8
- **Files:** `crates/graph/src/hierarchy.rs` (test module)
- **Action:** Add test with a document containing duplicate `## Overview` headings:
  - Build a heading list with two H2 "Overview" at different offsets.
  - Assign chunks to each section.
  - Assert first chunk → first "Overview", second chunk → second "Overview".
- **Verify:** `cargo test -p graph`
- **Depends on:** Task 3.13

---

### Task 3.15: Fix indented code fence detection in `heading_parser.rs`
- **PR:** PR-3
- **Theme:** 10
- **Errors:** H9
- **Files:** `crates/graph/src/heading_parser.rs:14`
- **Action:** Change the fence detection:
  ```rust
  // Before:
  if trimmed.starts_with("```") {
  
  // After:
  if line.trim_start().starts_with("```") {
  ```
  This catches fences with leading whitespace (e.g., `    ``` `). Note: use `line.trim_start()` instead of `trimmed` because `trimmed` is already `line.trim()` which strips all leading whitespace — but the original code already uses `trimmed`. The fix needs to check the *original* line for leading whitespace before the fence marker. Since `trimmed` already strips all whitespace, the current code is correct for detecting `` ``` `` after trimming. The actual issue is that `trimmed` is defined as `line.trim()`, which already removes all leading whitespace. So `trimmed.starts_with("```")` works for `    ``` ` because `trimmed` would be `` ``` `` after trimming.
  
  **Re-evaluate:** After reading the source, `let trimmed = line.trim();` means `trimmed` for `    ``` ` is `` ``` `` — so the check `trimmed.starts_with("```")` already works. The real issue might be different. Verify with a test before changing production code.
  
  **Revised action:** Add a test for indented fences first. If it passes, this is a false alarm. If it fails, fix accordingly.
- **Verify:** `cargo test -p graph`
- **Depends on:** (none)

---

### Task 3.16: Fix `evaluate.rs` dead `json` parameter
- **PR:** PR-3
- **Theme:** 11
- **Errors:** H1
- **Files:** `crates/cli/src/commands/evaluate.rs:359`
- **Action:** Change the `execute` function signature:
  ```rust
  // Before:
  pub fn execute(_args: &EvaluateArgs, _config: &Config, json: bool) -> i32 {
  
  // After:
  pub fn execute(args: &EvaluateArgs, _config: &Config, _json: bool) -> i32 {
  ```
  Then update all uses of `json` inside the function body to use `args.json` instead. If the function body currently uses `json` (the parameter), change them to `args.json`.
  
  **Note:** If the function is called from a dispatcher that passes `args.json` as the third argument, update that call site to pass a dummy value or remove the parameter. Check `crates/cli/src/main.rs` or the command dispatch for the call site.
- **Verify:** `cargo test -p cli`, `cargo clippy -- -D warnings`
- **Depends on:** (none)

---

### Task 3.17: Add `PartialEq` derive to `CiteError`
- **PR:** PR-3
- **Theme:** 11
- **Errors:** C11
- **Files:** `crates/common/src/error.rs:7`
- **Action:** Add `PartialEq` to the derive:
  ```rust
  // Before:
  #[derive(Debug, thiserror::Error)]
  
  // After:
  #[derive(Debug, PartialEq, thiserror::Error)]
  ```
- **Verify:** `cargo test -p common` — existing tests pass. Add a test asserting `CiteError::ConfigError { message: "x".into() } == CiteError::ConfigError { message: "x".into() }`.
- **Depends on:** (none)

---

### Task 3.18: Remove unused `tokio` and `tracing` deps from `providers`
- **PR:** PR-3
- **Theme:** 11
- **Errors:** H16
- **Files:** `crates/providers/Cargo.toml:8-9`
- **Action:** Remove these two lines:
  ```toml
  tokio = { workspace = true }
  tracing = { workspace = true }
  ```
- **Verify:** `cargo build -p providers` — compiles successfully. `cargo test -p providers` — tests pass. `cargo clippy -- -D warnings` — no warnings.
- **Depends on:** (none)

---

### Task 3.19: PR-3 quality gate
- **PR:** PR-3
- **Theme:** —
- **Errors:** —
- **Files:** —
- **Action:** Run full quality gate:
  ```bash
  cargo test
  cargo clippy -- -D warnings
  cargo fmt --check
  ```
  Verify that PR-3 changed lines are under 400.
- **Verify:** All three commands pass.
- **Depends on:** All PR-3 tasks (3.1–3.18)

---

## Deferred Items (Explicitly Out of Scope)

| Item | Reason |
|------|--------|
| Theme 11.4: `ScoredChunk` field duplication | Changes public retrieval API; deferred to architecture pass |
| DRY refactoring (3 themes) | T3 tier, separate pass |
| Dead code cleanup (6+ items) | T3 tier, separate pass |
| Test infrastructure (14 errors) | T3 tier, separate pass |
| Newtype migration (~50 files) | Separate effort |
| Type consistency (3 themes) | T3 tier, separate pass |
| Snapshot rollback completeness (H7) | Deferred to second pass |

---

## Task Summary

| PR | Tasks | Est. Prod Lines | Est. Test Lines | Est. Total |
|----|-------|:---:|:---:|:---:|
| PR-1 | 13 (1.1–1.13) | ~21 | ~61 | ~82 |
| PR-2 | 9 (2.1–2.9) | ~13 | ~28 | ~41 |
| PR-3 | 19 (3.1–3.19) | ~67 | ~61 | ~128 |
| **Total** | **41** | **~101** | **~150** | **~251** |
