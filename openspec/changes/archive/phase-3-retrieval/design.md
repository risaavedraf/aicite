# SDD Design — Phase 3: Retrieval Pipeline

## Architecture

```
CLI (search/retrieve)
  -> engine::retrieval::{search,retrieve}
      -> providers::EmbeddingProvider::embed(query)
      -> storage::Database::list_ready_chunk_embeddings()
      -> retrieval::{rank_by_cosine, top_k}
      -> CLI formatter (concise/full)
```

## Module changes

### `crates/storage`

Add embedding lookup API returning joined ready-corpus rows with decoded vectors:

- `list_ready_chunk_embeddings() -> Vec<ChunkEmbeddingRecord>`

Implementation details:
- SQL join across `embeddings`, `chunks`, `documents`
- filter: `documents.status = 'ready'`
- decode `BLOB` to `Vec<f32>` in little-endian groups of 4 bytes
- malformed vector blobs are skipped

### `crates/retrieval`

Implement pure ranking logic:
- `cosine_similarity(a,b) -> Option<f32>`
- `rank_by_similarity(query, candidates, k) -> Vec<ScoredChunk>`

Keep this crate deterministic and side-effect free.

### `crates/engine`

Add `retrieval` module orchestrating:
- query/k validation
- query embedding via provider
- storage fetch
- call retrieval ranking
- map to `SearchHit` / `RetrieveHit`

### `crates/cli`

Add commands:
- `commands/search.rs`
- `commands/retrieve.rs`

Wire into:
- `commands/mod.rs`
- `main.rs` command enum + dispatcher

Provider creation mirrors ingest command to keep behavior consistent.

## Error handling

- invalid `k` -> `invalid_parameter`
- empty query -> `invalid_parameter`
- query too long -> `query_too_long`
- provider errors bubble as `embedding_provider_error`
- storage issues bubble as `storage_error`

## Performance (MVP)

- O(N*d) scoring in-memory per query.
- Acceptable for current MVP corpus sizes.
- Future phase can add ANN index in `retrieval` without CLI contract break.
