use chrono::{DateTime, Utc};
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
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use storage::tags::{TagEntityType, TagRecord};
use storage::Database;
use uuid::Uuid;

const INGEST_LOCK_NAME: &str = "ingest_pipeline";
const INGEST_LOCK_RETRY_AFTER_SECONDS: u32 = 5;

struct SourceLifecycle {
    path: PathBuf,
    hash: String,
    modified_at: Option<DateTime<Utc>>,
}

struct PipelineOutput {
    chunk_count: u32,
    chunk_ids: Vec<ChunkId>,
}

/// Intermediate result from extraction and chunking, before storage.
struct ExtractionOutput {
    chunks: Vec<Chunk>,
    chunk_inputs: Vec<chunker::ChunkInput>,
    extraction: extractor::ExtractionResult,
}

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
    let lifecycle = source_lifecycle(path)?;

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
        // Recheck under the ingest lock so unchanged re-ingest cannot race with writes.
        if let Some(existing) = db.get_document_by_file_path(&lifecycle.path)? {
            if existing.source_hash.as_deref() == Some(lifecycle.hash.as_str()) {
                // Unchanged — skip re-ingest.
                return Ok(IngestResult {
                    document_id: existing.document_id,
                    display_name: existing.display_name,
                    status: existing.status,
                    chunk_count: existing.chunk_count,
                });
            }

            // PR 5: Changed source — reuse document_id, replace chunks.
            return handle_changed_source(db, provider, config, &existing, &lifecycle, &file_type);
        }

        // 2. Derive display name
        let display_name =
            validator::derive_display_name(&lifecycle.path, display_name_override, production_mode);

        // 3. Create document record
        let document_id = DocumentId::from(format!(
            "doc_{}",
            &Uuid::new_v4().to_string().replace('-', "")[..12]
        ));
        let doc = Document {
            document_id: document_id.clone(),
            display_name: display_name.clone(),
            file_path: lifecycle.path.clone(),
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
            source_hash: None,
            ingested_at: None,
            file_modified_at: None,
        };
        db.insert_document(&doc)?;

        // 4. Mark processing
        db.update_document_status(&document_id, DocumentStatus::Processing, None)?;

        // Run the rest of the pipeline; on any error, clean up and mark failed
        match run_pipeline(
            db,
            provider,
            config,
            &document_id,
            &lifecycle.path,
            &file_type,
        ) {
            Ok(output) => {
                // Success — store source lifecycle metadata and auto-tags before marking ready.
                db.update_document_lifecycle(
                    &document_id,
                    &lifecycle.hash,
                    Utc::now(),
                    lifecycle.modified_at,
                )?;
                apply_auto_tags(db, &document_id, &output.chunk_ids, &lifecycle.path)?;
                db.update_document_status(&document_id, DocumentStatus::Ready, None)?;
                db.update_document_chunk_count(&document_id, output.chunk_count)?;

                Ok(IngestResult {
                    document_id,
                    display_name,
                    status: DocumentStatus::Ready,
                    chunk_count: output.chunk_count,
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

/// Extract text and produce chunks with assigned IDs, without persisting.
fn extract_and_chunk(
    config: &IngestConfig,
    document_id: &str,
    path: &Path,
    file_type: &common::FileType,
) -> Result<ExtractionOutput, CiteError> {
    let extraction = extractor::extract_text(path, file_type)?;
    if extraction.pages.is_empty() {
        return Ok(ExtractionOutput {
            chunks: Vec::new(),
            chunk_inputs: Vec::new(),
            extraction,
        });
    }

    let pages: Vec<chunker::PageText> = extraction
        .pages
        .iter()
        .map(|p| chunker::PageText {
            page: p.page,
            text: p.text.clone(),
        })
        .collect();

    let chunk_inputs = chunker::chunk_text(
        &pages,
        config.chunk_size_chars,
        config.chunk_overlap_chars,
        config.min_chunk_chars,
    )?;

    if chunk_inputs.is_empty() {
        return Ok(ExtractionOutput {
            chunks: Vec::new(),
            chunk_inputs,
            extraction,
        });
    }

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

    Ok(ExtractionOutput {
        chunks,
        chunk_inputs,
        extraction,
    })
}

/// Compute embeddings for a slice of chunks.
#[allow(clippy::type_complexity)]
fn compute_embeddings<'a>(
    provider: &'a dyn EmbeddingProvider,
    chunks: &[Chunk],
) -> Result<Vec<(String, Vec<f32>, &'a str, &'a str)>, CiteError> {
    let mut embeddings = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        let vector = provider.embed(&chunk.text)?;
        embeddings.push((
            chunk.chunk_id.to_string(),
            vector,
            provider.model_id(),
            provider.provider_id(),
        ));
    }
    Ok(embeddings)
}

/// Handle changed-source re-ingest: reuse the existing document_id, replace
/// chunks/embeddings atomically, and mark only changed chunks with
/// `status:changed`. On failure, preserves the old ready representation.
fn handle_changed_source(
    db: &Database,
    provider: &dyn EmbeddingProvider,
    config: &IngestConfig,
    existing: &Document,
    lifecycle: &SourceLifecycle,
    file_type: &common::FileType,
) -> Result<IngestResult, CiteError> {
    let document_id = existing.document_id.clone();

    // Mark processing — old data is still intact.
    db.update_document_status(&document_id, DocumentStatus::Processing, None)?;

    // 1. Extract and chunk new content (no DB writes yet).
    let ext = match extract_and_chunk(config, &document_id, &lifecycle.path, file_type) {
        Ok(ext) => ext,
        Err(e) => {
            // Extraction failed — preserve old representation, mark failed.
            let error_info = ErrorInfo {
                code: e.code().to_string(),
                message: e.message(),
            };
            let _ =
                db.update_document_status(&document_id, DocumentStatus::Failed, Some(error_info));
            return Err(e);
        }
    };

    // 2. Compute embeddings for new chunks (no DB writes yet).
    let embeddings = match compute_embeddings(provider, &ext.chunks) {
        Ok(e) => e,
        Err(e) => {
            let error_info = ErrorInfo {
                code: e.code().to_string(),
                message: e.message(),
            };
            let _ =
                db.update_document_status(&document_id, DocumentStatus::Failed, Some(error_info));
            return Err(e);
        }
    };

    // 3. Build text-hash map from old chunks for change detection.
    let old_chunks = db.get_chunks_for_document(&document_id)?;
    let mut old_text_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for chunk in &old_chunks {
        let hash = format!("sha256:{:x}", Sha256::digest(chunk.text.as_bytes()));
        *old_text_counts.entry(hash).or_insert(0) += 1;
    }

    // 4. Detect which new chunks changed.
    let mut changed_indices: Vec<usize> = Vec::new();
    let mut remaining_counts = old_text_counts.clone();
    for (i, chunk) in ext.chunks.iter().enumerate() {
        let hash = format!("sha256:{:x}", Sha256::digest(chunk.text.as_bytes()));
        let count = remaining_counts.get_mut(&hash);
        match count {
            Some(c) if *c > 0 => {
                *c -= 1;
            }
            _ => {
                changed_indices.push(i);
            }
        }
    }

    // 5. Atomically replace old data with new data.
    //    If this fails, the transaction rolls back and old data survives.
    db.replace_chunks_for_document(&document_id, &ext.chunks, &embeddings)?;

    // 6. Apply chunk-local status:changed for changed/new chunks.
    for &idx in &changed_indices {
        let tag = TagRecord::new("status", "changed").expect("valid status:changed tag");
        db.set_tag_engine(TagEntityType::Chunk, &ext.chunks[idx].chunk_id, &tag)?;
    }

    // 7. Apply auto-tags (source_kind, workspace, type).
    let chunk_ids: Vec<ChunkId> = ext.chunks.iter().map(|c| c.chunk_id.clone()).collect();
    apply_auto_tags(db, &document_id, &chunk_ids, &lifecycle.path)?;

    // 8. Build hierarchy.
    if config.build_hierarchy {
        build_hierarchy_for_chunks(
            db,
            &document_id,
            file_type,
            &ext.extraction,
            &ext.chunk_inputs,
            &ext.chunks,
        )?;
    }

    // 9. Update lifecycle, chunk count, and mark ready.
    db.update_document_lifecycle(
        &document_id,
        &lifecycle.hash,
        Utc::now(),
        lifecycle.modified_at,
    )?;
    let chunk_count = ext.chunks.len() as u32;
    db.update_document_chunk_count(&document_id, chunk_count)?;
    db.update_document_status(&document_id, DocumentStatus::Ready, None)?;

    Ok(IngestResult {
        document_id,
        display_name: existing.display_name.clone(),
        status: DocumentStatus::Ready,
        chunk_count,
    })
}

/// Internal pipeline steps (extraction → chunking → storage → embedding)
fn run_pipeline(
    db: &Database,
    provider: &dyn EmbeddingProvider,
    config: &IngestConfig,
    document_id: &str,
    path: &Path,
    file_type: &common::FileType,
) -> Result<PipelineOutput, CiteError> {
    let ext = extract_and_chunk(config, document_id, path, file_type)?;
    if ext.chunks.is_empty() {
        return Ok(PipelineOutput {
            chunk_count: 0,
            chunk_ids: Vec::new(),
        });
    }

    let chunk_count = ext.chunks.len() as u32;
    db.insert_chunks(document_id, &ext.chunks)?;

    let embeddings = compute_embeddings(provider, &ext.chunks)?;
    db.insert_embeddings(&embeddings)?;

    // Build hierarchy if enabled and file is markdown
    if config.build_hierarchy {
        if matches!(file_type, common::FileType::Md) {
            // Reconstruct full text from pages for heading extraction
            let full_text: String = ext
                .extraction
                .pages
                .iter()
                .map(|p| p.text.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            let headings = extract_headings(&full_text);
            let chunk_offsets: Vec<usize> = ext
                .chunk_inputs
                .iter()
                .map(|c| c.offset_start as usize)
                .collect();
            let hierarchy = build_hierarchy(document_id, &headings, &chunk_offsets);

            let chunk_ids: Vec<String> =
                ext.chunks.iter().map(|c| c.chunk_id.to_string()).collect();

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

            for (ci, c) in ext.chunk_inputs.iter().enumerate() {
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
            let chunk_ids: Vec<String> =
                ext.chunks.iter().map(|c| c.chunk_id.to_string()).collect();
            let topic_id = format!("topic_{}_0", document_id);
            db.insert_topic(&topic_id, document_id, "Untitled", None)?;
            for chunk_id in &chunk_ids {
                db.set_chunk_hierarchy(chunk_id, &topic_id, None)?;
            }
        }
    }

    let chunk_ids = ext
        .chunks
        .iter()
        .map(|chunk| chunk.chunk_id.clone())
        .collect();
    Ok(PipelineOutput {
        chunk_count,
        chunk_ids,
    })
}

/// Build hierarchy (topics/concepts) for an existing set of chunks.
/// Reuses the same logic as `run_pipeline` but operates on pre-built data.
fn build_hierarchy_for_chunks(
    db: &Database,
    document_id: &str,
    file_type: &common::FileType,
    extraction: &extractor::ExtractionResult,
    chunk_inputs: &[chunker::ChunkInput],
    chunks: &[Chunk],
) -> Result<(), CiteError> {
    if matches!(file_type, common::FileType::Md) {
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
        let mut current_topic_id: Option<String> = topic_boundaries.first().map(|b| b.1.clone());

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
        let chunk_ids: Vec<String> = chunks.iter().map(|c| c.chunk_id.to_string()).collect();
        let topic_id = format!("topic_{}_0", document_id);
        db.insert_topic(&topic_id, document_id, "Untitled", None)?;
        for chunk_id in &chunk_ids {
            db.set_chunk_hierarchy(chunk_id, &topic_id, None)?;
        }
    }
    Ok(())
}

fn source_lifecycle(path: &Path) -> Result<SourceLifecycle, CiteError> {
    let bytes = fs::read(path).map_err(|_| CiteError::FileNotFound {
        path: path.to_path_buf(),
    })?;
    let hash = format!("sha256:{:x}", Sha256::digest(&bytes));
    let metadata = fs::metadata(path).map_err(|_| CiteError::FileNotFound {
        path: path.to_path_buf(),
    })?;
    let modified_at = metadata.modified().ok().map(DateTime::<Utc>::from);
    let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    Ok(SourceLifecycle {
        path: canonical_path,
        hash,
        modified_at,
    })
}

fn apply_auto_tags(
    db: &Database,
    document_id: &DocumentId,
    chunk_ids: &[ChunkId],
    path: &Path,
) -> Result<(), CiteError> {
    let tags = auto_tags(path);
    for tag in &tags {
        db.set_tag_engine(TagEntityType::Document, document_id, tag)?;
        for chunk_id in chunk_ids {
            db.set_tag_engine(TagEntityType::Chunk, chunk_id, tag)?;
        }
    }
    Ok(())
}

fn auto_tags(path: &Path) -> Vec<TagRecord> {
    let mut tags = vec![
        TagRecord::new("source_kind", "document").expect("valid source_kind tag"),
        TagRecord::new("workspace", workspace_name()).expect("valid workspace tag"),
    ];

    if let Some(document_type) = openspec_document_type(path) {
        tags.push(TagRecord::new("type", document_type).expect("valid type tag"));
    }

    tags
}

fn workspace_name() -> String {
    std::env::current_dir()
        .ok()
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "default".to_string())
}

fn openspec_document_type(path: &Path) -> Option<&'static str> {
    let path_text = path.to_string_lossy().replace('\\', "/");
    let mappings = [
        ("/openspec/prd/", "prd"),
        ("/openspec/specs/", "spec"),
        ("/openspec/architecture/", "architecture"),
        ("/openspec/guides/", "guide"),
        ("/openspec/rfc/", "rfc"),
    ];

    mappings
        .iter()
        .find_map(|(needle, document_type)| path_text.contains(needle).then_some(*document_type))
}

/// Clean up partial data from a failed ingestion
fn cleanup_partial(db: &Database, document_id: &str) -> Result<(), CiteError> {
    // Delete embeddings first (FK dependency)
    if let Err(e) = db.delete_embeddings_for_document(document_id) {
        eprintln!("Warning: failed to delete embeddings for {document_id}: {e}");
    }
    // Delete chunk tags before deleting chunks because tags are not FK-backed.
    if let Err(e) = db.delete_tags_for_chunks_of_document(document_id) {
        eprintln!("Warning: failed to delete chunk tags for {document_id}: {e}");
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
    use std::cell::Cell;
    use std::fs;
    use storage::tags::TagEntityType;
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

    struct CountingProvider {
        calls: Cell<usize>,
    }

    impl CountingProvider {
        fn new() -> Self {
            Self {
                calls: Cell::new(0),
            }
        }

        fn calls(&self) -> usize {
            self.calls.get()
        }
    }

    impl EmbeddingProvider for CountingProvider {
        fn embed(&self, _text: &str) -> Result<Vec<f32>, CiteError> {
            self.calls.set(self.calls.get() + 1);
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
        temp_txt_file_with_content(prefix, "hello world\nthis is a test file")
    }

    fn temp_txt_file_with_content(prefix: &str, content: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "aicite_ingest_{}_{}.txt",
            prefix,
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::write(&path, content).unwrap();
        path
    }

    fn temp_openspec_file(kind: &str, content: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "aicite_ingest_openspec_{}_{}",
            kind,
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let dir = root.join("openspec").join(kind).join("active");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("example.md");
        fs::write(&path, content).unwrap();
        path
    }

    fn row_count(db: &Database, table: &str) -> i64 {
        db.conn()
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
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
    fn test_ingest_stores_lifecycle_and_skips_unchanged_reingest() {
        let db = test_db();
        let provider = CountingProvider::new();
        let config = IngestConfig::default();
        let path = temp_txt_file_with_content("lifecycle_skip", "same content for both ingests");

        let first = ingest(&db, &provider, &config, &path, Some("skip-doc"), false).unwrap();
        let doc = db
            .get_document(&first.document_id)
            .unwrap()
            .expect("document");
        assert!(doc
            .source_hash
            .as_deref()
            .unwrap_or_default()
            .starts_with("sha256:"));
        assert!(doc.ingested_at.is_some());
        assert!(doc.file_modified_at.is_some());
        assert_eq!(doc.file_path, path.canonicalize().unwrap());
        assert!(db
            .get_document_by_file_path(&doc.file_path)
            .unwrap()
            .is_some());

        let calls_after_first = provider.calls();
        let chunks_after_first = row_count(&db, "chunks");
        let second = ingest(&db, &provider, &config, &path, Some("skip-doc"), false).unwrap();
        assert_eq!(second.document_id, first.document_id);
        assert_eq!(provider.calls(), calls_after_first);
        assert_eq!(row_count(&db, "documents"), 1);
        assert_eq!(row_count(&db, "chunks"), chunks_after_first);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_ingest_assigns_openspec_auto_tags_to_document_and_chunks() {
        let db = test_db();
        let provider = CountingProvider::new();
        let config = IngestConfig::default();
        let path = temp_openspec_file("rfc", "# RFC\n\nOpenSpec auto tags should apply.");

        let result = ingest(&db, &provider, &config, &path, None, false).unwrap();
        let document_tags = db
            .list_tags(TagEntityType::Document, &result.document_id)
            .unwrap();
        assert!(document_tags
            .iter()
            .any(|tag| tag.key == "source_kind" && tag.value == "document"));
        assert!(document_tags.iter().any(|tag| tag.key == "workspace"));
        assert!(document_tags
            .iter()
            .any(|tag| tag.key == "type" && tag.value == "rfc"));
        assert!(!document_tags.iter().any(|tag| tag.key == "status"));

        let chunk_id: String = db
            .conn()
            .query_row(
                "SELECT chunk_id FROM chunks WHERE document_id = ?1 LIMIT 1",
                [result.document_id.as_ref()],
                |row| row.get(0),
            )
            .unwrap();
        let chunk_tags = db.list_tags(TagEntityType::Chunk, &chunk_id).unwrap();
        assert!(chunk_tags
            .iter()
            .any(|tag| tag.key == "source_kind" && tag.value == "document"));
        assert!(chunk_tags.iter().any(|tag| tag.key == "workspace"));
        assert!(chunk_tags
            .iter()
            .any(|tag| tag.key == "type" && tag.value == "rfc"));
        assert!(!chunk_tags.iter().any(|tag| tag.key == "status"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_cleanup_partial_removes_chunk_tags() {
        let db = test_db();
        let document_id = DocumentId::from("cleanup-doc");
        let doc = Document {
            document_id: document_id.clone(),
            display_name: "cleanup.txt".to_string(),
            file_path: Path::new("/tmp/cleanup.txt").to_path_buf(),
            file_type: common::FileType::Txt,
            file_size_bytes: 100,
            status: DocumentStatus::Processing,
            chunk_count: 1,
            retry_count: 0,
            max_retry_count: 3,
            next_retry_at: None,
            error: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_hash: None,
            ingested_at: None,
            file_modified_at: None,
        };
        db.insert_document(&doc).unwrap();
        let chunk = Chunk {
            chunk_id: ChunkId::from("cleanup-chunk"),
            document_id,
            section_id: None,
            chunk_index: 0,
            text: "partial chunk".to_string(),
            page: None,
            offset_start: None,
            offset_end: None,
            created_at: Utc::now(),
        };
        db.insert_chunks("cleanup-doc", &[chunk]).unwrap();
        db.set_tag_engine(
            TagEntityType::Chunk,
            "cleanup-chunk",
            &TagRecord::new("status", "changed").unwrap(),
        )
        .unwrap();

        cleanup_partial(&db, "cleanup-doc").unwrap();

        assert_eq!(row_count(&db, "chunks"), 0);
        assert!(db
            .list_tags(TagEntityType::Chunk, "cleanup-chunk")
            .unwrap()
            .is_empty());
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
            source_hash: None,
            ingested_at: None,
            file_modified_at: None,
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

    // -----------------------------------------------------------------------
    // PR 5: Changed re-ingest replacement + chunk-local status:changed
    // -----------------------------------------------------------------------

    #[test]
    fn test_changed_source_reuses_document_id() {
        let db = test_db();
        let provider = CountingProvider::new();
        let config = IngestConfig::default();
        let path = temp_txt_file_with_content("changed_reuse", "original content for reuse test");

        let first = ingest(&db, &provider, &config, &path, Some("reuse-doc"), false).unwrap();
        let first_chunks = row_count(&db, "chunks");
        assert!(first_chunks > 0);

        // Change the file content.
        fs::write(&path, "completely new content that differs from original").unwrap();

        let second = ingest(&db, &provider, &config, &path, Some("reuse-doc"), false).unwrap();

        // Same document_id reused.
        assert_eq!(second.document_id, first.document_id);
        // Only one document exists.
        assert_eq!(row_count(&db, "documents"), 1);
        // Document is ready.
        let doc = db
            .get_document(&first.document_id)
            .unwrap()
            .expect("document");
        assert_eq!(doc.status, DocumentStatus::Ready);
        // Embeddings were called for new content.
        assert!(provider.calls() > first_chunks as usize);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_changed_source_detects_content_hash_changes() {
        let db = test_db();
        let provider = TestProvider;
        let config = IngestConfig::default();

        // File with 3 lines → likely 3 chunks.
        let content_v1 = "line one alpha\nline two beta\nline three gamma";
        let path = temp_txt_file_with_content("changed_detect", content_v1);

        let first = ingest(&db, &provider, &config, &path, Some("detect-doc"), false).unwrap();

        // Get old chunk IDs and set status:changed on one of them.
        let old_chunk_ids: Vec<String> = db
            .conn()
            .prepare("SELECT chunk_id FROM chunks WHERE document_id = ?1")
            .unwrap()
            .query_map([first.document_id.as_ref()], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<String>, _>>()
            .unwrap();
        assert!(!old_chunk_ids.is_empty());

        // Set status:changed on a chunk (to verify it gets cleared).
        db.set_tag_engine(
            TagEntityType::Chunk,
            &old_chunk_ids[0],
            &TagRecord::new("status", "changed").unwrap(),
        )
        .unwrap();

        // Change file content — all chunks should be different.
        fs::write(&path, "entirely new content here for the changed source").unwrap();

        let second = ingest(&db, &provider, &config, &path, Some("detect-doc"), false).unwrap();
        assert_eq!(second.document_id, first.document_id);

        // Verify new chunks have status:changed tags (because content is entirely new).
        let new_chunk_ids: Vec<String> = db
            .conn()
            .prepare("SELECT chunk_id FROM chunks WHERE document_id = ?1")
            .unwrap()
            .query_map([second.document_id.as_ref()], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<String>, _>>()
            .unwrap();

        for chunk_id in &new_chunk_ids {
            let tags = db.list_tags(TagEntityType::Chunk, chunk_id).unwrap();
            assert!(
                tags.iter()
                    .any(|t| t.key == "status" && t.value == "changed"),
                "Chunk {chunk_id} should have status:changed"
            );
        }

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_changed_source_handles_duplicate_text_counts() {
        let db = test_db();
        let provider = TestProvider;
        let config = IngestConfig {
            chunk_size_chars: 5,
            chunk_overlap_chars: 0,
            min_chunk_chars: 1,
            ..IngestConfig::default()
        };
        let path = temp_txt_file_with_content("duplicate_counts", "aaaaaaaaaabbbbb");

        let first = ingest(&db, &provider, &config, &path, Some("dupe-doc"), false).unwrap();
        assert_eq!(first.chunk_count, 3);

        // Old chunks are [aaaaa, aaaaa, bbbbb]; new chunks are
        // [aaaaa, bbbbb, bbbbb]. Multiset detection should mark only the extra
        // bbbbb as changed, not both duplicate bbbbb chunks.
        fs::write(&path, "aaaaabbbbbbbbbb").unwrap();
        let second = ingest(&db, &provider, &config, &path, Some("dupe-doc"), false).unwrap();
        assert_eq!(second.document_id, first.document_id);
        assert_eq!(second.chunk_count, 3);

        let changed_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*)
                 FROM tags t
                 JOIN chunks c ON c.chunk_id = t.entity_id
                 WHERE c.document_id = ?1
                   AND t.entity_type = 'chunk'
                   AND t.key = 'status'
                   AND t.value = 'changed'",
                [second.document_id.as_ref()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(changed_count, 1);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_changed_source_clears_stale_status_changed() {
        let db = test_db();
        let provider = TestProvider;
        let config = IngestConfig::default();

        let content = "first line alpha\nsecond line beta";
        let path = temp_txt_file_with_content("stale_clear", content);

        let first = ingest(&db, &provider, &config, &path, Some("stale-doc"), false).unwrap();

        // Manually set status:changed on a chunk.
        let chunk_id: String = db
            .conn()
            .query_row(
                "SELECT chunk_id FROM chunks WHERE document_id = ?1 LIMIT 1",
                [first.document_id.as_ref()],
                |row| row.get(0),
            )
            .unwrap();
        db.set_tag_engine(
            TagEntityType::Chunk,
            &chunk_id,
            &TagRecord::new("status", "changed").unwrap(),
        )
        .unwrap();

        // Re-ingest with SAME content (unchanged skip — hash matches).
        let second = ingest(&db, &provider, &config, &path, Some("stale-doc"), false).unwrap();
        assert_eq!(second.document_id, first.document_id);

        // The stale tag should still be there because unchanged skip doesn't
        // touch tags. This is correct behavior — unchanged skip is a no-op.
        let tags = db.list_tags(TagEntityType::Chunk, &chunk_id).unwrap();
        assert!(tags
            .iter()
            .any(|t| t.key == "status" && t.value == "changed"));

        // Now change the content and re-ingest — stale tag should be cleared.
        fs::write(&path, "new content to trigger changed re-ingest").unwrap();
        let third = ingest(&db, &provider, &config, &path, Some("stale-doc"), false).unwrap();
        assert_eq!(third.document_id, first.document_id);

        // Old chunk no longer exists (was replaced).
        let old_chunk_tags = db.list_tags(TagEntityType::Chunk, &chunk_id).unwrap();
        assert!(
            old_chunk_tags.is_empty(),
            "Old chunk tags should be cleared after replacement"
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_no_duplicate_active_documents_for_same_path() {
        let db = test_db();
        let provider = TestProvider;
        let config = IngestConfig::default();

        let path = temp_txt_file_with_content("no_dup", "original content");

        let first = ingest(&db, &provider, &config, &path, Some("no-dup"), false).unwrap();

        // Change file and re-ingest multiple times.
        fs::write(&path, "version 2 content").unwrap();
        let second = ingest(&db, &provider, &config, &path, Some("no-dup"), false).unwrap();
        fs::write(&path, "version 3 content").unwrap();
        let third = ingest(&db, &provider, &config, &path, Some("no-dup"), false).unwrap();

        // All should be the same document_id.
        assert_eq!(second.document_id, first.document_id);
        assert_eq!(third.document_id, first.document_id);

        // Only one document in the database.
        assert_eq!(row_count(&db, "documents"), 1);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_failure_preserves_last_ready_representation() {
        let db = test_db();
        let config = IngestConfig::default();

        let path = temp_txt_file_with_content("preserve", "original content for preservation");

        // First ingest succeeds.
        let good_provider = TestProvider;
        let first = ingest(
            &db,
            &good_provider,
            &config,
            &path,
            Some("preserve-doc"),
            false,
        )
        .unwrap();
        let original_chunk_count = row_count(&db, "chunks");
        let original_embedding_count = row_count(&db, "embeddings");
        assert!(original_chunk_count > 0);
        assert!(original_embedding_count > 0);

        // Change file and try to ingest with a failing provider.
        fs::write(&path, "new content that should fail to embed").unwrap();

        struct FailingProvider;
        impl EmbeddingProvider for FailingProvider {
            fn embed(&self, _text: &str) -> Result<Vec<f32>, CiteError> {
                Err(CiteError::StorageError {
                    message: "simulated embed failure".to_string(),
                })
            }
            fn model_id(&self) -> &str {
                "fail-model"
            }
            fn provider_id(&self) -> &str {
                "fail-provider"
            }
        }

        let result = ingest(
            &db,
            &FailingProvider,
            &config,
            &path,
            Some("preserve-doc"),
            false,
        );
        assert!(result.is_err());

        // Document should be marked as failed.
        let doc = db
            .get_document(&first.document_id)
            .unwrap()
            .expect("document");
        assert_eq!(doc.status, DocumentStatus::Failed);

        // Old data should be preserved (chunks and embeddings still exist).
        assert_eq!(
            row_count(&db, "chunks"),
            original_chunk_count,
            "Old chunks should survive when new pipeline fails"
        );
        assert_eq!(
            row_count(&db, "embeddings"),
            original_embedding_count,
            "Old embeddings should survive when new pipeline fails"
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_document_status_changed_ban_still_enforced() {
        let db = test_db();
        let changed = TagRecord::new("status", "changed").unwrap();

        // Engine path: rejected.
        assert!(matches!(
            db.set_tag_engine(TagEntityType::Document, "doc-1", &changed),
            Err(CiteError::InvalidParameter { .. })
        ));
        // User path: rejected.
        assert!(matches!(
            db.set_tag_user(TagEntityType::Document, "doc-1", &changed),
            Err(CiteError::InvalidParameter { .. })
        ));
    }
}
