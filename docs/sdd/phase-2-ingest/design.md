# SDD Design — Phase 2: Ingest Pipeline

## Architecture overview

The ingest pipeline follows the single-shot durable process model. Each `harness ingest` invocation validates, extracts, chunks, embeds, and persists — then exits.

```
CLI (ingest command)
  └─→ Engine::ingest()
        ├─→ Validator::validate_file()      [ingest crate]
        ├─→ Extractor::extract_text()       [ingest crate]
        ├─→ Chunker::chunk_text()           [ingest crate]
        ├─→ Provider::embed()               [providers crate]
        └─→ Database::insert_*()            [storage crate]
```

## Crate responsibilities

### ingest crate

**Modules**:
- `lib.rs` — re-exports
- `validator.rs` — file type, size, path validation
- `extractor.rs` — text extraction (PDF, TXT, MD)
- `chunker.rs` — text chunking with overlap
- `pipeline.rs` — orchestration of the ingest flow

**Public API**:
```rust
pub fn validate_file(path: &Path, config: &IngestConfig) -> Result<(FileType, u64), HarnessError>;
pub fn extract_text(path: &Path, file_type: &FileType) -> Result<ExtractionResult, HarnessError>;
pub fn chunk_text(result: &ExtractionResult, config: &IngestConfig) -> Result<Vec<ChunkInput>, HarnessError>;
pub fn derive_display_name(path: &Path, override_name: Option<&str>, production_mode: bool) -> String;
```

### providers crate

**Modules**:
- `lib.rs` — trait re-export
- `openai.rs` — OpenAI-compatible embedding provider

**Public API**:
```rust
pub struct OpenAICompatibleProvider { ... }
impl OpenAICompatibleProvider {
    pub fn new(config: &EmbeddingConfig) -> Result<Self, HarnessError>;
}
impl EmbeddingProvider for OpenAICompatibleProvider { ... }
```

### storage crate

**New modules**:
- `documents.rs` — document CRUD
- `chunks.rs` — chunk CRUD
- `embeddings.rs` — embedding CRUD

**Public API additions to Database**:
```rust
// Documents
pub fn insert_document(&self, doc: &Document) -> Result<(), HarnessError>;
pub fn get_document(&self, id: &str) -> Result<Option<Document>, HarnessError>;
pub fn list_documents(&self) -> Result<Vec<Document>, HarnessError>;
pub fn update_document_status(&self, id: &str, status: DocumentStatus, error: Option<ErrorInfo>) -> Result<(), HarnessError>;
pub fn update_document_chunk_count(&self, id: &str, count: u32) -> Result<(), HarnessError>;

// Chunks
pub fn insert_chunks(&self, document_id: &str, chunks: &[Chunk]) -> Result<(), HarnessError>;
pub fn delete_chunks_for_document(&self, document_id: &str) -> Result<u64, HarnessError>;

// Embeddings
pub fn insert_embeddings(&self, embeddings: &[(String, Vec<f32>, &str, &str)]) -> Result<(), HarnessError>;
pub fn delete_embeddings_for_document(&self, document_id: &str) -> Result<u64, HarnessError>;
```

### engine crate

**New methods**:
```rust
impl Engine {
    pub fn ingest(
        db: &Database,
        provider: &dyn EmbeddingProvider,
        config: &IngestConfig,
        path: &Path,
        display_name: Option<&str>,
    ) -> Result<IngestResult, HarnessError>;

    pub fn retry_document(
        db: &Database,
        document_id: &str,
    ) -> Result<Document, HarnessError>;

    pub fn list_documents(db: &Database) -> Result<Vec<Document>, HarnessError>;

    pub fn get_document(db: &Database, document_id: &str) -> Result<Document, HarnessError>;
}
```

### cli crate

**New commands**:
- `commands/ingest.rs` — `harness ingest <path>`
- `commands/list.rs` — `harness list`
- `commands/get.rs` — `harness get <id>`
- `commands/retry.rs` — `harness retry <id>`

