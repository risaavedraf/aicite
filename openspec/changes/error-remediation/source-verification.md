# Source Verification: SDD Spec vs Actual Code

**Generated:** 2026-06-02
**Scope:** Verify exact file:line references for 11 error-remediation themes.
**Note:** All file paths are relative to project root. Actual paths use `crates/` prefix.

---

## THEME 1: UTF-8 bytes-vs-chars

### 1a. `crates/graph/src/heading_parser.rs` — char_offset uses `line.len()`

| Claim | Actual |
|---|---|
| Lines 17, 35: `char_offset += line.len()` | **Lines 17, 37** |

**Line 17** (inside code-block toggle branch):
```rust
char_offset += line.len() + 1; // +1 for newline
```

**Line 37** (after heading/non-heading processing):
```rust
char_offset += line.len() + 1; // +1 for newline
```

**Verdict:** ✅ CONFIRMED (line numbers off by 2 on second occurrence). `line.len()` returns byte count, not char count. For multi-byte UTF-8 content, `char_offset` will be wrong.

**Proposed fix:** Replace `line.len()` with `line.chars().count()` on both lines.

---

### 1b. `crates/ingest/src/validator.rs` — byte-slice truncation

| Claim | Actual |
|---|---|
| Lines 97-98: `trimmed[..255]` byte-slice | **Lines 95-101** |

```rust
// lines 95-101
if trimmed.len() > 255 {
    trimmed[..255].to_string()
} else {
    trimmed.to_string()
}
```

**Verdict:** ✅ CONFIRMED (line range slightly different). `trimmed.len()` is byte length, and `trimmed[..255]` is a byte slice that will panic on multi-byte char boundaries. Earlier filtering uses `.chars().filter()` correctly, but the final truncation is byte-based.

**Proposed fix:** Use `trimmed.chars().take(255).collect::<String>()` instead of `trimmed[..255]`.

---

### 1c. `crates/ingest/src/extractor.rs` — `total_chars = content.len()`

| Claim | Actual |
|---|---|
| Line 37: `total_chars = content.len()` | **Line 37** ✅ EXACT |

```rust
let total_chars = content.len();
```

Also at **line 75** in `extract_pdf_text`:
```rust
total_chars += text.len();
```

**Verdict:** ✅ CONFIRMED. `content.len()` returns byte count, not Unicode char count.

**Proposed fix:** Replace `content.len()` with `content.chars().count()` on line 37, and `text.len()` with `text.chars().count()` on line 75.

---

### 1d. `crates/ingest/src/sentence_chunker.rs` — `current_text.len()` for thresholds and offsets

| Claim | Actual |
|---|---|
| Line 42: `current_text.len() < min_chars` | **Line 42** ✅ EXACT |
| Lines 47-48: `current_text.len()` for offset_end | **Line 47** ✅ EXACT (line 48 is the `chunks.push(`) |
| Lines 58-59: `current_text.len()` for offset_end | **Line 57** (offset_end), **line 58** is blank |

```rust
// line 42
} else if current_text.len() < min_chars {
    // line 47
    let offset_end = current_offset_start + current_text.len();

// line 57 (flush remaining)
let offset_end = current_offset_start + current_text.len();
```

**Verdict:** ✅ CONFIRMED (minor line-number drift on second occurrence). All three uses of `.len()` are byte-count on potentially multi-byte text.

**Proposed fix:** Replace all three `current_text.len()` with `current_text.chars().count()`.

---

### 1e. `crates/common/src/lib.rs` — char_len/char_truncate helpers

| Claim | Actual |
|---|---|
| Check if char_len/char_truncate helpers exist | **NOT PRESENT** |

```rust
pub mod error;
pub mod exit;
pub mod types;

pub use error::CiteError;
pub use exit::ExitCode;
pub use types::{...};
```

**Verdict:** ❌ NO HELPERS EXIST. `common/src/lib.rs` has no `char_len` or `char_truncate` utility functions. The crate only re-exports error, exit, and types modules.

**Proposed fix:** Add helper functions to `common/src/lib.rs` (or a new `common/src/str_util.rs` module):
```rust
pub fn char_len(s: &str) -> usize { s.chars().count() }
pub fn char_truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}
```

