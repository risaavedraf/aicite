# Explore Notes — Phase 2: Ingest Pipeline

## Current state

Phase 1 scaffold is complete. We have:
- Cargo workspace with 9 crates
- `common`: Document, Chunk, Citation types; HarnessError (19 variants); ExitCode enum
- `storage`: SQLite with WAL mode, migration system, initial schema (documents, chunks, embeddings, traces tables)
- `config`: Config loading with env/file/flag precedence
- `cli`: clap CLI skeleton with `harness health --json`
- `ingest`, `providers`, `engine`: stub crates

## What Phase 2 must deliver

From the roadmap and PRD:

1. **File validation** (FR-002): type checking (PDF/TXT/MD), size limits, path policy
2. **Text extraction** (FR-003): PDF, TXT, MD
3. **Chunking** (FR-004): 800-1200 tokens, 100-200 overlap, with metadata
4. **Embedding generation** (FR-005): configurable provider
5. **Document lifecycle** (FR-006): pending → processing → ready → failed
6. **CLI commands**: `harness ingest <path>`, `harness list`, `harness get`
7. **Error info** (FR-007): human-readable reason + machine-readable code
8. **Retry/backoff** (FR-013): bounded retries with exponential backoff
9. **Partial cleanup** (FR-011): rollback on failure
10. **Recovery path** (FR-014): `harness retry` for failed docs

## Crate dependency analysis

### ingest crate (main workhorse)
- Needs: `common`, `storage`, `providers`
- New deps needed:
  - PDF extraction: `pdf-extract` (simple) or `lopdf` (more control)
  - Token counting: `tiktoken-rs` (OpenAI-compatible) or simple whitespace split
  - UUID generation: `uuid` with v4 feature
  - File type detection: `mime_guess` or just extension-based (PRD says extension-based)

### providers crate
- Needs: `common`
- New deps needed:
  - HTTP client: `reqwest` (for OpenAI-compatible API calls)
  - Async runtime: already have `tokio` in workspace

### engine crate
- Needs: `common`, `storage`, `ingest`, `providers`
- Orchestrates the full ingest pipeline

### storage crate
- Needs: `common`
- Add CRUD operations for documents, chunks, embeddings

### cli crate
- Needs: `config`, `engine`, `common`
- Add `ingest`, `list`, `get`, `retry` commands

## Key design decisions needed

### 1. PDF extraction library
- **pdf-extract**: Simple API, extracts all text as string. No page-level tracking.
- **lopdf**: Lower-level, can extract text per page. More code needed.
- **Recommendation**: `lopdf` for page tracking (PRD wants page numbers in citations)

### 2. Token counting strategy
- **tiktoken-rs**: Accurate for OpenAI models, adds dependency
- **Whitespace split**: Simpler, less accurate for token boundaries
- **Recommendation**: Start with character-based approximation (4 chars ≈ 1 token) for MVP. Can upgrade to tiktoken later.

### 3. Chunk ID format
- `chunk_<uuid>` — globally unique, debuggable
- Sequential per document: `doc_123_chunk_000` — human-readable
- **Recommendation**: `chunk_<uuid>` for global uniqueness, with `chunk_index` for ordering

### 4. Embedding provider implementation
- Trait already exists in `providers`
- Need concrete `OpenAICompatibleProvider` using reqwest
- API key from env var (HARNESS_EMBEDDING_API_KEY or provider-specific)
- HTTPS-only enforcement (architecture decision)

### 5. Ingest pipeline flow
```
ingest(path)
  → validate_file(path, config)       # type, size, path policy
  → create_document_record(db)         # status: pending
  → set_status(db, processing)         # status: processing
  → extract_text(path, file_type)      # PDF/TXT/MD → String
  → chunk_text(text, config)           # 800-1200 tokens, 100-200 overlap
  → store_chunks(db, chunks)           # persist chunks
  → embed_chunks(provider, chunks)     # generate vectors
  → store_embeddings(db, embeddings)   # persist vectors
  → set_status(db, ready)              # status: ready
  → on error: cleanup_partial(db)      # rollback chunks/embeddings
  → on error: set_status(db, failed)   # with error info
```

### 6. Durable locks (Phase 5 scope)
- For now: simple in-process lock (single-shot model)
- Phase 5 adds durable locks + backlog

## Scope boundary — what's NOT in Phase 2

- Durable locks + backlog (Phase 5)
- `harness refresh` (Phase 5)
- Retrieval pipeline (Phase 3)
- Context packs (Phase 4)
- Golden dataset (Phase 6)

## Estimated scope

| Component | Estimated lines |
|---|---|
| ingest crate (validation, extraction, chunking) | ~400 |
| providers crate (OpenAI-compatible) | ~200 |
| storage crate (CRUD operations) | ~300 |
| engine crate (orchestration) | ~200 |
| CLI commands (ingest, list, get, retry) | ~300 |
| Tests | ~200 |
| **Total** | ~1600 |