## Data flow diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    harness ingest <path>                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌────────┐ │
│  │ Validate │───→│ Extract  │───→│  Chunk   │───→│ Embed  │ │
│  │  File    │    │  Text    │    │  Text    │    │Chunks  │ │
│  └──────────┘    └──────────┘    └──────────┘    └────────┘ │
│       │               │               │              │       │
│       ▼               ▼               ▼              ▼       │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │                    SQLite (storage)                      │ │
│  │  documents ──→ chunks ──→ embeddings                    │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                              │
│  On error: cleanup_partial() + status = Failed               │
└─────────────────────────────────────────────────────────────┘
```

## Error handling strategy

| Stage | On error | Cleanup |
|---|---|---|
| Validation | Return error immediately | No DB record created |
| Text extraction | Mark Failed | Delete document record |
| Chunking | Mark Failed | Delete document record |
| Store chunks | Mark Failed | Delete partial chunks |
| Embedding | Mark Failed | Delete embeddings + chunks |
| Store embeddings | Mark Failed | Delete partial embeddings |

All cleanup happens in a single transaction to ensure consistency.

## Token counting approach

For the MVP, we use character-based approximation:
- 1 token ≈ 4 characters (English text average)
- Default chunk: 1000 chars ≈ 250 tokens
- Default overlap: 200 chars ≈ 50 tokens

This is documented as an approximation. The upgrade path to `tiktoken-rs` is straightforward:
1. Add `tiktoken-rs` dependency
2. Replace `chars.len() / 4` with `tokenizer.encode(text).len()`
3. No API changes needed

## PDF extraction approach

Using `lopdf`:
1. Open document with `Document::load()`
2. Get page count from `document.get_pages()`
3. For each page, extract content with `document.extract_text()`
4. Handle encoding issues gracefully (lossy UTF-8 where needed)
5. Return page-indexed text

Edge cases:
- Scanned PDFs (no text layer) → empty string per page
- Encrypted PDFs → `InternalError`
- Corrupted PDFs → `InternalError`
- Very large PDFs → size limit enforced before extraction

## Display name derivation

```rust
fn derive_display_name(path: &Path, override_name: Option<&str>, production_mode: bool) -> String {
    if let Some(name) = override_name {
        return sanitize_display_name(name);
    }

    if production_mode {
        // Generic label to avoid PII
        return format!("document_{}", &uuid[..8]);
    }

    // Local/private mode: use filename
    path.file_name()
        .and_then(|n| n.to_str())
        .map(sanitize_display_name)
        .unwrap_or_else(|| "unknown".to_string())
}
```

## Configuration additions

### In Config struct

```rust
pub struct IngestConfig {
    pub max_file_size_bytes: u64,
    pub chunk_size_chars: usize,
    pub chunk_overlap_chars: usize,
    pub min_chunk_size_chars: usize,
    pub max_retry_count: u32,
    pub embedding_timeout_secs: u64,
    pub embedding_endpoint: Option<String>,
}
```

### Defaults

```rust
impl Default for IngestConfig {
    fn default() -> Self {
        Self {
            max_file_size_bytes: 50 * 1024 * 1024, // 50MB
            chunk_size_chars: 1000,
            chunk_overlap_chars: 200,
            min_chunk_size_chars: 100,
            max_retry_count: 3,
            embedding_timeout_secs: 30,
            embedding_endpoint: None,
        }
    }
}
```

## Testing strategy

### Unit tests (per crate)

**ingest crate**:
- `test_validate_txt_file` — valid .txt passes
- `test_validate_unsupported_type` — .csv returns UnsupportedFileType
- `test_validate_file_too_large` — returns FileTooLarge
- `test_validate_path_traversal` — returns PathRejected
- `test_chunk_text_basic` — correct chunk count and overlap
- `test_chunk_text_small_file` — single chunk for small text
- `test_chunk_text_page_boundaries` — page tracking works
- `test_derive_display_name` — with/without override, production mode

**providers crate**:
- `test_openai_provider_creation` — valid config
- `test_openai_provider_rejects_http` — non-HTTPS rejected

**storage crate**:
- `test_insert_and_get_document`
- `test_list_documents`
- `test_update_document_status`
- `test_insert_and_delete_chunks`
- `test_insert_and_delete_embeddings`

### Integration tests

**tests/ingest_e2e.rs**:
- `test_ingest_txt_file` — full pipeline with .txt
- `test_ingest_md_file` — full pipeline with .md
- `test_ingest_unsupported` — error handling
- `test_list_after_ingest` — documents appear in list
- `test_get_document` — retrieve by ID
- `test_retry_failed` — reset failed document