---

## THEME 2: FK enforcement

### `crates/storage/src/lib.rs` — WAL mode, no FK pragma

| Claim | Actual |
|---|---|
| Lines 34-40: WAL mode, no FK pragma | **Lines 34-41** |

```rust
// line 34-37
conn.pragma_update(None, "journal_mode", "WAL")
    .map_err(|e| CiteError::StorageError {
        message: format!("Failed to set WAL mode: {e}"),
    })?;

// line 39-41
conn.pragma_update(None, "busy_timeout", 5000)
    .map_err(|e| CiteError::StorageError {
        message: format!("Failed to set busy timeout: {e}"),
    })?;
```

No `PRAGMA foreign_keys = ON` anywhere in the codebase (verified via grep). SQL migrations define `FOREIGN KEY` constraints (e.g., `002_trace_citations.sql:14`, `006_hierarchy.sql`), but SQLite does not enforce them without the pragma.

**Verdict:** ✅ CONFIRMED. WAL and busy_timeout pragmas are set; FK enforcement pragma is missing. FK constraints exist in DDL but are inert.

**Proposed fix:** Add after line 41:
```rust
conn.pragma_update(None, "foreign_keys", "ON")
    .map_err(|e| CiteError::StorageError {
        message: format!("Failed to enable foreign keys: {e}"),
    })?;
```

---

## THEME 3: Production mode guard

### 3a. `crates/cli/src/commands/ingest.rs` — production_mode only controls display name

| Claim | Actual |
|---|---|
| Lines 60-67 | **Line 67** |

```rust
// line 67
let production_mode = config.runtime.mode == config::RuntimeMode::Production;
```

`production_mode` is computed and passed to `ingest::ingest()` and `ingest::ingest_next()`, where it reaches `validator::derive_display_name()` — which only changes the display name to `"document"` when true.

**Verdict:** ✅ CONFIRMED. `production_mode` affects display name only; no security/access guard.

---

### 3b. `crates/engine/src/ingest.rs` — production_mode parameter threaded but not guarded

| Claim | Actual |
|---|---|
| Lines 47, 74, 119 | **Line 47** (fn `ingest`), **Line 74** (fn `ingest_next`), **Line 119** (fn `ingest_internal`) |

```rust
// line 47: pub fn ingest(..., production_mode: bool)
// line 74: pub fn ingest_next(..., production_mode: bool)
// line 119: fn ingest_internal(..., production_mode: bool, ...)
```

`production_mode` flows to `validator::derive_display_name()` only.

**Verdict:** ✅ CONFIRMED. No guard call.

---

### 3c. `crates/engine/src/runtime_guard.rs` — `check_ingest_allowed()` defined, never called

| Claim | Actual |
|---|---|
| Lines 23-34 | **Lines 23-34** ✅ EXACT |

```rust
pub fn check_ingest_allowed(mode: &RuntimeMode) -> Result<(), CiteError> {
    match mode {
        RuntimeMode::LocalPrivateDemo => Ok(()),
        RuntimeMode::PublicPackagedDemo => Err(CiteError::RuntimeModeForbidden { ... }),
        RuntimeMode::Production => Err(CiteError::RuntimeModeForbidden { ... }),
    }
}
```

Grep for `check_ingest_allowed` in the entire codebase only finds the definition — zero call sites.

**Verdict:** ✅ CONFIRMED. Dead code.

**Proposed fix:** Call `runtime_guard::check_ingest_allowed(&config.runtime.mode)?` at the start of `ingest()` and `ingest_next()` in `crates/engine/src/ingest.rs`.

---

## THEME 4: Empty API key

### 4a. `crates/cli/src/commands/mod.rs` — `.unwrap_or_default()` on API key

| Claim | Actual |
|---|---|
| Line 94 | **Line 94** ✅ EXACT |

```rust
let api_key = resolve_api_key(config).unwrap_or_default();
```

`resolve_api_key` returns `Option<String>`, so `.unwrap_or_default()` yields `""` when no key is configured.

**Verdict:** ✅ CONFIRMED.

