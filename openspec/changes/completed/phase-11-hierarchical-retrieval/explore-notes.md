# Phase 11 — Explore Notes: Hierarchical Retrieval

## Current Retrieval Architecture

### Data Flow (Flat — v0.1.0)

```
CLI Command (context/search/retrieve)
  └─ Engine function (build_context / search / retrieve)
       ├─ Validate query (empty, length ≤ 4000)
       ├─ Enforce rate limit (per provider, per route)
       ├─ Embed query via provider (Gemini or OpenAI-compatible)
       ├─ db.list_ready_chunk_embeddings() → Vec<ChunkEmbeddingRecord>  ← LOADS ALL INTO MEMORY
       ├─ rank_by_similarity(query_vector, candidates, k) → Vec<ScoredChunk>
       └─ Format response (ContextResponse / SearchHit / RetrieveHit)
```

**Key bottleneck**: `list_ready_chunk_embeddings()` loads every chunk from every ready document into memory, computes cosine similarity against all of them. No hierarchy filtering, no topic scoping.

### Files and Their Roles

| File | Role |
|------|------|
| `crates/retrieval/src/lib.rs` | `cosine_similarity()`, `rank_by_similarity()`, `ScoredChunk` type |
| `crates/engine/src/retrieve.rs` | `search()`, `retrieve()`, query validation, rate limiting |
| `crates/engine/src/context.rs` | `build_context()`, result-kind logic, trace persistence, citation assembly |
| `crates/cli/src/commands/context.rs` | CLI handler for `cite context` |
| `crates/cli/src/commands/search.rs` | CLI handler for `cite search` |
| `crates/cli/src/commands/retrieve.rs` | CLI handler for `cite retrieve` |
| `crates/storage/src/embeddings.rs` | `list_ready_chunk_embeddings()` — the single flat retrieval query |
| `crates/storage/src/chunks.rs` | `insert_chunks()`, `set_chunk_hierarchy()`, `delete_chunks_for_document()` |
| `crates/storage/src/topics.rs` | `insert_topic()`, `get_topic()`, `list_topics_by_document()`, `update_topic_chunk_count()` |
| `crates/storage/src/concepts.rs` | `insert_concept()`, `get_concept()`, `list_concepts_by_topic()`, `update_concept_chunk_count()` |
| `crates/storage/src/semantic_links.rs` | `insert_semantic_link()`, `get_links_from()`, `get_links_to()` |
| `crates/config/src/lib.rs` | `RetrievalConfig { top_k, evidence_floor, confidence_threshold }`, `IngestConfig` (Phase 10 additions) |
| `crates/common/src/types.rs` | `ContextResponse`, `Citation`, `ContextMetadata`, `ResultKind` |
| `crates/ingest/src/lib.rs` | `ingest_document()` — already wires hierarchy when `build_hierarchy=true` |
| `crates/graph/src/hierarchy.rs` | `build_hierarchy()` — assigns chunks to topics/concepts |
| `crates/graph/src/types.rs` | `Topic`, `Concept`, `SemanticLink`, `HeadingSpan` |
| `crates/engine/tests/golden_test.rs` | Golden dataset evaluation tests |

### Key Function Signatures

```rust
// retrieval/src/lib.rs
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> Option<f32>
pub fn rank_by_similarity(query_vector: &[f32], candidates: &[ChunkEmbeddingRecord], k: usize) -> Vec<ScoredChunk>

// engine/src/retrieve.rs
pub fn search(db: &Database, provider: &dyn EmbeddingProvider, config: &RetrievalConfig, rate_limit: &RateLimitConfig, query: &str, k_override: Option<u32>) -> Result<Vec<SearchHit>, CiteError>
pub fn retrieve(db: &Database, provider: &dyn EmbeddingProvider, config: &RetrievalConfig, rate_limit: &RateLimitConfig, query: &str, k_override: Option<u32>) -> Result<Vec<RetrieveHit>, CiteError>

// engine/src/context.rs
pub fn build_context(db: &Database, provider: &dyn EmbeddingProvider, config: &RetrievalConfig, rate_limit: &RateLimitConfig, query: &str, k_override: Option<u32>) -> Result<ContextResponse, CiteError>

// storage/src/embeddings.rs
pub fn list_ready_chunk_embeddings(&self) -> Result<Vec<ChunkEmbeddingRecord>, CiteError>
```

