# SDD Spec — Phase 2: Ingest Pipeline

## 1. File validation

### Supported types

| Extension | FileType | Extraction method |
|---|---|---|
| `.pdf` | `Pdf` | lopdf, page-by-page |
| `.txt` | `Txt` | Direct read (UTF-8) |
| `.md`, `.markdown` | `Md` | Direct read (UTF-8) |

### Size limits

Configurable via `Config.ingest.max_file_size_bytes`. Default: 50 MB.

### Path policy

Before opening any file:
1. Resolve symlinks (`std::fs::canonicalize`)
2. Reject paths with `..` traversal sequences
3. Reject network paths (UNC `\\server`, `//server`)
4. Reject device files (`/dev/*`, `\\.\*`)
5. Path must exist and be a regular file

### Error mapping

| Condition | Error | Exit code |
|---|---|---|
| Unsupported extension | `UnsupportedFileType` | 1 |
| File too large | `FileTooLarge` | 1 |
| File not found | `FileNotFound` | 2 |
| Path rejected | `PathRejected` | 1 |

## 2. Text extraction

### TXT / MD

Read file as UTF-8 string. Lossy conversion is not allowed — invalid UTF-8 returns `InternalError`.

### PDF

Use `lopdf` to extract text per page:
1. Open PDF document
2. Iterate pages (1-indexed)
3. Extract text content from each page
4. Return `Vec<PageText>` where each entry has `page_number: u32` and `text: String`
5. Empty pages produce empty strings (preserving page numbering)

### Extraction result

```rust
pub struct ExtractionResult {
    pub pages: Vec<PageText>,
    pub total_chars: usize,
}

pub struct PageText {
    pub page: u32,
    pub text: String,
}
```

For TXT/MD, the result has a single page with `page: 1`.

## 3. Chunking

### Parameters

| Parameter | Default | Config key |
|---|---|---|
| Target chunk size | 1000 chars | `ingest.chunk_size_chars` |
| Overlap | 200 chars | `ingest.chunk_overlap_chars` |
| Min chunk size | 100 chars | `ingest.min_chunk_size_chars` |

### Algorithm

Character-based chunking with sentence-boundary awareness:

1. Concatenate all page text with page boundary markers
2. Split into candidate chunks at `chunk_size_chars`
3. For each split point, look backward up to `overlap` chars for a sentence boundary (`.`, `!`, `?`, `\n\n`)
4. If found, split at the sentence boundary
5. If not found, split at the character limit
6. Each chunk records: `text`, `page` (starting page), `offset_start`, `offset_end`
7. Drop chunks smaller than `min_chunk_size_chars` (except the last chunk)

### Page tracking

For PDFs:
- Track cumulative character offset per page
- Each chunk records the page number where it starts
- If a chunk spans multiple pages, record the starting page

For TXT/MD:
- All chunks have `page: None` (no page concept)

### Chunk metadata

```rust
pub struct ChunkInput {
    pub chunk_index: u32,
    pub text: String,
    pub page: Option<u32>,
    pub offset_start: u32,
    pub offset_end: u32,
}
```

## 4. Embedding generation

### Provider trait (already exists)

```rust
pub trait EmbeddingProvider {
    fn embed(&self, text: &str) -> Result<Vec<f32>, HarnessError>;
    fn model_id(&self) -> &str;
    fn provider_id(&self) -> &str;
}
```

### OpenAI-compatible implementation

New struct `OpenAICompatibleProvider`:
- HTTP POST to configurable endpoint (default: `https://api.openai.com/v1/embeddings`)
- Request body: `{"input": text, "model": model_id}`
- Response: `{"data": [{"embedding": [...]}]}`
- API key from `HARNESS_EMBEDDING_API_KEY` env var
- HTTPS-only enforcement: reject non-HTTPS URLs at construction time
- Configurable timeout (default: 30 seconds)
- Error mapping: HTTP errors → `EmbeddingProviderError`

### Batch embedding

For efficiency, embed all chunks in a single document ingestion:
1. Collect all chunk texts
2. Call `embed()` for each chunk (provider may batch internally)
3. Store embeddings alongside chunks

## 5. Storage CRUD operations

### Document operations

```rust
impl Database {
    pub fn insert_document(&self, doc: &Document) -> Result<(), HarnessError>;
    pub fn get_document(&self, id: &str) -> Result<Document, HarnessError>;
    pub fn list_documents(&self) -> Result<Vec<Document>, HarnessError>;
    pub fn update_document_status(&self, id: &str, status: DocumentStatus, error: Option<ErrorInfo>) -> Result<(), HarnessError>;
    pub fn update_document_chunk_count(&self, id: &str, count: u32) -> Result<(), HarnessError>;
    pub fn increment_retry_count(&self, id: &str) -> Result<(), HarnessError>;
    pub fn reset_retry_count(&self, id: &str) -> Result<(), HarnessError>;
}
```

### Chunk operations

```rust
impl Database {
    pub fn insert_chunks(&self, chunks: &[Chunk]) -> Result<(), HarnessError>;
    pub fn get_chunks_for_document(&self, doc_id: &str) -> Result<Vec<Chunk>, HarnessError>;
    pub fn delete_chunks_for_document(&self, doc_id: &str) -> Result<(), HarnessError>;
}
```

### Embedding operations

```rust
impl Database {
    pub fn insert_embeddings(&self, embeddings: &[(String, Vec<f32>, String, String)]) -> Result<(), HarnessError>;
    pub fn delete_embeddings_for_document(&self, doc_id: &str) -> Result<(), HarnessError>;
}
```

## 6. Ingest pipeline orchestration

### Engine method