---

### 4b. `crates/providers/src/gemini.rs` — accepts empty string

| Claim | Actual |
|---|---|
| Line 24 | **Line 24** ✅ EXACT |

```rust
pub fn new(model: &str, api_key: &str) -> Result<Self, CiteError> {
```

No validation that `api_key` is non-empty. An empty string passes through to the `x-goog-api-key` header.

**Verdict:** ✅ CONFIRMED.

**Proposed fix:** Add at the start of `new()`:
```rust
if api_key.is_empty() {
    return Err(CiteError::ConfigError {
        message: "API key must not be empty".to_string(),
    });
}
```

---

### 4c. `crates/providers/src/openai.rs` — accepts empty string

| Claim | Actual |
|---|---|
| Line 29 | **Line 29** ✅ EXACT |

```rust
pub fn new(endpoint: &str, model: &str, api_key: &str) -> Result<Self, CiteError> {
```

No empty-key check. The HTTPS endpoint check exists (line 31), but not a key check.

**Verdict:** ✅ CONFIRMED.

**Proposed fix:** Same as gemini.rs — reject empty `api_key`.

---

## THEME 5: Rate limit composite key

### `crates/engine/src/retrieve.rs` — rate_limit_key = provider.provider_id()

| Claim | Actual |
|---|---|
| Lines 273-275 | **Lines 278-280** |

```rust
fn rate_limit_key(provider: &dyn EmbeddingProvider) -> String {
    provider.provider_id().to_string()
}
```

**Verdict:** ✅ CONFIRMED (line numbers off by 5). The rate-limit key uses only `provider_id()` (e.g., `"gemini"`, `"openai-compatible"`). This means two different models on the same provider share the same rate limit bucket. With route also passed to `enforce_rate_limit()`, the composite key is effectively `(route, provider_id)` but lacks model specificity.

**Proposed fix:** Use `format!("{}:{}", provider.provider_id(), provider.model_id())`.

---

## THEME 6: Config-disconnect

### 6a. Hardcoded 30s timeouts

**`crates/providers/src/gemini.rs` line 28:**
```rust
.timeout(std::time::Duration::from_secs(30))
```

**`crates/providers/src/openai.rs` line 34:**
```rust
.timeout(std::time::Duration::from_secs(30))
```

**`crates/config/src/lib.rs` line 106:**
```rust
pub embedding_timeout_secs: u64,
```

Config has `embedding_timeout_secs` (default 30, env `CITE_EMBEDDING_TIMEOUT`), but providers hardcode 30 instead of reading from config.

**Verdict:** ✅ CONFIRMED.

**Proposed fix:** Pass `config.ingest.embedding_timeout_secs` through to provider constructors.

---

### 6b. Field name confusion

**`crates/config/src/lib.rs`** has both:
- `min_chunk_size_chars` (line 93, in `IngestConfig` struct) — used by fixed-size chunker
- `min_chunk_chars` (line 108) — used by sentence chunker
- `max_chunk_chars` (line 111) — used by sentence chunker

**`crates/engine/src/ingest.rs` line 221** passes `config.min_chunk_size_chars` to `chunker::chunk_text()`.

**`crates/ingest/src/chunker.rs` line 30** parameter is named `min_chunk_size_chars`.

The sentence chunker (`sentence_chunker.rs`) takes a `min_chars` parameter but is not called from `engine/src/ingest.rs` — the engine only uses fixed-size chunking. The `sentence_chunking` config flag exists but is not wired into the engine pipeline.

**Verdict:** ✅ CONFIRMED. Three similarly-named fields (`min_chunk_size_chars`, `min_chunk_chars`, `max_chunk_chars`) exist, creating confusion. Only `min_chunk_size_chars` is used in the actual pipeline.

---

## THEME 7: Silenced errors

### 7a. `crates/storage/src/snapshots.rs` — `.ok()` converts ALL errors to None

| Claim | Actual |
|---|---|
| Lines 68-73 | **Lines 68-73** ✅ EXACT |

