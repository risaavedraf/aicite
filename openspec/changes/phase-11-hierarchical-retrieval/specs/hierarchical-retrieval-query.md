# Spec: Hierarchical Retrieval Query

## Overview

Add a hierarchy-aware retrieval path that queries chunks with their topic/concept metadata, filters by topic or concept scope, and falls back to flat retrieval when hierarchy is unavailable.

## Requirements

### REQ-1: Hierarchical chunk embedding query

**ID**: REQ-HRQ-1
**Priority**: Must

The storage layer MUST provide a function to list chunk embeddings enriched with their topic/concept metadata, with optional filtering by topic_id or concept_id.

```rust
pub fn list_chunk_embeddings_hierarchical(
    &self,
    topic_filter: Option<&str>,
    concept_filter: Option<&str>,
) -> Result<Vec<HierarchicalChunkEmbedding>, CiteError>
```

**Data type**:
```rust
pub struct HierarchicalChunkEmbedding {
    pub chunk: ChunkEmbeddingRecord,
    pub topic_id: Option<String>,
    pub topic_name: Option<String>,
    pub concept_id: Option<String>,
    pub concept_name: Option<String>,
}
```

**Behavior**:
- JOINs `chunks` with `topics` and `concepts` tables
- If `topic_filter` is Some, returns only chunks belonging to that topic
- If `concept_filter` is Some, returns only chunks belonging to that concept
- Chunks with NULL topic_id/concept_id (not yet hierarchically ingested) are included when no filter is specified
- Uses existing indexes: `idx_chunks_concept`, `idx_chunks_topic`

### REQ-2: RetrievalConfig.use_hierarchy flag

**ID**: REQ-HRQ-2
**Priority**: Must

`RetrievalConfig` MUST include a `use_hierarchy: bool` field.

- Default: `true`
- When `true` and hierarchy data exists, use hierarchical retrieval
- When `false` or no hierarchy data exists, use flat retrieval

### REQ-3: Engine retrieval routing

**ID**: REQ-HRQ-3
**Priority**: Must

The `search()`, `retrieve()`, and `build_context()` functions MUST route between hierarchical and flat retrieval based on:
1. `RetrievalConfig.use_hierarchy` flag
2. Presence of hierarchy data in the database (at least one chunk with non-NULL topic_id)

**Routing logic**:
```rust
let use_hierarchy = config.use_hierarchy && db.has_hierarchy_data();

if use_hierarchy {
    let candidates = db.list_chunk_embeddings_hierarchical(topic_filter, concept_filter)?;
    // ... rank and enrich
} else {
    let candidates = db.list_ready_chunk_embeddings()?;
    // ... existing flat path
}
```

### REQ-4: Hierarchy data detection

**ID**: REQ-HRQ-4
**Priority**: Must

The storage layer MUST provide a function to detect whether hierarchy data exists:

```rust
pub fn has_hierarchy_data(&self) -> Result<bool, CiteError>
```

Returns `true` if at least one chunk has a non-NULL topic_id.

### REQ-5: Flat fallback preserves behavior

**ID**: REQ-HRQ-5
**Priority**: Must

When `use_hierarchy=false` or `--flat` flag is used, the retrieval output MUST be identical to v0.1.0 behavior — same query path, same response format, same scores.

## Scenarios

### S1: Hierarchical retrieval with topic filter
```
Given: DB has chunks with hierarchy (topic "Auth" has 3 chunks)
When: retrieve("JWT expiry", topic_filter=Some("auth_topic_id"))
Then: returns only chunks from "Auth" topic, ranked by similarity
```

### S2: Hierarchical retrieval without filter
```
Given: DB has chunks with hierarchy
When: retrieve("JWT expiry", topic_filter=None)
Then: returns top-k chunks from all topics, enriched with breadcrumb
```

### S3: Flat fallback — no hierarchy data
```
Given: DB has chunks but no hierarchy (all topic_id NULL)
When: retrieve("JWT expiry", use_hierarchy=true)
Then: falls back to flat retrieval, identical to v0.1.0
```

### S4: Flat fallback — explicit --flat
```
Given: DB has chunks with hierarchy
When: retrieve("JWT expiry", use_hierarchy=false / --flat flag)
Then: uses flat retrieval, identical to v0.1.0
```

### S5: Mixed corpus
```
Given: DB has some docs with hierarchy and some without
When: retrieve("JWT expiry", use_hierarchy=true, no topic filter)
Then: returns chunks from all docs (hierarchical and flat), ranked by similarity
```
