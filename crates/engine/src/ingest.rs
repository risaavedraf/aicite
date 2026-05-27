use chrono::Utc;
use common::types::{Chunk, Document, DocumentStatus, ErrorInfo};
use common::HarnessError;
use config::IngestConfig;
use ingest::chunker::{self};
use ingest::extractor::{self};
use ingest::validator;
use providers::EmbeddingProvider;
use std::path::Path;
use storage::Database;
use uuid::Uuid;

/// Result of a successful ingestion
#[derive(Debug, Clone)]
pub struct IngestResult {
    pub document_id: String,
    pub display_name: String,
    pub status: DocumentStatus,
    pub chunk_count: u32,
}

/// Run the full ingest pipeline for a file.
///
/// 1. Validate file (type, size, path policy)
/// 2. Create document record (pending)
/// 3. Mark processing
/// 4. Extract text
/// 5. Chunk text
/// 6. Store chunks
/// 7. Embed + store embeddings
/// 8. Mark ready (or failed with cleanup on error)
pub fn ingest(
    db: &Database,
    provider: &dyn EmbeddingProvider,
    config: &IngestConfig,
    path: &Path,
    display_name_override: Option<&str>,
    production_mode: bool,
) -> Result<IngestResult, HarnessError> {
    // 1. Validate
    let (file_type, _file_size) = validator::validate_file(path, config.max_file_size_bytes)?;

    // 2. Derive display name
    let display_name = validator::derive_display_name(path, display_name_override, production_mode);

    // 3. Create document record
    let document_id = format!("doc_{}", &Uuid::new_v4().to_string().replace('-', "")[..12]);
    let doc = Document {
        document_id: document_id.clone(),
        display_name: display_name.clone(),
        file_path: path.to_path_buf(),
        file_type: file_type.clone(),
        file_size_bytes: _file_size,
        status: DocumentStatus::Pending,
        chunk_count: 0,
        retry_count: 0,
        max_retry_count: config.max_retry_count,
        next_retry_at: None,
        error: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    db.insert_document(&doc)?;

    // 4. Mark processing
    db.update_document_status(&document_id, DocumentStatus::Processing, None)?;

    // Run the rest of the pipeline; on any error, clean up and mark failed
    match run_pipeline(db, provider, config, &document_id, path, &file_type) {
        Ok(chunk_count) => {
            // Success — mark ready
            db.update_document_status(&document_id, DocumentStatus::Ready, None)?;
            db.update_document_chunk_count(&document_id, chunk_count)?;

            Ok(IngestResult {
                document_id,
                display_name,
                status: DocumentStatus::Ready,
                chunk_count,
            })
        }
        Err(e) => {
            // Failure — clean up partial data and mark failed
            let _ = cleanup_partial(db, &document_id);
            let error_info = ErrorInfo {
                code: e.code().to_string(),
                message: e.message(),
            };
            let _ =
                db.update_document_status(&document_id, DocumentStatus::Failed, Some(error_info));
            Err(e)
        }
    }
}

/// Internal pipeline steps (extraction → chunking → storage → embedding)
fn run_pipeline(
    db: &Database,
    provider: &dyn EmbeddingProvider,
    config: &IngestConfig,
    document_id: &str,
    path: &Path,
    file_type: &common::FileType,
) -> Result<u32, HarnessError> {
    // Extract text
    let extraction = extractor::extract_text(path, file_type)?;

    if extraction.pages.is_empty() {
        return Ok(0);
    }

    // Convert to chunker::PageText
    let pages: Vec<chunker::PageText> = extraction
        .pages
        .iter()
        .map(|p| chunker::PageText {
            page: p.page,
            text: p.text.clone(),
        })
        .collect();

    // Chunk
    let chunk_inputs = chunker::chunk_text(
        &pages,
        config.chunk_size_chars,
        config.chunk_overlap_chars,
        config.min_chunk_size_chars,
    )?;

    if chunk_inputs.is_empty() {
        return Ok(0);
    }

    // Convert ChunkInput → Chunk for storage
    let now = Utc::now();
    let chunks: Vec<Chunk> = chunk_inputs
        .iter()
        .map(|ci| Chunk {
            chunk_id: format!(
                "chunk_{}",
                &Uuid::new_v4().to_string().replace('-', "")[..12]
            ),
            document_id: document_id.to_string(),
            section_id: None,
            chunk_index: ci.chunk_index,
            text: ci.text.clone(),
            page: ci.page,
            offset_start: Some(ci.offset_start),
            offset_end: Some(ci.offset_end),
            created_at: now,
        })
        .collect();

    let chunk_count = chunks.len() as u32;

    // Store chunks
    db.insert_chunks(document_id, &chunks)?;

    // Embed each chunk
    let mut embeddings: Vec<(String, Vec<f32>, &str, &str)> = Vec::new();
    for chunk in &chunks {
        let vector = provider.embed(&chunk.text)?;
        embeddings.push((
            chunk.chunk_id.clone(),
            vector,
            provider.model_id(),
            provider.provider_id(),
        ));
    }

    // Store embeddings
    db.insert_embeddings(&embeddings)?;

    Ok(chunk_count)
}

/// Clean up partial data from a failed ingestion
fn cleanup_partial(db: &Database, document_id: &str) -> Result<(), HarnessError> {
    // Delete embeddings first (FK dependency)
    let _ = db.delete_embeddings_for_document(document_id);
    // Delete chunks
    let _ = db.delete_chunks_for_document(document_id);
    Ok(())
}

/// Retry a failed document: reset to pending, clear error, increment retry count
pub fn retry_document(db: &Database, document_id: &str) -> Result<Document, HarnessError> {
    let doc = db
        .get_document(document_id)?
        .ok_or_else(|| HarnessError::DocumentNotFound {
            document_id: document_id.to_string(),
        })?;

    if doc.status != DocumentStatus::Failed {
        return Err(HarnessError::InvalidParameter {
            message: format!(
                "Document {} is not failed (status: {})",
                document_id, doc.status
            ),
        });
    }

    // Verify original file still exists
    if !doc.file_path.exists() {
        return Err(HarnessError::FileNotFound {
            path: doc.file_path,
        });
    }

    // Clean up any leftover data
    cleanup_partial(db, document_id)?;

    // Reset
    db.update_document_status(document_id, DocumentStatus::Pending, None)?;
    db.reset_retry_count(document_id)?;
    db.update_document_chunk_count(document_id, 0)?;

    // Return updated document
    let updated = db
        .get_document(document_id)?
        .ok_or_else(|| HarnessError::DocumentNotFound {
            document_id: document_id.to_string(),
        })?;

    Ok(updated)
}

/// List all documents
pub fn list_documents(db: &Database) -> Result<Vec<Document>, HarnessError> {
    db.list_documents()
}

/// Get a single document by ID
pub fn get_document(db: &Database, document_id: &str) -> Result<Document, HarnessError> {
    db.get_document(document_id)?
        .ok_or_else(|| HarnessError::DocumentNotFound {
            document_id: document_id.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use storage::Database;

    fn test_db() -> Database {
        Database::open_memory().expect("failed to open in-memory DB")
    }

    #[test]
    fn test_get_document_not_found() {
        let db = test_db();
        let result = get_document(&db, "doc_nonexistent");
        assert!(matches!(result, Err(HarnessError::DocumentNotFound { .. })));
    }

    #[test]
    fn test_list_documents_empty() {
        let db = test_db();
        let docs = list_documents(&db).unwrap();
        assert!(docs.is_empty());
    }

    #[test]
    fn test_retry_document_not_failed() {
        let db = test_db();
        // Insert a pending document
        let doc = Document {
            document_id: "doc_test1".to_string(),
            display_name: "test.txt".to_string(),
            file_path: Path::new("/tmp/test.txt").to_path_buf(),
            file_type: common::FileType::Txt,
            file_size_bytes: 100,
            status: DocumentStatus::Pending,
            chunk_count: 0,
            retry_count: 0,
            max_retry_count: 3,
            next_retry_at: None,
            error: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        db.insert_document(&doc).unwrap();

        // Retry should fail because status is pending, not failed
        let result = retry_document(&db, "doc_test1");
        assert!(matches!(result, Err(HarnessError::InvalidParameter { .. })));
    }

    #[test]
    fn test_retry_document_not_found() {
        let db = test_db();
        let result = retry_document(&db, "doc_nonexistent");
        assert!(matches!(result, Err(HarnessError::DocumentNotFound { .. })));
    }
}