```rust
// lines 68-73
let previous_snapshot_id: Option<String> = tx
    .query_row(
        "SELECT active_snapshot_id FROM snapshot_pointer WHERE id = 1",
        [],
        |row| row.get(0),
    )
    .ok();
```

`.ok()` on `Result<Option<String>, Error>` converts `Err(e)` to `None`, silently swallowing database errors (not just "row not found").

**Verdict:** ✅ CONFIRMED.

**Proposed fix:** Use `.optional()` from `rusqlite::OptionalExtension` (already imported) and match on the error separately:
```rust
let previous_snapshot_id: Option<String> = tx
    .query_row(...)
    .optional()
    .map_err(storage_err)?;
```

---

### 7b. `crates/engine/src/ingest.rs` — `let _ = cleanup_partial()` errors

| Claim | Actual |
|---|---|
| Lines 215-220 | **Lines 191-193** |

```rust
// lines 191-193 (inside the failure branch of match run_pipeline)
let _ = cleanup_partial(db, &document_id);
let error_info = ErrorInfo { ... };
let _ = db.update_document_status(&document_id, DocumentStatus::Failed, Some(error_info));
```

Also at **lines 246-247** inside `cleanup_partial`:
```rust
let _ = db.delete_embeddings_for_document(document_id);
let _ = db.delete_chunks_for_document(document_id);
```

**Verdict:** ✅ CONFIRMED (line numbers differ significantly from SDD claim of 215-220; actual is 191-193 and 246-247). Cleanup failures and the final status-update failure are both discarded.

**Proposed fix:** Log cleanup failures at minimum:
```rust
if let Err(e) = cleanup_partial(db, &document_id) {
    eprintln!("Warning: cleanup failed for {document_id}: {e}");
}
```

---

## THEME 8: Integer cast safety

### 8a. `crates/storage/src/util.rs` — `val as u32`

| Claim | Actual |
|---|---|
| Lines 37, 42, 47 | **Lines 37, 42, 47** ✅ EXACT |

```rust
// line 37
chunk_index: row.get::<_, i64>("chunk_index").map_err(storage_err)? as u32,

// line 42
.map(|v| v as u32),   // page

// line 47
.map(|v| v as u32),   // offset_start (line 47) and offset_end (line 51)
```

**Verdict:** ✅ CONFIRMED. All `i64 → u32` casts are unchecked. A value > `u32::MAX` would silently truncate.

**Proposed fix:** Use `u32::try_from(v).map_err(...)` or `.try_into()` with error handling.

---

### 8b. `crates/storage/src/embeddings.rs` — `val as u32`

| Claim | Actual |
|---|---|
| Lines 144, 148, 155 | **Lines 144, 148, 152, 155** |

In `list_chunk_embeddings_hierarchical`:
```rust
// line 144
chunk_index: row.get::<_, i64>(4).map_err(storage_err)? as u32,

// line 148
.map(|v| v as u32),  // page

// lines 152, 155
.map(|v| v as u32),  // offset_start
.map(|v| v as u32),  // offset_end
```

Same pattern in `list_ready_chunk_embeddings` (lines ~209, 213, 217, 220).

**Verdict:** ✅ CONFIRMED. Same unchecked `i64 → u32` casts as util.rs.

**Proposed fix:** Same `try_from` approach as util.rs.

---

## THEME 9: Provider unwrap

### 9a. `crates/cli/src/commands/context.rs` line 50

```rust
let provider = ctx.provider.as_ref().unwrap();
```

**Verdict:** ✅ CONFIRMED.

---

### 9b. `crates/cli/src/commands/ingest.rs` line 66

```rust
let provider = ctx.provider.as_ref().unwrap();
```

**Verdict:** ✅ CONFIRMED.

---

### 9c. `crates/cli/src/commands/retrieve.rs` line 80

```rust
let provider = ctx.provider.as_ref().unwrap();
```

**Verdict:** ✅ CONFIRMED (line 80 in retrieve.rs).

---

All three commands use `CommandContext::open()` which always sets `provider: Some(...)`, so the unwrap is currently safe. However, if `open()` logic changes, all three would panic.

**Proposed fix:** Either use `ctx.provider.as_ref().ok_or_else(|| ...)` with a graceful error, or add a helper method on `CommandContext` that returns a `Result<&dyn EmbeddingProvider, CiteError>`.

