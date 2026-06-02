# Code Review — Pending Items & Suggestions

**Date:** 2026-05-28
**Source:** Full code quality review with code-quality-review skill (Clean Code + Rust Idioms + GitHub Structure)

---

## Skipped Items (No Action Needed)

These items from the review were investigated and found to be already handled or not applicable:

### 1. `ingest_internal` refactor (119 lines)
**Status:** Already done
**Detail:** The codebase already has `run_pipeline()` extracted as a separate function handling the core pipeline (extraction → chunking → storage → embedding). `ingest_internal` only handles lock acquisition/release and error cleanup — which is the correct separation.

### 2. Unnecessary clones in citation building
**Status:** Structurally necessary
**Detail:** `build_citations_from_ranked` takes `&[ScoredChunk]` (borrowed) because `ranked` is needed afterward for `persist_trace` and `retrieved_chunks` count. Citation fields must be owned strings since `Citation` is a separate type returned in `ContextResponse`. Changing to `.into_iter()` would require extracting all dependent data first, adding complexity for no net gain.

### 3. Batch insert optimization in ingest
**Status:** Already done
**Detail:** The codebase already uses batch operations: `db.insert_chunks(document_id, &chunks)` and `db.insert_embeddings(&embeddings)`. Both APIs accept `&[...]` slices.

### 4. Document struct field grouping
**Status:** Too invasive
**Detail:** Splitting `Document` into sub-structs (`DocumentMetadata`, `ProcessingInfo`, `Timestamps`) would require changing every construction site across storage, ingest, and CLI crates. Not worth it for a suggestion-level improvement.

---

## Deferred Improvements (Future Work)

### 1. Newtype Migration — `DocumentId`, `ChunkId`, `TraceId`

**Priority:** Medium
**Effort:** Medium-Large (many files)
**Status:** Newtypes defined in `common/src/types.rs`, not yet used in call sites

The newtype wrappers are ready:
```rust
pub struct DocumentId(pub String);
pub struct ChunkId(pub String);
pub struct TraceId(pub String);
```
Each has `Display`, `From<String>`, `AsRef<str>`, `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `Serialize`, `Deserialize`.

**Migration path:**
1. Start with `storage/src/documents.rs` — change method signatures from `&str` to `&DocumentId`
2. Update `engine/src/ingest.rs` and `engine/src/context.rs` call sites
3. Update `cli/src/commands/*` to construct newtypes
4. Propagate to remaining crates

**Risk:** Compilation chain breaks across many files. Should be done incrementally, one crate at a time.

### 2. Fix Pre-existing `ResultKind` Use-After-Move

**Priority:** Low (pre-existing, not caused by review fixes)
**Effort:** Small (1-2 lines)
**File:** `crates/engine/src/context.rs` lines 318, 338

**Problem:** `ResultKind` is moved and then used again afterward.

**Fix options:**
- Add `.clone()` at line 299 before the move
- Or borrow `&result_kind` instead of consuming it

### 3. Improve Doc Test Coverage

**Priority:** Low
**Effort:** Small
**Current state:**
- 11 storage doc tests are `ignore` (require Database instance)
- 1 retrieval doc test is `ignore` (requires `ChunkEmbeddingRecord`)

**Improvement:**
- Change `ignore` to `no_run` if test helpers are added to construct `Database` in doc examples
- Add `#[doc(hidden)]` test utilities for doc test setup

---

## Completed Fixes (Applied 2026-05-28)

| Issue | Severity | Files | Status |
|-------|----------|-------|--------|
| unwrap() in production | Critical | trace.rs, search.rs | ✅ Fixed |
| build_context 212 lines | Warning | context.rs | ✅ Refactored |
| DRY violation API key | Warning | mod.rs, health.rs | ✅ Fixed |
| Doc comments retrieval | Suggestion | retrieval/src/lib.rs | ✅ Added |
| Doc comments graph | Suggestion | graph/src/lib.rs | ✅ Added |
| Doc comments storage | Suggestion | storage/src/documents.rs | ✅ Added |
| Doc comments common | Suggestion | common/src/types.rs | ✅ Added |
| Newtype definitions | Suggestion | common/src/types.rs | ✅ Defined |
| Doc examples | Suggestion | Multiple crates | ✅ Added |

**Validation:** 260 tests pass, 0 failures, 0 compiler warnings.