```rust
impl Engine {
    pub fn ingest(&self, db: &Database, provider: &dyn EmbeddingProvider, config: &IngestConfig, path: &Path, display_name: Option<&str>) -> Result<IngestResult, HarnessError>;
}
```

### Pipeline steps

```
1. validate_file(path, config)
   → FileType, file_size
   → Error if unsupported/too large/not found/path rejected

2. derive_display_name(path, display_name)
   → Use provided name or sanitize filename
   → Production mode: generic "document_<id>.<ext>"

3. create_document_record(db)
   → document_id = "doc_<uuid>"
   → status = Pending
   → INSERT INTO documents

4. update_status(db, Processing)
   → UPDATE documents SET status = 'processing'

5. extract_text(path, file_type)
   → ExtractionResult { pages, total_chars }
   → On error: goto step 9 (failure)

6. chunk_text(extraction_result, config)
   → Vec<ChunkInput>
   → On error: goto step 9 (failure)

7. store_chunks(db, document_id, chunks)
   → INSERT INTO chunks
   → On error: goto step 9 (failure)

8. embed_and_store(db, provider, document_id, chunks)
   → For each chunk: embed(text) → vector
   → INSERT INTO embeddings
   → On error: goto step 9 (failure)

9. finalize(db, document_id, success)
   → If success: status = Ready, update chunk_count
   → If failure: cleanup_partial(db), status = Failed with error info
```

### Partial cleanup on failure

```rust
fn cleanup_partial(db: &Database, document_id: &str) -> Result<(), HarnessError> {
    // Delete embeddings for this document's chunks
    db.delete_embeddings_for_document(document_id)?;
    // Delete chunks for this document
    db.delete_chunks_for_document(document_id)?;
    // Document record stays (with Failed status)
    Ok(())
}
```

## 7. CLI commands

### `harness ingest <path>`

**Flags**:
- `--display-name <name>`: Override display name
- `--json`: Machine output

**Behavior**:
1. Load config
2. Validate path
3. Call `engine.ingest()`
4. Output result or error

**JSON success response**:
```json
{
  "document_id": "doc_abc123",
  "display_name": "handbook.pdf",
  "status": "ready",
  "chunk_count": 42
}
```

**JSON error response**:
```json
{
  "error": {
    "code": "unsupported_file_type",
    "message": "Unsupported file type: .csv"
  }
}
```

### `harness list`

**Flags**:
- `--json`: Machine output

**Behavior**:
1. Load config
2. Open database
3. List all documents
4. Output

**JSON response**:
```json
{
  "documents": [
    {
      "document_id": "doc_abc123",
      "display_name": "handbook.pdf",
      "status": "ready",
      "chunk_count": 42,
      "retry_count": 0,
      "created_at": "2026-05-27T18:00:00Z"
    }
  ]
}
```

### `harness get <document_id>`

**Flags**:
- `--json`: Machine output

**Behavior**:
1. Load config
2. Open database
3. Get document by ID
4. Output

**JSON response**:
```json
{
  "document_id": "doc_abc123",
  "display_name": "handbook.pdf",
  "status": "ready",
  "chunk_count": 42,
  "retry_count": 0,
  "max_retry_count": 3,
  "next_retry_at": null,
  "error": null
}
```

### `harness retry <document_id>`

**Flags**:
- `--json`: Machine output

**Behavior**:
1. Load config
2. Open database
3. Get document — error if not found
4. Verify status is `failed` — error if not
5. Verify original file still exists — error if not
6. Reset: status → pending, retry_count → 0, clear error
7. Output

**JSON response**:
```json
{
  "document_id": "doc_abc123",
  "display_name": "handbook.pdf",
  "status": "pending",
  "retry_count": 0,
  "max_retry_count": 3,
  "next_retry_at": null
}
```

## 8. Config additions

Add to `Config`:

```rust
pub struct IngestConfig {
    pub max_file_size_bytes: u64,       // default: 50MB
    pub chunk_size_chars: usize,        // default: 1000
    pub chunk_overlap_chars: usize,     // default: 200
    pub min_chunk_size_chars: usize,    // default: 100
    pub max_retry_count: u32,           // default: 3
    pub embedding_timeout_secs: u64,    // default: 30
    pub embedding_endpoint: Option<String>, // default: None (use provider default)
}
```

Environment variables:
- `HARNESS_MAX_FILE_SIZE` (bytes)
- `HARNESS_CHUNK_SIZE` (chars)
- `HARNESS_CHUNK_OVERLAP` (chars)
- `HARNESS_EMBEDDING_TIMEOUT` (seconds)
- `HARNESS_EMBEDDING_ENDPOINT` (URL)

## 9. Testing requirements

### Unit tests

- File validation: valid files, unsupported types, too large, path traversal
- Chunking: correct size, overlap, page boundaries, empty text
- Display name derivation: with/without override, production mode sanitization
- Error code mapping

### Integration tests

- Ingest a .txt file end-to-end
- Ingest a .md file end-to-end
- Ingest a .pdf file end-to-end (if test PDF available)
- List documents
- Get document by ID
- Retry failed document
- Ingest unsupported file type
- Ingest file that's too large

### Test fixtures

- `tests/fixtures/sample.txt` — simple text file
- `tests/fixtures/sample.md` — markdown with headers
- `tests/fixtures/sample.pdf` — small PDF (if feasible)

## 10. Non-goals for this change

- Retrieval pipeline (Phase 3)
- Context packs (Phase 4)
- Durable locks + backlog (Phase 5)
- Rate limiting (Phase 5)
- `harness refresh` (Phase 5)
- Golden dataset (Phase 6)
- Keyword extraction or NLP
- Graph construction