---

## THEME 10: Graph robustness

### 10a. `crates/graph/src/hierarchy.rs` — `find()` matches first occurrence

| Claim | Actual |
|---|---|
| Lines 128-148 | **Lines 130-146** |

```rust
// lines 130-146
for (t_idx, topic_with_concepts) in topics.iter().enumerate() {
    if let Some(heading) = headings
        .iter()
        .find(|h| h.level == 2 && h.title == topic_with_concepts.topic.name)   // ← first match
    {
        boundaries.push((heading.char_offset, t_idx, None));
        for (c_idx, concept_with_chunks) in topic_with_concepts.concepts.iter().enumerate() {
            if let Some(h) = headings
                .iter()
                .find(|h| h.level == 3 && h.title == concept_with_chunks.concept.name)  // ← first match
            {
                boundaries.push((h.char_offset, t_idx, Some(c_idx)));
            }
        }
    }
}
```

**Verdict:** ✅ CONFIRMED. `.find()` returns the first matching heading. If duplicate heading titles exist (e.g., two `## API` sections), only the first occurrence's offset is used, causing incorrect chunk-to-topic assignment for the second.

**Proposed fix:** Use heading index tracking instead of title matching. Maintain an iterator position over the headings list matched to the sequential topic/concept creation order.

---

### 10b. `crates/graph/src/heading_parser.rs` — `starts_with("```")`

| Claim | Actual |
|---|---|
| Lines 14-16 | **Lines 14-16** ✅ EXACT |

