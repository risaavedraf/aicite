use chrono::Utc;
use common::types::{Chunk, Document, DocumentStatus, ErrorInfo};
use common::CiteError;
use common::{ChunkId, DocumentId};
use config::IngestConfig;
use graph::heading_parser::extract_headings;
use graph::hierarchy::build_hierarchy;
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
    pub document_id: DocumentId,
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
) -> Result<IngestResult, CiteError> {
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
) -> Result<(), CiteError> {
    let _ = validator::validate_file(path, config.max_file_size_bytes)?;
    db.upsert_ingest_backlog(path, display_name_override)
}

pub fn ingest_next(
    db: &Database,
    provider: &dyn EmbeddingProvider,
    config: &IngestConfig,
    production_mode: bool,
) -> Result<IngestNextResult, CiteError> {
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
        Err(CiteError::OperationInProgress {
            message,
            retry_after_seconds,
            lock_name,
        }) => {
            db.requeue_ingest_backlog(&item.queue_id)?;
            Err(CiteError::OperationInProgress {
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
) -> Result<IngestResult, CiteError> {
    // 1. Validate
    let (file_type, file_size) = validator::validate_file(path, config.max_file_size_bytes)?;

    let lock_owner_id = format!("ingest_{}", Uuid::new_v4());
    if !db.try_acquire_lock(INGEST_LOCK_NAME, &lock_owner_id)? {
        if queue_on_lock_conflict {
            db.upsert_ingest_backlog(path, display_name_override)?;
        }

        return Err(CiteError::OperationInProgress {
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
        let document_id = DocumentId::from(format!(
            "doc_{}",
            &Uuid::new_v4().to_string().replace('-', "")[..12]
        ));
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
                if let Err(cleanup_err) = cleanup_partial(db, &document_id) {
                    eprintln!("Warning: cleanup failed for {document_id}: {cleanup_err}");
                }
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
) -> Result<u32, CiteError> {
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
        config.min_chunk_chars,
    )?;

    if chunk_inputs.is_empty() {
        return Ok(0);
    }

    // Convert ChunkInput → Chunk for storage
    let now = Utc::now();
    let chunks: Vec<Chunk> = chunk_inputs
        .iter()
        .map(|ci| Chunk {
            chunk_id: ChunkId::from(format!(
                "chunk_{}",
                &Uuid::new_v4().to_string().replace('-', "")[..12]
            )),
            document_id: DocumentId::from(document_id),
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
            chunk.chunk_id.to_string(),
            vector,
            provider.model_id(),
            provider.provider_id(),
        ));
    }

    // Store embeddings
    db.insert_embeddings(&embeddings)?;

    // Build hierarchy if enabled and file is markdown
    if config.build_hierarchy {
        if matches!(file_type, common::FileType::Md) {
            // Reconstruct full text from pages for heading extraction
            let full_text: String = extraction
                .pages
                .iter()
                .map(|p| p.text.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            let headings = extract_headings(&full_text);
            let chunk_offsets: Vec<usize> = chunk_inputs
                .iter()
                .map(|c| c.offset_start as usize)
                .collect();
            let hierarchy = build_hierarchy(document_id, &headings, &chunk_offsets);

            let chunk_ids: Vec<String> = chunks.iter().map(|c| c.chunk_id.to_string()).collect();

            // Insert topics and concepts
            for topic_with_concepts in &hierarchy.topics {
                db.insert_topic(
                    &topic_with_concepts.topic.topic_id,
                    document_id,
                    &topic_with_concepts.topic.name,
                    topic_with_concepts.topic.summary.as_deref(),
                )?;

                for concept_with_chunks in &topic_with_concepts.concepts {
                    db.insert_concept(
                        &concept_with_chunks.concept.concept_id,
                        &topic_with_concepts.topic.topic_id,
                        &concept_with_chunks.concept.name,
                        concept_with_chunks.concept.summary.as_deref(),
                    )?;
                }
            }

            // Assign chunks to topics/concepts
            let mut assigned: Vec<bool> = vec![false; chunk_ids.len()];
            for topic_with_concepts in &hierarchy.topics {
                for concept_with_chunks in &topic_with_concepts.concepts {
                    for &ci in &concept_with_chunks.chunk_indices {
                        if ci < chunk_ids.len() {
                            db.set_chunk_hierarchy(
                                &chunk_ids[ci],
                                &topic_with_concepts.topic.topic_id,
                                Some(&concept_with_chunks.concept.concept_id),
                            )?;
                            assigned[ci] = true;
                        }
                    }
                }
            }

            // Assign remaining chunks to topics via heading offsets
            let mut topic_boundaries: Vec<(usize, String)> = Vec::new();
            for twc in &hierarchy.topics {
                if let Some(h) = headings
                    .iter()
                    .find(|h| h.level == 2 && h.title == twc.topic.name)
                {
                    topic_boundaries.push((h.char_offset, twc.topic.topic_id.to_string()));
                }
            }
            topic_boundaries.sort_by_key(|b| b.0);

            if topic_boundaries.is_empty() && !hierarchy.topics.is_empty() {
                topic_boundaries.push((0, hierarchy.topics[0].topic.topic_id.to_string()));
            }

            let mut bp = 0usize;
            let mut current_topic_id: Option<String> =
                topic_boundaries.first().map(|b| b.1.clone());

            for (ci, c) in chunk_inputs.iter().enumerate() {
                let offset = c.offset_start as usize;
                while bp < topic_boundaries.len() && offset >= topic_boundaries[bp].0 {
                    current_topic_id = Some(topic_boundaries[bp].1.clone());
                    bp += 1;
                }
                if !assigned[ci] {
                    if let Some(ref tid) = current_topic_id {
                        db.set_chunk_hierarchy(&chunk_ids[ci], tid, None)?;
                    }
                }
            }
        } else {
            // Non-markdown: single "Untitled" topic
            let chunk_ids: Vec<String> = chunks.iter().map(|c| c.chunk_id.to_string()).collect();
            let topic_id = format!("topic_{}_0", document_id);
            db.insert_topic(&topic_id, document_id, "Untitled", None)?;
            for chunk_id in &chunk_ids {
                db.set_chunk_hierarchy(chunk_id, &topic_id, None)?;
            }
        }
    }

    Ok(chunk_count)
}

/// Clean up partial data from a failed ingestion
fn cleanup_partial(db: &Database, document_id: &str) -> Result<(), CiteError> {
    // Delete embeddings first (FK dependency)
    if let Err(e) = db.delete_embeddings_for_document(document_id) {
        eprintln!("Warning: failed to delete embeddings for {document_id}: {e}");
    }
    // Delete chunks
    if let Err(e) = db.delete_chunks_for_document(document_id) {
        eprintln!("Warning: failed to delete chunks for {document_id}: {e}");
    }
    Ok(())
}

/// Retry a failed document: reset to pending, clear error, and reset retry_count to 0
pub fn retry_document(db: &Database, document_id: &str) -> Result<Document, CiteError> {
    let doc = db
        .get_document(document_id)?
        .ok_or_else(|| CiteError::DocumentNotFound {
            document_id: document_id.to_string(),
        })?;

    if doc.status != DocumentStatus::Failed {
        return Err(CiteError::InvalidParameter {
            message: format!(
                "Document {} is not failed (status: {})",
                document_id, doc.status
            ),
        });
    }

    // Verify original file still exists
    if !doc.file_path.exists() {
        return Err(CiteError::FileNotFound {
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
        .ok_or_else(|| CiteError::DocumentNotFound {
            document_id: document_id.to_string(),
        })?;

    Ok(updated)
}

/// List all documents
pub fn list_documents(db: &Database) -> Result<Vec<Document>, CiteError> {
    db.list_documents()
}

/// Get a single document by ID
pub fn get_document(db: &Database, document_id: &str) -> Result<Document, CiteError> {
    db.get_document(document_id)?
        .ok_or_else(|| CiteError::DocumentNotFound {
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
        fn embed(&self, _text: &str) -> Result<Vec<f32>, CiteError> {
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
            "aicite_ingest_{}_{}.txt",
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
        assert!(matches!(result, Err(CiteError::DocumentNotFound { .. })));
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
            document_id: DocumentId::from("doc_test1"),
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
        assert!(matches!(result, Err(CiteError::InvalidParameter { .. })));
    }

    #[test]
    fn test_retry_document_not_found() {
        let db = test_db();
        let result = retry_document(&db, "doc_nonexistent");
        assert!(matches!(result, Err(CiteError::DocumentNotFound { .. })));
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
        assert!(matches!(err, CiteError::OperationInProgress { .. }));

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