### Current Response JSON Formats

**`cite context --json`** returns `ContextResponse`:
```json
{
  "context_pack_id": "ctx_...",
  "result_kind": "context",
  "query_id": "qry_...",
  "trace_id": "trace_...",
  "instructions": "Use only the cited context...",
  "citations": [
    {
      "citation_id": "c1",
      "document_id": "...",
      "display_name": "architecture.txt",
      "chunk_id": "...",
      "page": null,
      "offset": null,
      "text": "JWT tokens with 15-min expiry",
      "score": 0.725,
      "confidence_label": null
    }
  ],
  "metadata": {
    "schema_version": "context-v1",
    "ranking_method": "vector_cosine_v1",
    "top_score": 0.725,
    ...
  }
}
```

**`cite search --json`** returns:
```json
{
  "query": "...",
  "top_k": 5,
  "hit_count": 1,
  "results": [
    {
      "chunk_id": "...",
      "document_id": "...",
      "display_name": "architecture.txt",
      "section_id": null,
      "chunk_index": 0,
      "page": null,
      "offset_start": null,
      "offset_end": null,
      "score": 0.725,
      "preview": "JWT tokens with..."
    }
  ]
}
```

**`cite retrieve --json`** returns:
```json
{
  "query": "...",
  "top_k": 5,
  "hit_count": 1,
  "results": [
    {
      "chunk_id": "...",
      "document_id": "...",
      "display_name": "architecture.txt",
      "section_id": null,
      "chunk_index": 0,
      "score": 0.725,
      "text": "JWT tokens with 15-min expiry"
    }
  ]
}
```

---

## Hierarchy Infrastructure Available from Phase 10

### Schema (Migration 006)

```sql
topics(topic_id PK, document_id FK, name, summary, embedding BLOB, chunk_count, created_at)
concepts(concept_id PK, topic_id FK, name, summary, embedding BLOB, chunk_count, created_at)
semantic_links(link_id PK, source_chunk_id FK, target_chunk_id FK, similarity_score, link_type, created_at)
chunks: + concept_id TEXT FK nullable, + topic_id TEXT FK nullable
```

### Storage CRUD (already implemented)

- `insert_topic()`, `get_topic()`, `list_topics_by_document()`, `update_topic_chunk_count()`
- `insert_concept()`, `get_concept()`, `list_concepts_by_topic()`, `update_concept_chunk_count()`
- `insert_semantic_link()`, `get_links_from()`, `get_links_to()`
- `set_chunk_hierarchy(chunk_id, topic_id, concept_id)` on chunks

### Config (already added in Phase 10)

- `IngestConfig.sentence_chunking: bool` (default: false)
- `IngestConfig.min_chunk_chars: usize` (default: 30)
- `IngestConfig.max_chunk_chars: usize` (default: 200)
- `IngestConfig.build_hierarchy: bool` (default: false)

### Ingest Pipeline (already wired)

`ingest_document()` in `crates/ingest/src/lib.rs` already:
- Extracts headings from markdown → builds hierarchy → inserts topics/concepts → assigns chunks to topics/concepts
- Creates "Untitled" topic for non-markdown files when `build_hierarchy=true`

---

## Gaps: What Phase 11 Needs

### 1. Storage: No hierarchy-aware retrieval query

**Current**: `list_ready_chunk_embeddings()` loads ALL chunks with no filtering.
**Need**: A variant that can:
- Filter by topic_id (for `--topic` flag)
- Filter by concept_id (for `--concept` flag)
- Join topic/concept metadata for breadcrumb enrichment
- Still support flat fallback

**Required new function**:
```rust
pub fn list_chunk_embeddings_hierarchical(
    &self,
    topic_filter: Option<&str>,
    concept_filter: Option<&str>,
) -> Result<Vec<HierarchicalChunkEmbedding>, CiteError>
```

