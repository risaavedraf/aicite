# Explore Notes — Phase 3: Retrieval Pipeline

## Current state

Phase 2 left the system with:
- Documents, chunks, and embeddings persisted in SQLite.
- Embedding vectors stored as `BLOB` (little-endian `f32`).
- Ingest lifecycle and CLI commands (`ingest`, `list`, `get`, `retry`) complete.
- `retrieval` and `graph` crates still as stubs.

## What Phase 3 must deliver

From roadmap:
1. Vector index storage and lookup (on top of existing `embeddings` table)
2. Cosine similarity scoring
3. Top-k retrieval (`k` in 1..10, default 5)
4. `cite search` command
5. `cite retrieve` command
6. Source/section/chunk metadata attachment
7. Partial-corpus handling (only `ready` documents)

## Key design choices

- **Index strategy (MVP)**: use SQLite as canonical vector store; load ready chunk embeddings into memory for ranking per query.
- **Similarity**: cosine similarity with zero-norm and dimension-mismatch protection.
- **Readiness filter**: SQL join constrained to `documents.status = 'ready'`.
- **Commands**:
  - `cite search <query> [--k N]`: concise ranked hits.
  - `cite retrieve <query> [--k N]`: richer hit payload (text + offsets + page + metadata).
- **Provider**: reuse configured embedding provider to embed the query text.

## Risks

- Full-table scans on large corpora (acceptable for MVP; ANN index deferred).
- Embedding dimension mismatch across providers/models.
- User confusion between `search` and `retrieve` semantics.

## Non-goals

- ANN/HNSW/FAISS index.
- Hybrid lexical+vector reranking.
- Context packs/citations/traces output model from Phase 4.
