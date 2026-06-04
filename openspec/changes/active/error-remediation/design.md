# Design: Error Remediation — 11 Themes, 3 PRs

**Change ID:** error-remediation
**Date:** 2026-06-02
**Status:** design
**Inputs:** proposal.md, spec.md, source-verification.md, analisis-final-v2.md

---

## Table of Contents

1. [Fix Architecture](#1-fix-architecture)
2. [Technical Decisions](#2-technical-decisions)
3. [Error Handling Strategy](#3-error-handling-strategy)
4. [Testing Strategy](#4-testing-strategy)
5. [Rollout Strategy](#5-rollout-strategy)
6. [Contracts](#6-contracts)

---

## 1. Fix Architecture

### 1.1 System Context

The fixes span 9 crates but are organized by **theme** (cross-cutting concern), not by crate. Each theme represents a failure pattern that appears in one or more crates. The 3-PR structure groups themes by blast radius and risk profile.

```
┌─────────────────────────────────────────────────────────────┐
│                        CLI (commands/)                       │
│  Theme 4: unwrap_or_default → error  ┌──────────────────┐   │
│  Theme 9: unwrap() → .provider()?    │ CommandContext     │   │
│  Theme 11: evaluate --json, dead     │   .open() →       │   │
│          param                       │   create_provider()│   │
│                                      └────────┬─────────┘   │
└──────────────────────────────┬──────────────────┘            │
                               │                               │
┌──────────────────────────────▼───────────────────────────────┐
│                      Engine (engine/)                         │
│  Theme 3: wire check_ingest_allowed()                        │
│  Theme 5: composite rate_limit_key                           │
│  Theme 7: log cleanup_partial errors                         │
└────┬────────────────────────┬────────────────────────────┬───┘
     │                        │                            │
┌────▼───────┐   ┌───────────▼──────────┐   ┌────────────▼───┐
│  Ingest    │   │    Providers          │   │   Storage      │
│ Theme 1:   │   │ Theme 4: empty key   │   │ Theme 2: FK    │
│  len()→    │   │ Theme 6: timeout     │   │ Theme 7: .ok() │
│  chars()   │   │  passthrough         │   │ Theme 8: casts │
│            │   │ Theme 11: rm deps    │   │                │
└────────────┘   └──────────────────────┘   └────────────────┘
     │
┌────▼───────┐   ┌──────────────────────┐
│  Graph     │   │   Config              │
│ Theme 1:   │   │ Theme 6: field        │
│  offset    │   │  consolidation        │
│ Theme 10:  │   │  + timeout wiring     │
│  hierarchy │   └──────────────────────┘
└────────────┘
     │
┌────▼───────┐   ┌──────────────────────┐
│  Common    │   │   Retrieval           │
│ Theme 1:   │   │ Theme 11: ScoredChunk │
│  helpers   │   │  (evaluate only)      │
│ Theme 11:  │   └──────────────────────┘
│  PartialEq │
└────────────┘
```

### 1.2 Data Flow Changes Per Theme

#### Theme 1: UTF-8 Bytes-vs-Chars (PR-1)

**Current data flow (broken for non-ASCII):**
```
User file (UTF-8) → heading_parser: char_offset += line.len() [bytes]
                  → extractor: total_chars = content.len() [bytes]
                  → sentence_chunker: current_text.len() [bytes]
                  → validator: trimmed[..255] [byte slice, panic risk]
                  ↓
                  Offsets in chars, headings in bytes → WRONG topic assignment
```

**Fixed data flow:**
```
User file (UTF-8) → heading_parser: char_offset += line.chars().count() [chars]
                  → extractor: total_chars = content.chars().count() [chars]
                  → sentence_chunker: current_text.chars().count() [chars]
                  → validator: trimmed.chars().take(255).collect() [char boundary safe]
                  ↓
                  Offsets and headings both in chars → CORRECT topic assignment
```

**Key invariant after fix:** All offset-related measurements (`char_offset`, `offset_start`, `offset_end`, `total_chars`, truncation boundaries) operate on Unicode scalar values (`.chars().count()`), not bytes (`.len()`). The `common` crate provides `char_len` and `char_truncate` helpers as a named alternative to inline `chars().count()`.

#### Theme 2: FK Enforcement (PR-1)

**Current:** SQLite accepts orphan rows. `PRAGMA foreign_keys` is never set.
**Fixed:** `Database::open()` and `Database::open_memory()` both execute `PRAGMA foreign_keys = ON` after WAL/busy_timeout setup. Any insert violating FK constraints returns `CiteError::StorageError`.

**Data flow change:** None — this is a gate, not a transformation. The existing DDL migrations already define FK constraints; this just enables enforcement.

#### Theme 3: Production Mode Guard (PR-2)

**Current:** `check_ingest_allowed()` is defined and tested in `engine/src/runtime_guard.rs` but never called. `production_mode` only controls display name sanitization.

**Fixed:** `ingest()` and `ingest_next()` call `runtime_guard::check_ingest_allowed(&config.runtime.mode)?` at the top of their entry points (in `cli/src/commands/ingest.rs`).

```
CLI ingest command
  → check_ingest_allowed(mode)?     ← NEW: blocks Production/PublicPackagedDemo
  → create_provider(config)?         ← existing
  → ingest::ingest(db, provider, ...)  ← existing, production_mode still controls display name
```

**Why guard in CLI, not engine:** The guard depends on `config.runtime.mode`, which is only available at the CLI layer. The engine functions receive `production_mode: bool`, not the mode enum. Wiring the guard at the CLI level avoids changing the engine function signatures (which would cascade into every caller).

#### Theme 4: Empty API Key (PR-2)

**Current flow:**
```
resolve_api_key(config) → Option<String>
.unwrap_or_default() → "" (empty string)
→ provider.new(model, "") → accepts silently
→ embed("text") → HTTP 401 "Unauthorized" (cryptic)
```

**Fixed flow:**
```
resolve_api_key(config) → Option<String>
.ok_or(CiteError::ConfigError { "No API key configured..." })? → String
→ provider.new(model, &key) → validates non-empty
→ embed("text") → normal operation
```

**Defense in depth:** Even if a future code path bypasses CLI validation, providers themselves reject empty keys at construction time.

#### Theme 5: Rate Limit Composite Key (PR-2)

**Current:** `rate_limit_key = provider.provider_id()` — two models on the same provider share one rate limit bucket.

**Fixed:** `rate_limit_key = format!("{}:{}", provider.provider_id(), provider.model_id())` — each provider+model pair gets its own bucket.

**Why not the full 4-part composite (mode, corpus, provider, scope):** The `enforce_rate_limit` function only receives `provider` and `route`. The `route` already differentiates search/context/retrieve. Adding `model_id` is the minimal correct fix. Full composite key requires changing the function signature (deferred).

#### Theme 6: Config-Disconnect (PR-3)

**Current:** `embedding_timeout_secs` exists in config but providers hardcode 30s. `min_chunk_size_chars` and `min_chunk_chars` are two confusingly similar fields. `max_chunk_chars: 200` contradicts `chunk_size_chars: 1000`.

**Fixed flow:**
```
Config::load() → config.ingest.embedding_timeout_secs (30 default, env CITE_EMBEDDING_TIMEOUT)
  → CLI passes to provider constructor: GeminiProvider::new(model, key, timeout_secs)
  → Client::builder().timeout(Duration::from_secs(timeout_secs))

Config field consolidation:
  min_chunk_size_chars → REMOVED (renamed to min_chunk_chars)
  min_chunk_chars      → RETAINED (now the single canonical field)
  max_chunk_chars      → default changed from 200 to 1500
```

**Two-phase approach for field rename:**
- **PR-1 (Theme 1):** Fix sentence_chunker using *current* field names (`min_chunk_chars`). The fixed-size chunker uses `min_chunk_size_chars` — leave it as-is.
- **PR-3 (Theme 6):** Rename `min_chunk_size_chars` → remove it from struct, consolidate all references to `min_chunk_chars`. Update env var mapping from `CITE_MIN_CHUNK_SIZE_CHARS` → use `CITE_MIN_CHUNK_CHARS` (already exists).

#### Theme 7: Silenced Errors (PR-3)

**Current patterns:**
- `storage/snapshots.rs:68-73`: `.ok()` converts ALL errors to `None`
- `engine/ingest.rs:191-193`: `let _ = cleanup_partial(...)` discards cleanup errors
- `engine/ingest.rs:246-247`: `let _ = db.delete_embeddings/chunks(...)` discards deletion errors

**Fixed patterns:**
- `snapshots.rs`: `.optional()` from `rusqlite::OptionalExtension` — only `QueryReturnedNoRows` becomes `None`; other errors propagate as `StorageError`
- `ingest.rs`: `if let Err(e) = cleanup_partial(...) { eprintln!("Warning: cleanup failed for {document_id}: {e}") }` — log to stderr, don't suppress
- `ingest.rs`: Same log-then-continue for `delete_embeddings` and `delete_chunks`

**Design decision: log-then-continue, not propagate.** Cleanup errors during failure handling should not mask the original error. If cleanup fails, the original error is still the one the user needs to see. Logging to stderr ensures observability without changing error semantics.

#### Theme 8: Integer Cast Safety (PR-3)

**Current:** `row.get::<_, i64>("chunk_index")? as u32` — silent truncation for values > 4,294,967,295.

**Fixed:** `u32::try_from(row.get::<_, i64>("chunk_index")?).map_err(storage_err)?`

**Scope:** All `as u32` casts in `storage/src/util.rs` (4 locations), `storage/src/embeddings.rs` (8 locations). No other storage files use unchecked casts (verified via grep).

#### Theme 9: Provider Unwrap (PR-3)

**Current:** 3 of 4 commands use `ctx.provider.as_ref().unwrap()`. Only `search.rs` handles it correctly.

**Fixed:** Add `CommandContext::provider()` method returning `Result<&dyn EmbeddingProvider, CiteError>`. All 4 commands use the same safe accessor.

```rust
impl CommandContext {
    pub fn provider(&self) -> Result<&dyn EmbeddingProvider, CiteError> {
        self.provider.as_deref().ok_or_else(|| CiteError::ConfigError {
            message: "No embedding provider configured".to_string(),
        })
    }
}
```

**Why a method, not `ok_or` inline:** The method ensures future commands get safe access by default. The `ConfigError` variant is appropriate because a missing provider is a configuration problem (e.g., `open_db_only()` was used for a command that needs a provider).

#### Theme 10: Graph Robustness (PR-3)

**Current:** `hierarchy.rs` uses `headings.iter().find(|h| h.title == name)` which returns the *first* matching heading. Duplicate `## Overview` headings cause all subsequent chunks to be assigned to the first occurrence.

**Fixed:** Sequential heading consumption. Instead of searching by title, track a position cursor through the headings list. Topics and concepts are matched in the order they appear in the document (which matches the order `build_hierarchy` creates them from headings).

```rust
let mut heading_idx = 0usize;
for (t_idx, topic_with_concepts) in topics.iter().enumerate() {
    // Advance heading_idx past already-consumed headings
    while heading_idx < headings.len() {
        let h = &headings[heading_idx];
        if h.level == 2 && h.title == topic_with_concepts.topic.name {
            boundaries.push((h.char_offset, t_idx, None));
            heading_idx += 1;
            break;
        }
        heading_idx += 1;
    }
    // ... similar for concepts
}
```

**Why not index-based (no title matching at all):** Topics are created in heading order, so matching by sequential position is correct. However, we still verify the title to maintain the invariant that boundaries correspond to the expected topic. This handles edge cases where an H2 is followed by H4+ (which are ignored by `build_hierarchy` but still consumed by the heading iterator).

**Code block fence fix:** The `trimmed.starts_with("```")` check already works correctly for ` ```rust ` etc. The line is already trimmed, so indented fences (e.g., `    ``` `) are not detected. The fix: `line.trim_start().starts_with("```")` instead of `trimmed.starts_with("```")`. This catches fences with leading whitespace.

#### Theme 11: Misc High-Tier (PR-3)

| Fix | Data Flow Change |
|-----|-----------------|
| `evaluate.rs` `--json` dead param | `execute(_args, _config, json)` → `execute(args, _config, _json)` using `args.json` |
| `CiteError` `PartialEq` | Add `PartialEq` to derive — all variants eligible (`PathBuf`, `Option<String>`, `u32` all implement `PartialEq`) |
| `providers/Cargo.toml` unused deps | Remove `tokio` and `tracing` — zero references in `crates/providers/src/` |
| `ScoredChunk` field duplication | **Deferred** — embedding `ChunkEmbeddingRecord` changes the public retrieval API and touches engine+cli consumers. Not worth the risk in this pass. Document as tech debt. |

---

## 2. Technical Decisions

### Decision 1: `chars().count()` instead of a newtype for string measurements

**Context:** The UTF-8 bug exists because `str::len()` returns byte count. An alternative approach would be to introduce a `CharCount(usize)` newtype that wraps the result of `chars().count()` and only permits construction through a safe path.

**Decision:** Use `.chars().count()` inline (with `common::char_len` helper for discoverability). Do NOT create a newtype.

**Rationale:**
- Newtypes are the right answer for domain identifiers (`DocumentId`, `ChunkId`) where mixing types is a compile-time bug. Character counts don't mix — they're always `usize` compared to other `usize` values.
- A `CharCount` newtype would require `.0` access everywhere, adding noise to arithmetic like `current_offset_start + current_text.chars().count()`.
- The `common::char_len` and `common::char_truncate` helpers provide named discoverability without the ceremony of a newtype. Future developers searching for "how do I count characters" will find them.
- CI grep check (see §4.5) catches future misuse of `.len()` in offset/truncation contexts.

### Decision 2: `try_from` instead of a wrapper for integer casts

**Context:** The `as u32` casts in storage silently truncate `i64` values. Options include: (a) `u32::try_from(v).map_err(storage_err)?`, (b) a `safe_cast(v) -> Result<u32, CiteError>` helper, (c) changing SQLite column types.

**Decision:** Use `u32::try_from(v).map_err(storage_err)?` inline.

**Rationale:**
- `try_from` is the idiomatic Rust pattern for checked casts. It's universally understood and grep-able.
- A helper function adds indirection without adding clarity — the error mapping is always the same (`storage_err`).
- Changing SQLite column types (to `INTEGER` instead of `BIGINT`) would require a migration and doesn't solve the problem if values genuinely exceed `u32::MAX`.
- The cast site is the right place to make the decision because different call sites might want different error types.

### Decision 3: Log-then-continue for cleanup errors

**Context:** When `ingest_internal` fails, it calls `cleanup_partial` to delete partial chunks and embeddings. Currently, cleanup errors are discarded with `let _ =`. Options: (a) propagate cleanup errors, (b) log and continue, (c) ignore.

**Decision:** Log to stderr and continue. Do NOT propagate.

**Rationale:**
- The cleanup runs in the *failure path*. The user needs to see the *original* error (e.g., "embedding failed: HTTP 403"), not "cleanup failed: database locked". Propagating cleanup errors would mask the real problem.
- `let _ =` is worse — orphan data accumulates silently and there's no evidence cleanup was attempted.
- `eprintln!` is appropriate because the engine has no logging framework (`tracing` is being removed from providers as unused). A future pass could switch to `tracing::warn!` when the project adopts structured logging.
- Cleanup failure is not a user-facing error — it's an operational concern. The document is already marked `Failed`, which is the correct user-visible state.

### Decision 4: Guard at CLI layer, not engine

**Context:** `check_ingest_allowed()` takes a `&RuntimeMode`. The engine's `ingest()` and `ingest_next()` receive `production_mode: bool`, not `RuntimeMode`. Where should the guard be wired?

**Decision:** Call `check_ingest_allowed` in `cli/src/commands/ingest.rs` before any engine calls.

**Rationale:**
- `RuntimeMode` is a config concept. The engine is designed to be config-agnostic — it receives pre-processed parameters.
- Changing engine function signatures to accept `RuntimeMode` instead of `bool` would cascade to every caller and change the engine's API contract.
- The CLI is the correct boundary for "should this operation be allowed at all?" — it's where user intent meets system policy.
- `enqueue_ingest` also needs guarding (it doesn't call the engine), which is only possible at the CLI layer.

### Decision 5: Two-phase config field rename

**Context:** `min_chunk_size_chars` (used by fixed-size chunker) and `min_chunk_chars` (used by sentence chunker) are confusingly similar. Theme 1 (PR-1) fixes the sentence chunker's UTF-8 bugs. Theme 6 (PR-3) consolidates fields.

**Decision:** PR-1 fixes sentence_chunker using the *current* field name (`min_chunk_chars`). PR-3 renames `min_chunk_size_chars` → removes it, consolidates to `min_chunk_chars`.

**Rationale:**
- If we renamed fields in PR-1, the PR would mix "fix bugs" with "rename config" — two different concerns that confuse reviewers.
- PR-1 and PR-3 are sequential. After PR-1, the sentence chunker works correctly with `min_chunk_chars`. After PR-3, the fixed-size chunker also uses `min_chunk_chars` (which had a different default — 100 → the consolidated default needs a product decision, but 100 is reasonable since the sentence chunker used 30).
- Env var migration: `CITE_MIN_CHUNK_SIZE_CHARS` is removed. `CITE_MIN_CHUNK_CHARS` (already defined in `EnvOverrides`) takes over.

### Decision 6: `PartialEq` on `CiteError` via derive

**Context:** `CiteError` currently derives `Debug` and `thiserror::Error`. Adding `PartialEq` enables `assert_eq!` on errors in tests.

**Decision:** Add `PartialEq` to the derive list.

**Rationale:**
- All enum variants contain types that implement `PartialEq` (`String`, `PathBuf`, `u32`, `Option<String>`).
- This enables cleaner test assertions: `assert_eq!(result.unwrap_err(), CiteError::ConfigError { ... })` instead of `assert!(matches!(...))`.
- `PartialEq` on errors is a common Rust pattern (e.g., `std::io::ErrorKind` is `PartialEq`). The main caveat is that `std::io::Error` itself is not `PartialEq` — but `CiteError` is a closed enum with no dynamic content, so structural equality is meaningful.

### Decision 7: `char_truncate` returns owned `String`, not `&str`

**Context:** The `common` crate will have `char_truncate(s: &str, max: usize)`. Should it return `&str` (zero-copy) or `String`?

**Decision:** Return `String`.

**Rationale:**
- `char_truncate` needs to find the byte position of the Nth character. `char_indices().nth(max)` returns a byte index, which can be used to slice `&s[..idx]` — but this lifetime ties the return to the input, which is inconvenient for the current callers (which already produce owned `String`s).
- The callers (`sanitize_display_name`, `validator.rs`) already work with `String` values. Returning `String` avoids lifetime issues.
- An alternative `char_truncate_ref(s: &str, max: usize) -> &str` could be added later for performance-sensitive paths, but no current call site needs it.

---

## 3. Error Handling Strategy

### 3.1 When to Propagate vs Log-and-Continue

The codebase uses three error handling patterns. This change standardizes when each is appropriate:

| Pattern | When to Use | Examples in This Change |
|---------|-------------|------------------------|
| **Propagate (`?`)** | The error prevents the operation from completing correctly. The caller (or user) needs to know. | FK violation on insert, empty API key, `try_from` overflow, `check_ingest_allowed` rejection, `.optional()` on snapshot query |
| **Log-and-continue** | The error is in a secondary/cleanup path. Suppressing it would hide operational issues, but propagating it would mask the primary error. | `cleanup_partial` failure, `delete_embeddings_for_document` failure, `delete_chunks_for_document` failure |
| **Log-and-propagate** | Same as propagate, but the error context needs enrichment before surfacing. | `decode_vector_blob` returning `None` (currently `continue` — deferred to second pass as it requires handling the partial result set) |

### 3.2 Error Message Conventions

All `CiteError` variants follow the `#[error("...")]` format from thiserror. New error messages in this change follow these conventions:

| Variant | Message Format | Example |
|---------|---------------|---------|
| `ConfigError` | Action-oriented: what's wrong + what to do | `"No API key configured. Set CITE_API_KEY environment variable or add api_key to config file."` |
| `StorageError` | Operation + cause | `"Failed to enable foreign keys: {e}"` |
| `RuntimeModeForbidden` | Operation + reason | `"Ingest is not allowed in production mode"` |
| `EmbeddingProviderError` | Provider + HTTP status + body | `"Gemini API returned HTTP 401: {body}"` |

**Principles:**
- Messages include enough context to diagnose without a stack trace.
- For `ConfigError`, messages include the remediation step (set env var, run command).
- For `StorageError`, messages include the underlying error via `{e}`.
- No error message includes the word "error" — that's redundant since the prefix is already "Error: {e}" in the CLI handler.

### 3.3 How to Use `CiteError` Variants

Each theme maps to specific `CiteError` variants. The design ensures consistency:

| Theme | Primary Variant | Fallback Variant |
|-------|----------------|-----------------|
| Theme 1 (UTF-8) | N/A — no new errors, just correct computation | — |
| Theme 2 (FK) | `StorageError` | — |
| Theme 3 (Guard) | `RuntimeModeForbidden` (already exists) | — |
| Theme 4 (API key) | `ConfigError` | — |
| Theme 5 (Rate limit) | N/A — changes key format, not error type | — |
| Theme 6 (Config) | `ConfigError` (for invalid timeout) | `StorageError` (if timeout causes embed failure) |
| Theme 7 (Silenced) | `StorageError` (for DB errors surfacing from `.optional()`) | — |
| Theme 8 (Casts) | `StorageError` (via `storage_err`) | — |
| Theme 9 (Unwrap) | `ConfigError` (missing provider = config problem) | — |
| Theme 10 (Graph) | N/A — algorithmic fix, no new errors | — |
| Theme 11 (Misc) | `ConfigError` (unused deps → compile error) | — |

### 3.4 Error Propagation in Cleanup Paths

The `cleanup_partial` function in `engine/src/ingest.rs` is called in two contexts:

1. **Failure path** (`ingest_internal` line 191): Original error already captured. Cleanup failure is logged, not propagated.
2. **Retry path** (`retry_document` line ~150): Cleanup is the *primary* operation. Failure IS propagated via `?`.

This distinction is intentional: in the failure path, cleanup is best-effort. In the retry path, cleanup is the operation itself.

---

## 4. Testing Strategy

### 4.1 Per-Theme Test Plan

#### Theme 1: UTF-8 Bytes-vs-Chars

**Test type:** Unit tests in each affected crate.

| Test | Crate | Fixture | Assertion |
|------|-------|---------|-----------|
| `heading_parser_utf8_offsets` | graph | `"## 🎉\n\nSome text\n\n## 日本語"` | `char_offset` values match character positions, not byte positions |
| `heading_parser_accented_offsets` | graph | `"## Café\n\ncontent\n\n## Résumé"` | Same as above for accented Latin |
| `validator_truncate_multibyte` | ingest | 300-char emoji string `"🎉".repeat(300)` | No panic; result is 255 characters (not 255 bytes) |
| `validator_truncate_cjk_boundary` | ingest | 260 CJK characters | No panic; 255-char result is valid UTF-8 |
| `extractor_total_chars_non_ascii` | ingest | Temp file with `"日本語テスト"` (6 chars, 18 bytes) | `total_chars == 6` |
| `sentence_chunker_offsets_non_ascii` | ingest | `"Café con leche. Más café."` | `offset_start` and `offset_end` are character-based |
| `sentence_chunker_min_chars_non_ascii` | ingest | Short multi-byte sentences | Merge threshold uses character count |
| `char_len_basic` | common | `"hello"` = 5, `"日本語"` = 3, `"🎉🎊"` = 2 | Returns character count |
| `char_truncate_basic` | common | `"日本語テスト"` truncated to 3 | Returns `"日本語"` |
| `char_truncate_empty` | common | `""` truncated to 100 | Returns `""` |
| `char_truncate_exact` | common | String exactly 255 chars | Returns full string |

**Fixture data:**
```rust
const EMOJI: &str = "🎉🎊🎈🎁🎀";
const CJK: &str = "日本語テスト文字列";
const ACCENTED: str = "élégant café résumé naïve";
const MIXED: &str = "Hello 日本語 🎉 café";
```

#### Theme 2: FK Enforcement

**Test type:** Unit test in `storage/src/lib.rs`.

| Test | Fixture | Assertion |
|------|---------|-----------|
| `fk_enforcement_rejects_orphan_chunk` | Insert chunk with non-existent `document_id` | `Err(CiteError::StorageError)` containing "FOREIGN KEY" |
| `fk_pragma_returns_1` | `PRAGMA foreign_keys` query after `open_memory()` | Returns 1 (true) |
| `fk_allows_valid_insert` | Insert document, then insert chunk referencing it | `Ok(())` |

#### Theme 3: Production Mode Guard

**Test type:** Unit test in `cli/src/commands/ingest.rs` or integration test.

| Test | Config | Assertion |
|------|--------|-----------|
| `ingest_blocked_in_production` | `RuntimeMode::Production` | Returns `RuntimeModeForbidden` error |
| `ingest_blocked_in_public_demo` | `RuntimeMode::PublicPackagedDemo` | Returns `RuntimeModeForbidden` error |
| `ingest_allowed_in_local_demo` | `RuntimeMode::LocalPrivateDemo` | Proceeds (may fail on missing file, but not on guard) |

**Testing approach:** The guard call in `cli/src/commands/ingest.rs` can be tested by mocking the config. However, since the function uses `CommandContext::open()` which does real I/O, an integration test is more appropriate. Alternatively, extract the guard check into a testable helper.

#### Theme 4: Empty API Key

**Test type:** Unit tests in each provider + CLI.

| Test | Crate | Assertion |
|------|-------|-----------|
| `gemini_rejects_empty_key` | providers | `GeminiProvider::new("model", "")` → `Err(ConfigError)` |
| `openai_rejects_empty_key` | providers | `OpenAICompatibleProvider::new("https://x", "model", "")` → `Err(ConfigError)` |
| `gemini_accepts_nonempty_key` | providers | `GeminiProvider::new("model", "key")` → `Ok` |
| `create_provider_no_key_returns_error` | cli | `create_provider` with no env vars and no config key → `Err(ConfigError)` with message mentioning `CITE_API_KEY` |

#### Theme 5: Rate Limit Composite Key

**Test type:** Unit test in `engine/src/retrieve.rs`.

| Test | Assertion |
|------|-----------|
| `rate_limit_key_includes_model_id` | `rate_limit_key(&provider)` returns `"gemini:gemini-embedding-001"` format |
| `rate_limit_key_different_models_differ` | Two providers with same `provider_id` but different `model_id` produce different keys |

**Note:** `rate_limit_key` is currently `fn` (not `pub`). It may need to be `pub(crate)` for testing, or tested indirectly through `enforce_rate_limit`.

#### Theme 6: Config-Disconnect

**Test type:** Unit tests in config + integration test.

| Test | Crate | Assertion |
|------|-------|-----------|
| `provider_uses_configured_timeout` | providers | `GeminiProvider::new("model", "key", 60)` → client timeout is 60s |
| `config_default_max_chunk_chars` | config | `IngestConfig::default().max_chunk_chars == 1500` |
| `env_timeout_overrides_default` | config | `CITE_EMBEDDING_TIMEOUT=60` → `config.ingest.embedding_timeout_secs == 60` |
| `consolidated_min_chunk_chars` | config | No `min_chunk_size_chars` field; only `min_chunk_chars` |

#### Theme 7: Silenced Errors

**Test type:** Unit test in storage.

| Test | Fixture | Assertion |
|------|---------|-----------|
| `activate_snapshot_surfaces_db_error` | Corrupt or locked DB during `activate_snapshot` | `Err(CiteError::StorageError)` (not `None`) |
| `activate_snapshot_returns_none_when_no_pointer` | Fresh DB with no snapshot_pointer row | `Ok(None)` |

#### Theme 8: Integer Cast Safety

**Test type:** Unit test in storage.

| Test | Fixture | Assertion |
|------|---------|-----------|
| `row_to_chunk_rejects_overflow_index` | Mock row with `chunk_index = u32::MAX as i64 + 1` | `Err(CiteError::StorageError)` |
| `row_to_chunk_accepts_valid_index` | Mock row with `chunk_index = 42` | `chunk.chunk_index == 42` |

**Testing approach:** The `row_to_chunk` function takes a `rusqlite::Row`, which is hard to mock directly. Options: (a) test indirectly through `insert_chunk` + `get_chunk` with boundary values, (b) refactor `row_to_chunk` to take a trait. Option (a) is pragmatic for this pass.

#### Theme 9: Provider Unwrap

**Test type:** Unit test on `CommandContext`.

| Test | Fixture | Assertion |
|------|---------|-----------|
| `provider_returns_error_when_none` | `CommandContext { db, provider: None }` | `.provider()` → `Err(ConfigError)` |
| `provider_returns_ref_when_some` | `CommandContext { db, provider: Some(...) }` | `.provider()` → `Ok(&dyn EmbeddingProvider)` |

#### Theme 10: Graph Robustness

**Test type:** Unit test in `graph/src/hierarchy.rs`.

| Test | Fixture | Assertion |
|------|---------|-----------|
| `duplicate_h2_headings_assigned_correctly` | Headings: `[H2 "Overview" @0, H2 "API" @100, H2 "Overview" @200]` with chunks at `[10, 110, 210]` | First chunk → first "Overview", third chunk → second "Overview" |
| `indented_code_fence_toggled` | Markdown with `    ` ` ` ` (4-space indented fence)` | Headings inside indented code blocks are ignored |

#### Theme 11: Misc

| Test | Crate | Assertion |
|------|-------|-----------|
| `cite_error_partial_eq` | common | `CiteError::ConfigError { .. } == CiteError::ConfigError { .. }` with same message |
| `evaluate_uses_args_json` | cli | (Manual verification that `execute` reads `args.json`, not a separate param) |
| `providers_compiles_without_tokio_tracing` | providers | `cargo build -p providers` succeeds |

### 4.2 Integration vs Unit Test Decisions

| Theme | Unit | Integration | Rationale |
|-------|------|-------------|-----------|
| 1. UTF-8 | ✅ All fixes | ❌ | Pure computation, no I/O |
| 2. FK | ✅ In-memory DB | ❌ | `open_memory()` provides full SQLite |
| 3. Guard | ❌ | ✅ Needs config + mode | Guard is at CLI boundary |
| 4. API key | ✅ Providers | ✅ CLI `create_provider` | Provider tests are self-contained; CLI test verifies error message |
| 5. Rate limit | ✅ Key format | ❌ | Pure string formatting |
| 6. Config | ✅ Config struct | ❌ Provider HTTP | Config tests verify struct fields; timeout is tested via construction |
| 7. Silenced | ✅ In-memory DB | ❌ | `open_memory()` + manual query |
| 8. Casts | ✅ In-memory DB | ❌ | Insert boundary values, read back |
| 9. Unwrap | ✅ CommandContext | ❌ | Direct method call |
| 10. Graph | ✅ Heading fixtures | ❌ | Pure computation |
| 11. Misc | ✅ derive test | ❌ | Single-line change |

### 4.3 How to Test Without Real API Keys

**Provider tests** (Themes 4, 6) test *construction*, not API calls:
- `GeminiProvider::new("model", "key")` → succeeds (no HTTP)
- `GeminiProvider::new("model", "")` → fails with `ConfigError` (no HTTP)
- `provider.embed("text")` is NOT called in these tests

**Engine tests** use the existing `TestProvider` (defined in `engine/src/ingest.rs`):
```rust
struct TestProvider;
impl EmbeddingProvider for TestProvider {
    fn embed(&self, _text: &str) -> Result<Vec<f32>, CiteError> {
        Ok(vec![0.1, 0.2, 0.3])
    }
    fn model_id(&self) -> &str { "test-model" }
    fn provider_id(&self) -> &str { "test-provider" }
}
```

**CLI tests** (Theme 3 guard) test the guard logic, not the full pipeline. The guard function `check_ingest_allowed` is a pure function of `RuntimeMode` — no I/O involved.

### 4.4 Test Naming Conventions

Follow existing codebase conventions:
- `test_<function>_<scenario>` — e.g., `test_extract_headings_utf8_offsets`
- `test_<theme>_<specific_behavior>` — e.g., `test_fk_enforcement_rejects_orphan`
- All tests in `#[cfg(test)] mod tests { ... }` blocks within the file they test

### 4.5 CI Safety Net: UTF-8 Lint

Add a CI step (or document a manual check) to catch future `.len()` misuse in offset/truncation contexts:

```bash
# Catch suspicious .len() usage in offset/char-counting contexts
grep -rn '\.len()' crates/*/src/ --include='*.rs' \
  | grep -v '#\[cfg(test)\]' \
  | grep -v '// safe:' \
  | grep -iE 'offset|char|count|position|truncat' \
  && echo "WARNING: Review .len() usage in offset/char contexts" \
  || true
```

This is a best-effort lint, not a compile-time guarantee. Document it in `CONTRIBUTING.md` as a review checklist item.

---

## 5. Rollout Strategy

### 5.1 PR Merge Order

```
PR-1 (Data Integrity) ──→ PR-2 (Security) ──→ PR-3 (Config + Defensive)
     ↓                        ↓                      ↓
  ~50 lines               ~40 lines              ~195 lines
  4 crates                4 crates               7 crates
  Zero behavioral         Blocks prod            Largest but
  change for ASCII        deployment              lowest risk/change
```

**PR-1** is self-contained and has no dependencies on PR-2 or PR-3. It can be reviewed and merged independently.

**PR-2** depends on PR-1 only conceptually (both fix ingest pipeline issues). Technically independent — no code overlap.

**PR-3** has one soft dependency on PR-1: the config field rename (Theme 6) should happen *after* the chunker UTF-8 fix (Theme 1) to avoid fixing against confused field names. Since Theme 1 is in PR-1 and Theme 6 is in PR-3, the merge order handles this naturally.

### 5.2 Risk Mitigations Per PR

#### PR-1: Data Integrity (~50 lines)

| Risk | Likelihood | Impact | Mitigation |
|------|:----------:|:------:|-----------|
| Existing tests fail because they use ASCII-only data | Low | Low | ASCII `len() == chars().count()`, so existing tests produce identical results |
| FK enforcement surfaces orphan data in existing DBs | Low | Medium | SQLite only enforces on new writes; existing orphans persist silently |
| Missing a `len()` call | Low | High | CI grep check + manual audit of all `len()` in graph/ingest crates |
| `char_truncate` off-by-one | Low | Medium | Test with exact boundary lengths |

**Rollback plan:** Revert the single PR-1 commit. No data migration, no schema change (FK pragma is session-scoped, not persistent).

#### PR-2: Security + Onboarding (~40 lines)

| Risk | Likelihood | Impact | Mitigation |
|------|:----------:|:------:|-----------|
| Users in Production mode suddenly can't ingest | Medium | Low | This is the *intended behavior*. Document in CHANGELOG. |
| Rate limit key change resets counters | Low | Low | CLI counters are in-memory (reset on restart). Storage-persisted counters: acceptable for CLI tool. |
| Empty key error confuses users who relied on silent pass-through | Low | Low | Error message includes remediation step ("Set CITE_API_KEY...") |

**Rollback plan:** Revert the single PR-2 commit. The guard was previously dead code, so reverting restores the pre-fix behavior.

#### PR-3: Config + Defensive + Robustness (~195 lines)

| Risk | Likelihood | Impact | Mitigation |
|------|:----------:|:------:|-----------|
| Config field rename breaks existing env vars | Medium | Low | `CITE_MIN_CHUNK_SIZE_CHARS` → removed. Document migration. `CITE_MIN_CHUNK_CHARS` (already exists) takes over. |
| Previously silenced errors now surface | Medium | Low | `optional()` only surfaces *unexpected* DB errors (not "row not found"). In practice, DB errors in snapshot queries are extremely rare. |
| `try_from` rejects values that `as u32` silently accepted | Low | Medium | Only rejects values > 4.2B, which would never occur in practice for chunk indices, page numbers, or offsets. |
| Graph boundary fix changes chunk assignment | Medium | Medium | Only affects documents with duplicate heading titles. Test with such documents. |
| Provider constructor API change (timeout param) | Medium | Low | All call sites are in `cli/src/commands/mod.rs` — single location to update. |
| `CiteError::PartialEq` breaks existing pattern matching | Low | Low | Adding `PartialEq` doesn't change existing behavior — `matches!` still works. |

**Rollback plan:** Revert the PR-3 commit. The config field rename is the riskiest change — if it causes issues, the rollback restores the dual-field names. The silenced error changes are the second riskiest — reverting restores `.ok()` and `let _ =` patterns.

### 5.3 Merge Sequence Details

```
Step 1: Create branch error-remediation/pr-1-data-integrity
        - Commit: common helpers (char_len, char_truncate)
        - Commit: graph heading_parser UTF-8 fix
        - Commit: ingest UTF-8 fixes (validator, extractor, sentence_chunker)
        - Commit: tests for Theme 1
        - Commit: storage FK pragma
        - Commit: tests for Theme 2
        - Verify: cargo test && cargo clippy -- -D warnings && cargo fmt --check
        - PR-1 lines: ~50

Step 2: Create branch error-remediation/pr-2-security (from main, after PR-1 merges)
        - Commit: provider empty key validation
        - Commit: CLI api key error handling
        - Commit: runtime guard wiring
        - Commit: rate limit composite key
        - Commit: tests for Themes 3-5
        - Verify: cargo test && cargo clippy -- -D warnings && cargo fmt --check
        - PR-2 lines: ~40

Step 3: Create branch error-remediation/pr-3-config-defensive (from main, after PR-2 merges)
        - Commit: config field consolidation + timeout wiring
        - Commit: silenced error fixes (snapshots, cleanup)
        - Commit: integer cast safety
        - Commit: provider unwrap helper
        - Commit: graph robustness
        - Commit: misc fixes (evaluate, PartialEq, deps)
        - Commit: tests for Themes 6-11
        - Verify: cargo test && cargo clippy -- -D warnings && cargo fmt --check
        - PR-3 lines: ~195
```

### 5.4 Per-PR Quality Gates

Each PR must pass before merge:
```bash
cargo test                    # All existing + new tests pass
cargo clippy -- -D warnings   # No warnings
cargo fmt --check             # Formatting correct
# Lines changed < 400
```

---

## 6. Contracts

### 6.1 API Changes

#### 6.1.1 Provider Constructors (Theme 4 + Theme 6)

**Before:**
```rust
// GeminiProvider
pub fn new(model: &str, api_key: &str) -> Result<Self, CiteError>

// OpenAICompatibleProvider
pub fn new(endpoint: &str, model: &str, api_key: &str) -> Result<Self, CiteError>
```

**After:**
```rust
// GeminiProvider
pub fn new(model: &str, api_key: &str, timeout_secs: u64) -> Result<Self, CiteError>
// New precondition: api_key must not be empty → Err(ConfigError)

// OpenAICompatibleProvider
pub fn new(endpoint: &str, model: &str, api_key: &str, timeout_secs: u64) -> Result<Self, CiteError>
// New precondition: api_key must not be empty → Err(ConfigError)
```

**Breaking change:** Yes — all call sites must add the `timeout_secs` parameter. There is exactly one call site per provider (in `cli/src/commands/mod.rs::create_provider`).

**Alternative considered:** Builder pattern. Rejected — the constructors have 3-4 parameters, which is below the threshold where builders add value.

#### 6.1.2 CommandContext Helper (Theme 9)

**Before:**
```rust
// No provider() method — callers use:
ctx.provider.as_ref().unwrap()
```

**After:**
```rust
impl CommandContext {
    /// Get the embedding provider, returning an error if none was configured.
    pub fn provider(&self) -> Result<&dyn EmbeddingProvider, CiteError>
}
```

**Breaking change:** No — this is an addition. Callers that switch from `.unwrap()` to `.provider()?` change their error handling but not their type signatures.

#### 6.1.3 Config Field Rename (Theme 6)

**Before:**
```rust
pub struct IngestConfig {
    pub min_chunk_size_chars: usize,  // used by fixed-size chunker
    pub min_chunk_chars: usize,       // used by sentence chunker
    pub max_chunk_chars: usize,       // default 200
    // ...
}
```

**After:**
```rust
pub struct IngestConfig {
    // min_chunk_size_chars: REMOVED
    pub min_chunk_chars: usize,       // canonical field for all chunkers (default: 100)
    pub max_chunk_chars: usize,       // default: 1500
    // ...
}
```

**Breaking change:** Yes — `CITE_MIN_CHUNK_SIZE_CHARS` env var is removed. Users must switch to `CITE_MIN_CHUNK_CHARS`.

**Migration path:**
```bash
# Before (broken — env var had no effect on sentence chunker)
CITE_MIN_CHUNK_SIZE_CHARS=100

# After (canonical)
CITE_MIN_CHUNK_CHARS=100
```

#### 6.1.4 `CiteError` Derive Addition (Theme 11)

**Before:**
```rust
#[derive(Debug, thiserror::Error)]
pub enum CiteError { ... }
```

**After:**
```rust
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum CiteError { ... }
```

**Breaking change:** No — adding derives is always backward-compatible.

#### 6.1.5 `EmbeddingProvider` Trait (no change)

The `EmbeddingProvider` trait is unchanged. The timeout is passed at provider construction time, not per-call. This is the correct design because:
- HTTP clients are typically created once with a fixed timeout
- Changing timeout per-call would require `&mut self` on `embed()`
- The config value doesn't change during a CLI invocation

### 6.2 Database Contract Changes

**FK enforcement:** `PRAGMA foreign_keys = ON` is session-scoped (not persistent). It must be set on every `Database::open()` call. This means:
- In-memory test DBs also enforce FK constraints
- Existing production DBs are unaffected (orphan rows persist; only new writes are constrained)
- No migration required

### 6.3 Environment Variable Contract

| Variable | Status | Notes |
|----------|--------|-------|
| `CITE_API_KEY` | Unchanged | Still works; error message now mentions it |
| `CITE_EMBEDDING_TIMEOUT` | **Now effective** | Previously ignored; now sets provider HTTP timeout |
| `CITE_MIN_CHUNK_SIZE_CHARS` | **Removed** | Use `CITE_MIN_CHUNK_CHARS` instead |
| `CITE_MIN_CHUNK_CHARS` | Unchanged | Now canonical for all chunkers |
| `CITE_MAX_CHUNK_CHARS` | Unchanged | Default changed from 200 to 1500 |
| `CITE_RUNTIME_MODE` | Unchanged | Now actually enforced for ingest |

### 6.4 Backward Compatibility Matrix

| Change | Old behavior | New behavior | Migration needed |
|--------|-------------|-------------|:---:|
| UTF-8 fixes | Broken for non-ASCII | Correct for all text | No |
| FK enforcement | Silently accepts orphans | Rejects orphans on new writes | No |
| Production guard | Ingest works everywhere | Ingest blocked in Production | No (intended) |
| Empty key | Silent 401 later | Clear error immediately | No |
| Rate limit key | `provider_id` | `provider_id:model_id` | No |
| Provider timeout | Hardcoded 30s | Configurable | No |
| Config field rename | `min_chunk_size_chars` + `min_chunk_chars` | `min_chunk_chars` only | **Yes** (env var) |
| Max chunk default | 200 | 1500 | No |
| Silenced errors | Errors swallowed | Errors logged/propagated | No |
| Integer casts | Silent truncation | Error on overflow | No |
| Provider unwrap | Panic on failure | Graceful error | No |
| Graph robustness | Wrong assignment for duplicates | Correct assignment | No |
| `PartialEq` on `CiteError` | Not available | Available | No |
| Unused deps | `tokio`, `tracing` in providers | Removed | No |

---

## Appendix A: Line Budget Per PR

| PR | Theme | Production Lines | Test Lines | Total |
|----|-------|:---:|:---:|:---:|
| PR-1 | 1. UTF-8 helpers | ~8 | ~25 | ~33 |
| PR-1 | 1. heading_parser | ~2 | ~10 | ~12 |
| PR-1 | 1. validator | ~2 | ~8 | ~10 |
| PR-1 | 1. extractor | ~2 | ~5 | ~7 |
| PR-1 | 1. sentence_chunker | ~3 | ~5 | ~8 |
| PR-1 | 2. FK pragma | ~4 | ~8 | ~12 |
| **PR-1 Total** | | **~21** | **~61** | **~82** |
| PR-2 | 4. API key (providers) | ~6 | ~10 | ~16 |
| PR-2 | 4. API key (CLI) | ~2 | ~5 | ~7 |
| PR-2 | 3. Guard wiring | ~4 | ~8 | ~12 |
| PR-2 | 5. Rate limit key | ~1 | ~5 | ~6 |
| **PR-2 Total** | | **~13** | **~28** | **~41** |
| PR-3 | 6. Config consolidation | ~15 | ~15 | ~30 |
| PR-3 | 6. Timeout wiring | ~8 | ~5 | ~13 |
| PR-3 | 7. Silenced errors | ~8 | ~8 | ~16 |
| PR-3 | 8. Integer casts | ~8 | ~5 | ~13 |
| PR-3 | 9. Provider unwrap | ~8 | ~8 | ~16 |
| PR-3 | 10. Graph robustness | ~10 | ~15 | ~25 |
| PR-3 | 11. Misc | ~10 | ~5 | ~15 |
| **PR-3 Total** | | **~67** | **~61** | **~128** |
| **Grand Total** | | **~101** | **~150** | **~251** |

All PRs are well under the 400-line budget.

---

## Appendix B: Files Modified Per PR

### PR-1 (Data Integrity)
- `crates/common/src/lib.rs` — add `char_len`, `char_truncate`
- `crates/graph/src/heading_parser.rs` — `line.len()` → `line.chars().count()`
- `crates/ingest/src/validator.rs` — `trimmed[..255]` → `chars().take(255)`
- `crates/ingest/src/extractor.rs` — `content.len()` → `content.chars().count()`
- `crates/ingest/src/sentence_chunker.rs` — `current_text.len()` → `current_text.chars().count()`
- `crates/storage/src/lib.rs` — add FK pragma

### PR-2 (Security + Onboarding)
- `crates/providers/src/gemini.rs` — empty key check
- `crates/providers/src/openai.rs` — empty key check
- `crates/cli/src/commands/mod.rs` — `unwrap_or_default()` → `ok_or(ConfigError)`
- `crates/cli/src/commands/ingest.rs` — wire `check_ingest_allowed`
- `crates/engine/src/retrieve.rs` — composite rate limit key

### PR-3 (Config + Defensive + Robustness)
- `crates/config/src/lib.rs` — field consolidation, default change
- `crates/providers/src/gemini.rs` — accept `timeout_secs` param
- `crates/providers/src/openai.rs` — accept `timeout_secs` param
- `crates/cli/src/commands/mod.rs` — pass timeout to providers
- `crates/storage/src/snapshots.rs` — `.ok()` → `.optional()`
- `crates/engine/src/ingest.rs` — log cleanup errors
- `crates/storage/src/util.rs` — `as u32` → `try_from`
- `crates/storage/src/embeddings.rs` — `as u32` → `try_from`
- `crates/cli/src/commands/context.rs` — `.unwrap()` → `.provider()?`
- `crates/cli/src/commands/ingest.rs` — `.unwrap()` → `.provider()?`
- `crates/cli/src/commands/retrieve.rs` — `.unwrap()` → `.provider()?`
- `crates/graph/src/hierarchy.rs` — sequential heading matching
- `crates/graph/src/heading_parser.rs` — indented fence detection
- `crates/cli/src/commands/evaluate.rs` — dead `json` param
- `crates/common/src/error.rs` — `PartialEq` derive
- `crates/providers/Cargo.toml` — remove tokio/tracing
