# Phase 11 Proposal — Hierarchical Retrieval

## Problem

Phase 10 built the hierarchical graph foundation (topics, concepts, chunks with hierarchy, sentence chunker). But the retrieval pipeline still uses flat vector search — `list_ready_chunk_embeddings()` loads ALL chunks into memory with no hierarchy awareness. Cosine scores remain 0.62-0.69 because large chunks dilute matches, and the agent gets no semantic context about where each chunk lives in the document hierarchy.

## Goal

Consume the hierarchy infrastructure in the retrieval flow to deliver:
- **Chunk-first retrieval with topic/concept enrichment** — small chunks (30-200 chars) match precisely, then get enriched with their breadcrumb
- **Breadcrumb in responses** — `"arch.txt > Auth > JWT"` so the agent knows the semantic context
- **`--flat` fallback** — backward-compatible v0.1.0 behavior when hierarchy is unavailable or explicitly requested
- **Scoped query flags** — `--topic` and `--concept` to narrow search within a semantic area

## Scope

### In scope
- New hierarchical retrieval query in storage layer
- `RetrievalConfig.use_hierarchy` field
- Engine enrichment layer (breadcrumb assembly)
- Response type updates (`Citation`, `SearchHit`, `RetrieveHit` + breadcrumb fields)
- CLI flags: `--flat`, `--topic`, `--concept`
- Tests: hierarchical retrieval, flat fallback, scoped queries, breadcrumb correctness

### Out of scope
- Semantic links population (Phase 12 territory)
- Compact/full response mode (Phase 12)
- Topic management CLI commands (`topics list`, `topics rename`, etc.)
- Evaluation fixtures / golden dataset updates (Phase 12)

## Approach

**Strategy**: Modify the retrieval flow to check hierarchy availability and route accordingly.

```
CLI (context/search/retrieve) with optional --flat / --topic / --concept
  └─ Engine
       ├─ --flat OR no hierarchy data → existing flat path (unchanged)
       └─ hierarchical path:
            ├─ storage: list_chunk_embeddings_hierarchical(topic?, concept?)
            ├─ retrieval: rank_by_similarity (same cosine logic)
            └─ engine: enrich with breadcrumb (topic_name > concept_name > doc_name)
```

**Key principle**: The cosine similarity logic does NOT change. Only the query scope (which chunks to consider) and response enrichment (breadcrumb metadata) change.

## Estimated scope

| Area | Est. Lines |
|------|------------|
| Storage hierarchical query | ~80 |
| Config update | ~20 |
| Engine retrieval routing | ~120 |
| Context enrichment | ~80 |
| Response type updates | ~40 |
| CLI flag additions | ~60 |
| Tests | ~200 |
| **Total** | **~600** |

Well under the 300-line review budget per slice. Can be split into 3-4 slices.

## Risks

| Risk | Mitigation |
|------|------------|
| Mixed corpus (some docs with hierarchy, some without) | Automatic flat fallback per-document |
| Performance regression from JOIN query | SQLite indexes on topic_id/concept_id already exist from Phase 10 |
| Breaking existing tests | `--flat` produces identical output to v0.1.0 |

## Dependencies

- Phase 10 complete (hierarchy schema, CRUD, ingest pipeline) ✅
- No external dependencies

## Recommendation

Proceed to Spec → Design → Tasks → Apply.