```rust
// lines 14-16
if trimmed.starts_with("```") {
    in_code_block = !in_code_block;
    char_offset += line.len() + 1; // +1 for newline
    continue;
}
```

This toggle is correct for raw `` ``` `` fences but:
- `starts_with("```")` matches `` ```rust ``, `` ```python ``, etc. (correct — toggles on any fence).
- The toggle `!in_code_block` assumes proper nesting, but will break if a document has an odd number of ``` lines (e.g., a malformed file).

**Verdict:** ✅ CONFIRMED. The toggle approach is fragile but standard for simple markdown parsing.

**Proposed fix (optional):** This is low-risk for well-formed markdown. Consider matching the closing fence to the opening one only if robustness is critical.

---

## THEME 11: Misc

### 11a. `crates/cli/src/commands/evaluate.rs` — `--json` flag duplicated

| Claim | Actual |
|---|---|
| Lines 14-18 | **Lines 14-16** |

```rust
// lines 14-16
#[derive(clap::Args)]
pub struct EvaluateArgs {
    /// Output results as JSON
    #[arg(long)]
    pub json: bool,
}
```

But `execute()` signature (line 250):
```rust
pub fn execute(_args: &EvaluateArgs, _config: &Config, json: bool) -> i32 {
```

The `json` parameter is passed separately, ignoring `args.json`. The `--json` flag exists in `EvaluateArgs` but is never read from it.

**Verdict:** ✅ CONFIRMED. The `args.json` field is dead; the actual `json` comes from the function parameter.

**Proposed fix:** Remove `json: bool` from the `execute` signature and use `args.json` instead, or remove the field from `EvaluateArgs`.

---

### 11b. `crates/common/src/error.rs` — `CiteError` no `PartialEq`

| Claim | Actual |
|---|---|
| Line 7 | **Line 7** ✅ EXACT |

```rust
#[derive(Debug, thiserror::Error)]
pub enum CiteError {
```

Only `Debug` and `thiserror::Error` are derived. No `PartialEq`. This makes test assertions verbose (requires `matches!` macro or manual pattern matching).

**Verdict:** ✅ CONFIRMED.

**Proposed fix:** Add `PartialEq` to the derive list. Note: `PathBuf` implements `PartialEq`, so all variants are eligible. The `OperationInProgress` variant has `Option<String>` which is also `PartialEq`.

---

### 11c. `crates/providers/Cargo.toml` — tokio/tracing unused deps

| Claim | Actual |
|---|---|
| Lines 8-9 | **Lines 8-9** ✅ EXACT |

```toml
tokio = { workspace = true }
tracing = { workspace = true }
```

Verified via grep: zero occurrences of `tokio` or `tracing` in `crates/providers/src/`. The crate uses `reqwest::blocking::Client`, not async, and has no logging/tracing instrumentation.

**Verdict:** ✅ CONFIRMED. Both dependencies are unused.

**Proposed fix:** Remove lines 8-9 from `crates/providers/Cargo.toml`.

---

### 11d. `crates/retrieval/src/lib.rs` — ScoredChunk field duplication

| Claim | Actual |
|---|---|
| Lines 40-64 | **Lines 40-64** ✅ EXACT |

`ScoredChunk` duplicates fields from `ChunkEmbeddingRecord`:
- `chunk_id`, `document_id`, `display_name`, `section_id`, `chunk_index`, `text`, `page`, `offset_start`, `offset_end`

Plus adds: `score`, `topic_id`, `topic_name`, `concept_id`, `concept_name`.

The same duplication exists in `engine/src/retrieve.rs::Hit` and `cli/src/commands/retrieve.rs::RetrieveResultItem`.

**Verdict:** ✅ CONFIRMED. Three structs carry the same chunk metadata fields.

**Proposed fix (optional):** Embed `ChunkEmbeddingRecord` (or a shared metadata struct) inside `ScoredChunk` rather than copying each field. This reduces maintenance surface but changes the public API.

---

## Summary Table

| Theme | Claim | Actual Lines | Status |
|---|---|---|---|
| 1a. heading_parser.rs `len()` | 17, 35 | **17, 37** | ✅ confirmed (off by 2) |
| 1b. validator.rs byte-slice | 97-98 | **95-101** | ✅ confirmed (range differs) |
| 1c. extractor.rs `content.len()` | 37 | **37** | ✅ exact |
| 1d. sentence_chunker.rs `len()` | 42, 47-48, 58-59 | **42, 47, 57** | ✅ confirmed (minor drift) |
| 1e. common helpers | — | — | ❌ do not exist |
| 2. FK pragma | 34-40 | **34-41** | ✅ confirmed |
| 3a. production_mode display | 60-67 | **67** | ✅ confirmed |
| 3b. production_mode param | 47, 74, 119 | **47, 74, 119** | ✅ exact |
| 3c. check_ingest_allowed | 23-34 | **23-34** | ✅ exact |
| 4a. unwrap_or_default | 94 | **94** | ✅ exact |
| 4b. gemini empty key | 24 | **24** | ✅ exact |
| 4c. openai empty key | 29 | **29** | ✅ exact |
| 5. rate limit key | 273-275 | **278-280** | ✅ confirmed (off by 5) |
| 6a. hardcoded timeout | gemini 31, openai 34 | **gemini 28, openai 34** | ✅ confirmed |
| 6b. field name confusion | 94, 126-128 | **93, 108, 111** | ✅ confirmed |
| 7a. snapshots `.ok()` | 68-73 | **68-73** | ✅ exact |
| 7b. cleanup `let _` | 215-220 | **191-193** | ✅ confirmed (line differs) |
| 8a. util.rs `as u32` | 37, 42, 47 | **37, 42, 47** | ✅ exact |
| 8b. embeddings.rs `as u32` | 144, 148, 155 | **144, 148, 152, 155** | ✅ confirmed |
| 9a. context.rs unwrap | 50 | **50** | ✅ exact |
| 9b. ingest.rs unwrap | 66 | **66** | ✅ exact |
| 9c. retrieve.rs unwrap | 80 | **80** | ✅ exact |
| 10a. hierarchy `find()` | 128-148 | **130-146** | ✅ confirmed |
| 10b. heading_parser `` ``` `` | 14-16 | **14-16** | ✅ exact |
| 11a. evaluate `--json` dup | 14-18 | **14-16** | ✅ confirmed |
| 11b. CiteError no PartialEq | 7 | **7** | ✅ exact |
| 11c. unused deps | 8-9 | **8-9** | ✅ exact |
| 11d. ScoredChunk duplication | 40-64 | **40-64** | ✅ exact |
