# Phase 11 Tasks — Hierarchical Retrieval

## Slice A — Storage: Hierarchical Query

**Goal**: Add hierarchy-aware retrieval query to storage layer.

**Allowlist**:
- `crates/storage/src/embeddings.rs` (modify)
- `crates/storage/src/lib.rs` (re-export if needed)

**Estimated lines**: ~80

**Changes**:
1. Add `HierarchicalChunkEmbedding` struct with fields: `chunk: ChunkEmbeddingRecord`, `topic_id: Option<String>`, `topic_name: Option<String>`, `concept_id: Option<String>`, `concept_name: Option<String>`
2. Add `list_chunk_embeddings_hierarchical(&self, topic_filter: Option<&str>, concept_filter: Option<&str>) -> Result<Vec<HierarchicalChunkEmbedding>, CiteError>`
   - SQL: LEFT JOIN topics + concepts, filter by topic_id/concept_id when provided
3. Add `has_hierarchy_data(&self) -> Result<bool, CiteError>`
   - SQL: `SELECT EXISTS(SELECT 1 FROM chunks WHERE topic_id IS NOT NULL LIMIT 1)`
4. Unit tests for both functions

**Dependencies**: None

---

## Slice B — Config + Response Types

**Goal**: Add `use_hierarchy` to RetrievalConfig and breadcrumb fields to response types.

**Allowlist**:
- `crates/config/src/lib.rs` (modify)
- `crates/common/src/types.rs` (modify)

**Estimated lines**: ~60

**Changes**:
1. Add `pub use_hierarchy: bool` to `RetrievalConfig` (default: `true`)
2. Add to `Citation`: `pub topic_name: Option<String>`, `pub concept_name: Option<String>`, `pub breadcrumb: Option<String>`
3. Add to `SearchHit`: same three fields
4. Add to `RetrieveHit`: same three fields
5. Update `Default` impls and any constructors

**Dependencies**: None

---

## Slice C — Engine: Hierarchical Retrieval + Enrichment

**Goal**: Wire hierarchical retrieval into engine functions with breadcrumb enrichment.

**Allowlist**:
- `crates/engine/src/retrieve.rs` (modify)
- `crates/engine/src/context.rs` (modify)

**Estimated lines**: ~200

**Changes**:
1. Add helper: `fn build_breadcrumb(display_name, topic_name, concept_name) -> String`
2. Modify `search()` signature: add `topic_filter: Option<&str>`, `concept_filter: Option<&str>`
3. Modify `retrieve()` signature: same
4. Modify `build_context()` signature: same
5. In each function:
   - Check `config.use_hierarchy && db.has_hierarchy_data()`
   - If hierarchical: call `list_chunk_embeddings_hierarchical()`, rank, enrich with breadcrumb
   - If flat: use existing `list_ready_chunk_embeddings()`, breadcrumb = null
6. Update all internal callers of these functions

**Dependencies**: Slice A, Slice B

---

## Slice D — CLI Flags

**Goal**: Add --flat, --topic, --concept flags to retrieval commands.

**Allowlist**:
- `crates/cli/src/commands/context.rs` (modify)
- `crates/cli/src/commands/search.rs` (modify)
- `crates/cli/src/commands/retrieve.rs` (modify)

**Estimated lines**: ~100

**Changes**:
1. Add clap args to each command struct:
   - `#[arg(long)] flat: bool`
   - `#[arg(long)] topic: Option<String>`
   - `#[arg(long)] concept: Option<String>`
2. Add validation:
   - `--flat` + `--topic`/`--concept` → error
   - `--topic` + `--concept` → error
3. Resolve topic/concept name → ID (if not a UUID, search by name)
4. Pass `topic_filter`/`concept_filter` to engine functions
5. Set `use_hierarchy = false` when `--flat`

**Dependencies**: Slice C

---

## Slice E — Tests + Verification

**Goal**: Full test suite, backward compat verification.

**Allowlist**:
- All test files
- `openspec/changes/phase-11-hierarchical-retrieval/` (verification artifacts)

**Estimated lines**: ~200

**Changes**:
1. New tests:
   - Hierarchical retrieval with topic filter
   - Hierarchical retrieval with concept filter
   - Hierarchical retrieval without filter (breadcrumb enrichment)
   - Flat fallback when no hierarchy data
   - `--flat` produces null breadcrumb
   - CLI flag validation (conflict errors)
2. Run full suite: `cargo test`
3. Run lint: `cargo clippy -- -D warnings`
4. Run format: `cargo fmt --check`
5. Verify all 209 existing tests still pass

**Dependencies**: All previous slices

---

## Summary

| Slice | Description | Est. Lines | Depends On |
|-------|-------------|------------|------------|
| A | Storage hierarchical query | ~80 | — |
| B | Config + response types | ~60 | — |
| C | Engine retrieval + enrichment | ~200 | A, B |
| D | CLI flags | ~100 | C |
| E | Tests + verification | ~200 | All |
| **Total** | | **~640** | |

**Implementation order**: A → B → C → D → E

**Parallelization**: A and B can start simultaneously.
