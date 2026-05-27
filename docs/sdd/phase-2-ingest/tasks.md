# SDD Tasks — Phase 2: Ingest Pipeline

## Task overview

| # | Task | Depends on | Estimated lines | Commit message |
|---|---|---|---|---|
| 1 | Add dependencies + config extensions | — | ~100 | `feat: add ingest dependencies and config extensions` |
| 2 | Storage CRUD operations | 1 | ~300 | `feat(storage): add document, chunk, and embedding CRUD` |
| 3 | File validator | 1 | ~150 | `feat(ingest): add file validation` |
| 4 | Text extractor (TXT/MD/PDF) | 1 | ~250 | `feat(ingest): add text extraction for PDF, TXT, MD` |
| 5 | Text chunker | 1 | ~200 | `feat(ingest): add text chunking with overlap` |
| 6 | Embedding provider (OpenAI-compatible) | 1 | ~200 | `feat(providers): add OpenAI-compatible embedding provider` |
| 7 | Ingest pipeline orchestration | 2, 3, 4, 5, 6 | ~250 | `feat(engine): add ingest pipeline orchestration` |
| 8 | CLI commands (ingest, list, get, retry) | 7 | ~300 | `feat(cli): add ingest, list, get, retry commands` |
| 9 | Integration tests | 8 | ~200 | `test: add ingest pipeline integration tests` |

## Task 1: Dependencies + config extensions

**Goal**: Add required crate dependencies and extend config with ingest settings.

**Files to modify**:
- `Cargo.toml` — workspace dependencies
- `crates/ingest/Cargo.toml` — add lopdf, uuid
- `crates/providers/Cargo.toml` — add reqwest, tokio
- `crates/engine/Cargo.toml` — add ingest, providers
- `crates/config/src/lib.rs` — add IngestConfig struct

**Steps**:
1. Add workspace dependencies: `lopdf`, `reqwest`, `uuid`, `nanoid`
2. Update crate Cargo.toml files
3. Add `IngestConfig` to `Config`
4. Add ingest env vars to `EnvOverrides`
5. Verify `cargo check` passes

**Acceptance**:
- `cargo check` compiles
- Config has ingest section with defaults

---

## Task 2: Storage CRUD operations

**Goal**: Implement database operations for documents, chunks, and embeddings.

**Files to create/modify**:
- `crates/storage/src/documents.rs` — document CRUD
- `crates/storage/src/chunks.rs` — chunk CRUD
- `crates/storage/src/embeddings.rs` — embedding CRUD
- `crates/storage/src/lib.rs` — re-exports, Database methods

**Steps**:
1. Create `documents.rs` with insert/get/list/update methods
2. Create `chunks.rs` with insert/delete methods
3. Create `embeddings.rs` with insert/delete methods
4. Add `Document` ↔ DB row conversion
5. Add tests for each operation
6. Verify `cargo test -p storage` passes

**Acceptance**:
- `insert_document` + `get_document` roundtrips
- `list_documents` returns all
- `update_document_status` changes status
- `insert_chunks` + `delete_chunks_for_document` works
- `insert_embeddings` + `delete_embeddings_for_document` works
- All tests pass

---

## Task 3: File validator

**Goal**: Validate file type, size, and path policy.

**File**: `crates/ingest/src/validator.rs`

**Steps**:
1. Implement `validate_file(path, config) -> Result<(FileType, u64), HarnessError>`
2. Check path policy: resolve symlinks, reject traversal, network, device files
3. Check file exists
4. Check file extension → FileType
5. Check file size against limit
6. Add tests
7. Verify `cargo test -p ingest` passes

**Acceptance**:
- Valid .txt/.md/.pdf passes
- Unsupported extension returns `UnsupportedFileType`
- Too large returns `FileTooLarge`
- Missing file returns `FileNotFound`
- Traversal path returns `PathRejected`
- All tests pass

---

## Task 4: Text extractor

**Goal**: Extract text from PDF, TXT, and MD files.

**File**: `crates/ingest/src/extractor.rs`

**Steps**:
1. Implement `extract_text(path, file_type) -> Result<ExtractionResult, HarnessError>`
2. TXT/MD: read as UTF-8
3. PDF: use lopdf to extract per-page text
4. Handle edge cases: empty files, encoding errors, corrupted PDFs
5. Add tests with fixture files
6. Create test fixtures: `tests/fixtures/sample.txt`, `tests/fixtures/sample.md`
7. Verify `cargo test -p ingest` passes

**Acceptance**:
- TXT extraction returns correct text
- MD extraction returns correct text
- PDF extraction returns page-indexed text
- Empty file returns empty result
- Corrupted file returns appropriate error
- All tests pass

---

## Task 5: Text chunker

**Goal**: Split text into overlapping chunks with metadata.

**File**: `crates/ingest/src/chunker.rs`

