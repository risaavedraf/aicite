use chrono::Utc;
use common::types::{Chunk, Document, DocumentStatus, ErrorInfo};
use common::HarnessError;
use config::IngestConfig;
use ingest::chunker::{self};
use ingest::extractor::{self};
use ingest::validator;
use providers::EmbeddingProvider;
use std::path::{Path, PathBuf};
use storage::Database;
use uuid::Uuid;

const INGEST_LOCK_NAME: &str = "ingest_pipeline";
const INGEST_LOCK_RETRY_AFTER_SECONDS: u32 = 5;

/// Result of a successful ingestion
#[derive(Debug, Clone)]
pub struct IngestResult {
    pub document_id: String,
    pub display_name: String,
    pub status: DocumentStatus,
    pub chunk_count: u32,
}

#[derive(Debug, Clone)]
pub enum IngestNextResult {
    Empty,
    Ingested(IngestResult),
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
    ingest_internal(
        db,
        provider,
        config,
        path,
        display_name_override,
        production_mode,
        true,
    )
}

pub fn enqueue_ingest(
    db: &Database,
    config: &IngestConfig,
    path: &Path,
    display_name_override: Option<&str>,
) -> Result<(), HarnessError> {
    let _ = validator::validate_file(path, config.max_file_size_bytes)?;
    db.upsert_ingest_backlog(path, display_name_override)
}

pub fn ingest_next(
    db: &Database,
    provider: &dyn EmbeddingProvider,
    config: &IngestConfig,
    production_mode: bool,
) -> Result<IngestNextResult, HarnessError> {
    let Some(item) = db.claim_next_ingest_backlog()? else {
        return Ok(IngestNextResult::Empty);
    };

    let source_path = PathBuf::from(&item.source_path);
    match ingest_internal(
        db,
        provider,
        config,
        &source_path,
        item.display_name_override.as_deref(),
        production_mode,
        false,
    ) {
        Ok(result) => {
            db.mark_ingest_backlog_done(&item.queue_id)?;
            Ok(IngestNextResult::Ingested(result))
        }
        Err(HarnessError::OperationInProgress {
            message,
            retry_after_seconds,
            lock_name,
        }) => {
            db.requeue_ingest_backlog(&item.queue_id)?;
            Err(HarnessError::OperationInProgress {
                message,
                retry_after_seconds,
                lock_name,
            })
        }
        Err(err) => {
            db.mark_ingest_backlog_failed(&item.queue_id)?;
            Err(err)
        }
    }
}

fn ingest_internal(
    db: &Database,
    provider: &dyn EmbeddingProvider,
    config: &IngestConfig,
    path: &Path,
    display_name_override: Option<&str>,
    production_mode: bool,
    queue_on_lock_conflict: bool,
) -> Result<IngestResult, HarnessError> {
    // 1. Validate
    let (file_type, file_size) = validator::validate_file(path, config.max_file_size_bytes)?;

    let lock_owner_id = format!("ingest_{}", Uuid::new_v4());
    if !db.try_acquire_lock(INGEST_LOCK_NAME, &lock_owner_id)? {
        if queue_on_lock_conflict {
            db.upsert_ingest_backlog(path, display_name_override)?;
        }

        return Err(HarnessError::OperationInProgress {
            message: "Ingest pipeline is busy; request queued".to_string(),
            retry_after_seconds: INGEST_LOCK_RETRY_AFTER_SECONDS,
            lock_name: Some(INGEST_LOCK_NAME.to_string()),
        });
    }

    let result = (|| {
        // 2. Derive display name
        let display_name =
            validator::derive_display_name(path, display_name_override, production_mode);

        // 3. Create document record
        let document_id = format!("doc_{}", &Uuid::new_v4().to_string().replace('-', "")[..12]);
        let doc = Document {
            document_id: document_id.clone(),
            display_name: display_name.clone(),
            file_path: path.to_path_buf(),
            file_type: file_type.clone(),
            file_size_bytes: file_size,
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
                let _ = db.update_document_status(
                    &document_id,
                    DocumentStatus::Failed,
                    Some(error_info),
                );
                Err(e)
            }
        }
    })();

    let release_result = db.release_lock(INGEST_LOCK_NAME, &lock_owner_id);

    match (result, release_result) {
        (Err(err), _) => Err(err),
        (Ok(_), Err(release_err)) => Err(release_err),
        (Ok(value), Ok(())) => Ok(value),
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

/// Retry a failed document: reset to pending, clear error, and reset retry_count to 0
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
    use providers::EmbeddingProvider;
    use std::fs;
    use storage::Database;

    struct TestProvider;

    impl EmbeddingProvider for TestProvider {
        fn embed(&self, _text: &str) -> Result<Vec<f32>, HarnessError> {
            Ok(vec![0.1, 0.2, 0.3])
        }

        fn model_id(&self) -> &str {
            "test-model"
        }

        fn provider_id(&self) -> &str {
            "test-provider"
        }
    }

    fn test_db() -> Database {
        Database::open_memory().expect("failed to open in-memory DB")
    }

    fn temp_txt_file(prefix: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "aiharness_ingest_{}_{}.txt",
            prefix,
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::write(&path, "hello world\nthis is a test file").unwrap();
        path
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

    #[test]
    fn test_ingest_lock_conflict_upserts_backlog_and_returns_operation_in_progress() {
        let db = test_db();
        db.try_acquire_lock(INGEST_LOCK_NAME, "other-owner")
            .unwrap();

        let path = temp_txt_file("lock_conflict");
        let provider = TestProvider;
        let config = IngestConfig::default();

        let err = ingest(&db, &provider, &config, &path, Some("queued-doc"), false).unwrap_err();
        assert!(matches!(err, HarnessError::OperationInProgress { .. }));

        assert_eq!(db.ingest_backlog_count().unwrap(), 1);
        assert_eq!(
            db.ingest_backlog_display_name_for_source(&path).unwrap(),
            Some("queued-doc".to_string())
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_enqueue_ingest_adds_queued_item() {
        let db = test_db();
        let config = IngestConfig::default();
        let path = temp_txt_file("enqueue");

        enqueue_ingest(&db, &config, &path, Some("queued-doc")).unwrap();

        assert_eq!(db.ingest_backlog_count().unwrap(), 1);
        assert_eq!(
            db.ingest_backlog_display_name_for_source(&path).unwrap(),
            Some("queued-doc".to_string())
        );
        assert_eq!(
            db.ingest_backlog_status_for_source(&path).unwrap(),
            Some("queued".to_string())
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_ingest_next_processes_claimed_item_and_marks_done() {
        let db = test_db();
        let config = IngestConfig::default();
        let provider = TestProvider;
        let path = temp_txt_file("next_success");

        enqueue_ingest(&db, &config, &path, Some("next-doc")).unwrap();

        let result = ingest_next(&db, &provider, &config, false).unwrap();
        match result {
            IngestNextResult::Ingested(ingested) => {
                assert_eq!(ingested.status, DocumentStatus::Ready);
                assert_eq!(ingested.display_name, "next-doc");
            }
            IngestNextResult::Empty => panic!("expected an ingested item"),
        }

        assert_eq!(
            db.ingest_backlog_status_for_source(&path).unwrap(),
            Some("done".to_string())
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_ingest_next_empty_queue_returns_empty() {
        let db = test_db();
        let config = IngestConfig::default();
        let provider = TestProvider;

        let result = ingest_next(&db, &provider, &config, false).unwrap();
        assert!(matches!(result, IngestNextResult::Empty));
    }
}
