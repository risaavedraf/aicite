# Phase 11 Design — Hierarchical Retrieval

## Architecture

### Modified retrieval flow

```
CLI (context/search/retrieve)
  │
  ├─ Parse flags: --flat, --topic, --concept
  │   ├─ Validate: --flat conflicts with --topic/--concept
  │   └─ Validate: --topic and --concept conflict with each other
  │
  ├─ Resolve topic/concept filters (name → ID lookup if needed)
  │
  └─ Engine function
       │
       ├─ Determine path: use_hierarchy = config.use_hierarchy && !flat_flag && db.has_hierarchy_data()
       │
       ├─ FLAT PATH (use_hierarchy == false):
       │    ├─ db.list_ready_chunk_embeddings()  ← unchanged
       │    ├─ rank_by_similarity(query_vector, candidates, k)
       │    └─ Return results with breadcrumb = null
       │
       └─ HIERARCHICAL PATH (use_hierarchy == true):
            ├─ db.list_chunk_embeddings_hierarchical(topic_filter, concept_filter)
            ├─ rank_by_similarity(query_vector, candidates, k)
            ├─ Enrich each result with breadcrumb from HierarchicalChunkEmbedding
            └─ Return results with populated breadcrumb
```

## Slices

### Slice A — Storage: Hierarchical Query

**Files**:
- `crates/storage/src/embeddings.rs` (modify)
- `crates/storage/src/lib.rs` (re-export)

**Changes**:

1. Add `HierarchicalChunkEmbedding` type:
```rust
pub struct HierarchicalChunkEmbedding {
    pub chunk: ChunkEmbeddingRecord,
    pub topic_id: Option<String>,
    pub topic_name: Option<String>,
    pub concept_id: Option<String>,
    pub concept_name: Option<String>,
}
```

2. Add `list_chunk_embeddings_hierarchical()`:
```sql
SELECT
    c.chunk_id, c.document_id, c.text, c.offset_start, c.offset_end,
    c.embedding, c.status, c.created_at,
    t.topic_id, t.name as topic_name,
    cp.concept_id, cp.name as concept_name
FROM chunks c
LEFT JOIN topics t ON c.topic_id = t.topic_id
LEFT JOIN concepts cp ON c.concept_id = cp.concept_id
WHERE c.embedding IS NOT NULL AND c.status = 'ready'
  AND ($topic_filter IS NULL OR c.topic_id = $topic_filter)
  AND ($concept_filter IS NULL OR c.concept_id = $concept_filter)
```

3. Add `has_hierarchy_data()`:
```sql
SELECT EXISTS(SELECT 1 FROM chunks WHERE topic_id IS NOT NULL LIMIT 1)
```

**Estimated lines**: ~80

---

### Slice B — Config + Response Types

**Files**:
- `crates/config/src/lib.rs` (modify)
- `crates/common/src/types.rs` (modify)

**Changes**:

1. Add to `RetrievalConfig`:
```rust
pub use_hierarchy: bool,  // default: true
```

2. Add breadcrumb fields to `Citation`, `SearchHit`, `RetrieveHit`:
```rust
pub topic_name: Option<String>,
pub concept_name: Option<String>,
pub breadcrumb: Option<String>,
```

**Estimated lines**: ~60

---

### Slice C — Engine: Hierarchical Retrieval + Enrichment

**Files**:
- `crates/engine/src/retrieve.rs` (modify)
- `crates/engine/src/context.rs` (modify)

**Changes**:

1. Modify `search()`, `retrieve()`, `build_context()` to:
   - Accept optional `topic_filter: Option<&str>` and `concept_filter: Option<&str>`
   - Check `use_hierarchy && has_hierarchy_data()`
   - Route to hierarchical or flat path
   - Enrich results with breadcrumb when hierarchical

2. Add breadcrumb builder helper:
```rust
fn build_breadcrumb(display_name: &str, topic_name: Option<&str>, concept_name: Option<&str>) -> String {
    match (topic_name, concept_name) {
        (Some(topic), Some(concept)) => format!("{} > {} > {}", display_name, topic, concept),
        (Some(topic), None) => format!("{} > {}", display_name, topic),
        _ => display_name.to_string(),
    }
}
```

**Estimated lines**: ~200

---

### Slice D — CLI Flags

**Files**:
- `crates/cli/src/commands/context.rs` (modify)
- `crates/cli/src/commands/search.rs` (modify)
- `crates/cli/src/commands/retrieve.rs` (modify)

**Changes**:

1. Add clap args to each command:
```rust
#[arg(long)]
flat: bool,

#[arg(long)]
topic: Option<String>,

#[arg(long)]
concept: Option<String>,
```

2. Add validation:
- `--flat` conflicts with `--topic`/`--concept`
- `--topic` and `--concept` conflict with each other

3. Resolve topic/concept name → ID via storage functions

4. Pass filters to engine functions

**Estimated lines**: ~100

---

### Slice E — Tests + Verification

**Files**:
- `crates/storage/src/embeddings.rs` (tests)
- `crates/engine/src/retrieve.rs` (tests)
- `crates/engine/src/context.rs` (tests)

**Tests needed**:
1. `list_chunk_embeddings_hierarchical` with no filter
2. `list_chunk_embeddings_hierarchical` with topic filter
3. `list_chunk_embeddings_hierarchical` with concept filter
4. `has_hierarchy_data` returns true when hierarchy exists
5. `has_hierarchy_data` returns false when no hierarchy
6. Engine hierarchical retrieval enriches breadcrumb
7. Engine flat retrieval returns null breadcrumb
8. Engine falls back to flat when no hierarchy data
9. CLI flag validation (conflicts)
10. All 209 existing tests still pass

**Estimated lines**: ~200

---

## Dependency graph

```
A (storage query) ──┐
B (config + types) ─┼── C (engine) ── D (CLI) ── E (tests)
                     │
```

A and B can be done in parallel. C depends on both. D depends on C. E depends on all.

## Workload forecast

| Slice | Lines | Risk |
|-------|-------|------|
| A | ~80 | Low |
| B | ~60 | Low |
| C | ~200 | Medium |
| D | ~100 | Low |
| E | ~200 | Low |
| **Total** | **~640** | |

All slices well under 300-line budget. No chained PRs needed.

## Backward compatibility

- `use_hierarchy` defaults to `true` but falls back automatically when no hierarchy data exists
- `--flat` produces identical output to v0.1.0
- All existing tests continue to pass
- New breadcrumb fields are `null` in flat mode (JSON backward compat)