**Steps**:
1. Implement `chunk_text(result, config) -> Result<Vec<ChunkInput>, HarnessError>`
2. Character-based splitting at `chunk_size_chars`
3. Sentence-boundary awareness for split points
4. Overlap handling
5. Page tracking for PDFs
6. Min chunk size filtering
7. Add tests
8. Verify `cargo test -p ingest` passes

**Acceptance**:
- Correct chunk count for given text length
- Overlap between consecutive chunks
- Page numbers tracked for PDFs
- Small text produces single chunk
- Empty text produces empty vec
- All tests pass

---

## Task 6: Embedding provider

**Goal**: Implement OpenAI-compatible embedding provider.

**Files**:
- `crates/providers/src/openai.rs`
- `crates/providers/src/lib.rs` — re-export

**Steps**:
1. Create `OpenAICompatibleProvider` struct
2. Implement `new(config)` — validate HTTPS, create HTTP client
3. Implement `EmbeddingProvider` trait
4. HTTP POST to embedding endpoint
5. Parse response, extract embedding vector
6. Handle errors: HTTP, timeout, invalid response
7. Add tests (mock or integration)
8. Verify `cargo test -p providers` passes

**Acceptance**:
- Provider rejects non-HTTPS endpoints
- `embed()` returns vector on success
- `embed()` returns `EmbeddingProviderError` on failure
- `model_id()` and `provider_id()` return correct values
- All tests pass

---

## Task 7: Ingest pipeline orchestration

**Goal**: Wire together validation, extraction, chunking, embedding, and storage.

**File**: `crates/engine/src/ingest.rs`

**Steps**:
1. Create `IngestResult` struct
2. Implement `Engine::ingest()` — full pipeline
3. Implement `Engine::retry_document()` — reset failed doc
4. Implement `Engine::list_documents()` — delegate to storage
5. Implement `Engine::get_document()` — delegate to storage
6. Implement `cleanup_partial()` — rollback on failure
7. Display name derivation
8. Add tests
9. Verify `cargo test -p engine` passes

**Acceptance**:
- `ingest()` creates document, chunks, embeddings
- `ingest()` marks document as Ready on success
- `ingest()` marks document as Failed on error
- `ingest()` cleans up partial data on failure
- `retry_document()` resets failed doc
- All tests pass

---

## Task 8: CLI commands

**Goal**: Add ingest, list, get, retry commands to CLI.

**Files**:
- `crates/cli/src/commands/ingest.rs`
- `crates/cli/src/commands/list.rs`
- `crates/cli/src/commands/get.rs`
- `crates/cli/src/commands/retry.rs`
- `crates/cli/src/commands/mod.rs` — update
- `crates/cli/src/main.rs` — update

**Steps**:
1. Create `ingest.rs` — parse path + display-name, call engine, format output
2. Create `list.rs` — call engine, format output
3. Create `get.rs` — parse doc_id, call engine, format output
4. Create `retry.rs` — parse doc_id, call engine, format output
5. Update `Commands` enum with new variants
6. Update `main()` dispatch
7. Add output structs for JSON serialization
8. Verify `cargo build` passes

**Acceptance**:
- `harness ingest <path>` works (human + JSON)
- `harness list` works (human + JSON)
- `harness get <id>` works (human + JSON)
- `harness retry <id>` works (human + JSON)
- `cargo build` passes
- `cargo clippy` passes

---

## Task 9: Integration tests

**Goal**: End-to-end tests for the ingest pipeline.

**File**: `tests/ingest_e2e.rs`

**Steps**:
1. Create test fixtures (sample.txt, sample.md)
2. Test: ingest .txt → list → get
3. Test: ingest .md → list → get
4. Test: ingest unsupported → error
5. Test: retry failed document
6. Test: list shows correct status
7. Verify `cargo test` passes

**Acceptance**:
- All integration tests pass
- `cargo test` passes (unit + integration)
- `cargo clippy -- -D warnings` passes
- `cargo fmt --check` passes

---

## Execution order

```
1 → 2, 3, 4, 5, 6 (parallel) → 7 → 8 → 9
```

Task 1 must complete first (dependencies). Tasks 2-6 can be done in parallel. Task 7 depends on 2+3+4+5+6. Task 8 depends on 7. Task 9 depends on 8.

## Review budget

| Task | Estimated lines | Within 400-line budget? |
|---|---|---|
| 1 | ~100 | ✓ |
| 2 | ~300 | ✓ |
| 3 | ~150 | ✓ |
| 4 | ~250 | ✓ |
| 5 | ~200 | ✓ |
| 6 | ~200 | ✓ |
| 7 | ~250 | ✓ |
| 8 | ~300 | ✓ |
| 9 | ~200 | ✓ |

All tasks are within the 400-line review budget. No chained PRs needed.
