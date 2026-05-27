# Storage CRUD Implementation Report

## Summary

Added document, chunk, and embedding CRUD operations to the `storage` crate.
All operations compile cleanly and all 23 unit tests pass with in-memory SQLite.

## Files Changed

| File | Action | Description |
|------|--------|-------------|
| `crates/storage/Cargo.toml` | modified | Added `chrono = { workspace = true }` dependency |
| `crates/storage/src/lib.rs` | modified | Added `mod documents; mod chunks; mod embeddings; mod util;` declarations; added `Database::open_memory()` test constructor and updated test |
| `crates/storage/src/util.rs` | **new** | Shared helpers: `storage_err()`, `format_dt()`, `parse_dt()` |
| `crates/storage/src/documents.rs` | **new** | Document CRUD (7 methods) + 11 tests |
| `crates/storage/src/chunks.rs` | **new** | Chunk CRUD (2 methods) + 6 tests |
| `crates/storage/src/embeddings.rs` | **new** | Embedding CRUD (2 methods) + 5 tests |

## API Surface

### Documents (`impl Database`)
- `insert_document(&self, doc: &Document) -> Result<(), HarnessError>`
- `get_document(&self, id: &str) -> Result<Option<Document>, HarnessError>`
- `list_documents(&self) -> Result<Vec<Document>, HarnessError>`
- `update_document_status(&self, id: &str, status: DocumentStatus, error: Option<ErrorInfo>) -> Result<(), HarnessError>`
- `update_document_chunk_count(&self, id: &str, count: u32) -> Result<(), HarnessError>`
- `increment_retry_count(&self, id: &str) -> Result<(), HarnessError>`
- `reset_retry_count(&self, id: &str) -> Result<(), HarnessError>`

### Chunks (`impl Database`)
- `insert_chunks(&self, document_id: &str, chunks: &[Chunk]) -> Result<(), HarnessError>`
- `delete_chunks_for_document(&self, document_id: &str) -> Result<u64, HarnessError>`

### Embeddings (`impl Database`)
- `insert_embeddings(&self, embeddings: &[(String, Vec<f32>, &str, &str)]) -> Result<(), HarnessError>`
- `delete_embeddings_for_document(&self, document_id: &str) -> Result<u64, HarnessError>`

## Implementation Details

- **Error mapping**: All rusqlite errors are mapped to `HarnessError::StorageError` via a shared `storage_err()` helper.
- **Transactions**: Batch inserts (chunks, embeddings) use `conn.unchecked_transaction()` with automatic rollback on error. This was chosen because rusqlite 0.31.0's `Connection::transaction()` requires `&mut self`, while the `Database` methods take `&self`.
- **Datetime handling**: SQLite `TEXT` datetime columns are formatted as `%Y-%m-%d %H:%M:%S` (compatible with SQLite's `datetime()` function). Parsed via `chrono::NaiveDateTime`.
- **Enum mapping**: `DocumentStatus` and `FileType` are stored as lowercase strings (`"pending"`, `"processing"`, `"pdf"`, etc.) matching their `Display` impls.
- **Embedding vectors**: Stored as BLOBs — `Vec<f32>` serialized as little-endian bytes via `f32::to_le_bytes()`.
- **Validation**: `insert_chunks` validates that all chunks' `document_id` matches the provided parameter, returning `StorageError` on mismatch.
- **Not-found detection**: Update methods return `HarnessError::DocumentNotFound` when the target row doesn't exist.

## Test Results

```
running 23 tests
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Test coverage by module:
- **documents**: 11 tests (CRUD, status updates with/without error, retry counting, not-found, duplicate PK, full-field roundtrip)
- **chunks**: 6 tests (insert, all fields, mismatched doc ID, transaction rollback, delete, delete-empty)
- **embeddings**: 5 tests (insert with BLOB verification, rollback on failure, cascade delete by document, delete-empty, empty vector)
- **lib**: 1 test (in-memory health check)

## Open Risks / Notes

1. **No `get_embeddings` or `get_chunks` query methods** — only write/delete operations were requested. Read-back was verified in tests via raw `conn().query_row()` calls.
2. **`unchecked_transaction`** — used because `Connection::transaction()` requires `&mut self` in rusqlite 0.31.0. Safe for single-writer patterns but does not guard against nested transactions at the Rust level. If the crate evolves toward concurrent writers, consider wrapping `Connection` in a `Mutex` and switching to `transaction()`.
3. **`row_to_chunk` helper** is defined but unused (dead_code suppressed) — ready for when chunk query methods are added.

## Validation

```
cargo check -p storage   → clean (0 warnings)
cargo test  -p storage   → 23/23 passed
```
