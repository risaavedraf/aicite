# SDD Proposal — Phase 2: Ingest Pipeline

## Change name

`phase-2-ingest` — Document ingestion, chunking, and embedding pipeline

## Problem

Phase 1 established the project scaffold but no documents can be ingested. The CLI has no way to import files, extract text, split into chunks, generate embeddings, or manage document lifecycle. Without ingestion, there is no corpus, and without a corpus, retrieval (Phase 3) is impossible.

## Proposed change

Implement the full ingest pipeline: file validation, text extraction (PDF/TXT/MD), token-aware chunking with overlap, embedding generation via configurable provider, document lifecycle management, and CLI commands for ingest/list/get/retry.

## Scope

### In scope
- **File validation**: type checking (PDF/TXT/MD), configurable size limits, path policy (symlinks resolved, traversal/network paths rejected)
- **Text extraction**: PDF (page-level via lopdf), TXT, MD (plain text)
- **Chunking**: 800-1200 token chunks with 100-200 token overlap, character-based approximation for MVP
- **Embedding generation**: OpenAI-compatible provider via reqwest (HTTPS-only), configurable model/API key
- **Document lifecycle**: pending → processing → ready → failed, with error info
- **Storage CRUD**: insert/get/list/update for documents, chunks, embeddings
- **CLI commands**: `harness ingest <path>`, `harness list`, `harness get`, `harness retry`
- **Retry logic**: bounded retries (3 attempts), exponential backoff, recovery on startup
- **Partial cleanup**: rollback chunks/embeddings on failure
- **Display name**: derive sanitized label from filename when not provided

### Out of scope
- Durable locks + backlog (Phase 5)
- `harness refresh` (Phase 5)
- Retrieval pipeline (Phase 3)
- Context packs (Phase 4)
- Golden dataset (Phase 6)
- Rate limiting (Phase 5)
- `harness search/retrieve/context/read/trace` (Phases 3-4)

## Acceptance criteria

1. `harness ingest ./docs/sample.txt --json` ingests a text file and returns document_id, status, chunk_count
2. `harness ingest ./docs/sample.pdf --json` ingests a PDF with page-level extraction
3. `harness ingest ./docs/sample.md --json` ingests a markdown file
4. `harness list --json` returns all documents with status
5. `harness get <doc_id> --json` returns document metadata
6. `harness ingest ./unsupported.csv --json` returns `unsupported_file_type` error
7. `harness ingest ./huge.pdf --json` returns `file_too_large` if exceeds limit
8. Failed ingestion marks document as `failed` with error info
9. `harness retry <doc_id> --json` resets a failed document to `pending`
10. Partial data (chunks, embeddings) is cleaned up on failure
11. `cargo test` passes
12. `cargo clippy -- -D warnings` passes
13. `cargo fmt --check` passes

## Risks

| Risk | Mitigation |
|---|---|
| PDF extraction quality varies | Use lopdf for page-level extraction; test with real PDFs |
| Token counting inaccuracy (char-based) | Document the approximation; upgrade path to tiktoken |
| Embedding provider API changes | Abstract behind trait; provider is swappable |
| Large file memory usage | Stream/chunk extraction; configurable size limit |
| Scope creep into Phase 3/4/5 | Strict boundary: ingest only, no retrieval logic |

## Dependencies to add

| Crate | Version | Purpose |
|---|---|---|
| `lopdf` | 0.34 | PDF text extraction with page tracking |
| `reqwest` | 0.12 | HTTP client for embedding API (with rustls-tls) |
| `uuid` | 1 | Document/chunk ID generation (v4) |
| `nanoid` | 0.4 | Short IDs for display |
| `walkdir` | 2 | Directory traversal for path validation |

## Estimated size

~1500-1800 lines of Rust + tests

## Sequencing

This is Phase 2. Depends on Phase 1 (scaffold). Phase 3 (retrieval) depends on this.