Where `HierarchicalChunkEmbedding` extends `ChunkEmbeddingRecord` with:
```rust
pub struct HierarchicalChunkEmbedding {
    pub chunk: ChunkEmbeddingRecord,
    pub topic_id: Option<String>,
    pub topic_name: Option<String>,
    pub concept_id: Option<String>,
    pub concept_name: Option<String>,
}
```

### 2. Config: No retrieval-level hierarchy flag

**Current**: `RetrievalConfig` has only `top_k`, `evidence_floor`, `confidence_threshold`.
**Need**: Add `use_hierarchy: bool` (default: true when hierarchy data exists, false otherwise).

### 3. Engine: No hierarchy enrichment in retrieval

**Current**: `search()` and `retrieve()` return flat hits with no topic/concept context.
**Need**: After ranking by similarity, enrich each hit with its topic/concept breadcrumb.

### 4. Response types: No breadcrumb fields

**Current**: `Citation`, `SearchHit`, `RetrieveHit` have no hierarchy fields.
**Need**: Add breadcrumb metadata to all response types.

### 5. CLI: No `--flat`, `--topic`, `--concept` flags

**Current**: `context`, `search`, `retrieve` only accept `query` and `--k`.
**Need**: Add flags for hierarchy control.

---

## Estimated Scope of Changes

| Area | Files | Est. Lines | Risk |
|------|-------|------------|------|
| New storage query (hierarchical) | `storage/src/embeddings.rs` | ~80 | Low — new function, no breaking changes |
| RetrievalConfig update | `config/src/lib.rs` | ~20 | Low — additive field with default |
| Hierarchical retrieval engine | `engine/src/retrieve.rs` | ~120 | Medium — new code path + fallback |
| Context enrichment | `engine/src/context.rs` | ~80 | Medium — breadcrumb in metadata |
| Response type updates | `common/src/types.rs` | ~40 | Low — additive fields |
| CLI flag additions | `cli/src/commands/{context,search,retrieve}.rs` | ~60 | Low — new clap args |
| Tests | engine + storage test modules | ~200 | Low — standard patterns |
| **Total** | | **~600** | |

### Implementation Strategy

The retrieval flow should be modified as:

```
CLI Command with --flat / --topic / --concept
  └─ Engine function
       ├─ If --flat OR no hierarchy data: use existing flat path (unchanged)
       ├─ Else: use hierarchical path:
       │    ├─ db.list_chunk_embeddings_hierarchical(topic_filter, concept_filter)
       │    ├─ rank_by_similarity(query_vector, candidates, k)  ← same cosine logic
       │    └─ Enrich results with topic/concept breadcrumb
       └─ Format response with breadcrumb fields
```

### Key Constraints

- **Backward compat**: `--flat` must produce identical output to v0.1.0
- **Default behavior**: When hierarchy data exists, use it; when not, fall back to flat
- **No semantic_links yet**: Phase 11 does NOT populate `semantic_links` — that table is empty from Phase 10
- **Flat retrieval unchanged**: The existing `list_ready_chunk_embeddings()` stays as-is for `--flat` fallback

### Test Coverage

Existing tests that must continue passing:
- `crates/retrieval/src/lib.rs` — 4 tests (cosine similarity, rank_by_similarity)
- `crates/engine/src/retrieve.rs` — 8 tests (search, retrieve, rate limits, validation)
- `crates/engine/src/context.rs` — 12 tests (build_context, read, trace, result-kind)
- `crates/engine/tests/golden_test.rs` — golden dataset evaluation
- Total: ~209 tests project-wide (Phase 10 baseline)

New tests needed:
- Hierarchical retrieval with topic filter
- Hierarchical retrieval with concept filter
- Breadcrumb enrichment correctness
- `--flat` produces identical output to v0.1.0
- No hierarchy data → automatic flat fallback
- Mixed corpus (some docs with hierarchy, some without)

### v0.2.0 Phase Map Context

From `docs/sdd/v0.2-phase-map.md`:

> **Phase 11 — Hierarchical retrieval and CLI**
> Goal: consume hierarchy in retrieval flow.
> Deliverables:
> - chunk-first retrieval + topic/concept enrichment
> - breadcrumb in context outputs
> - `--flat` fallback + scoped query flags

Phase 12 will add compact/full response mode, evaluation fixtures, and release prep.
