pub mod chunker;
pub mod extractor;
pub mod sentence_chunker;
pub mod validator;

use chrono::Utc;
use common::types::Chunk;
use common::CiteError;
use config::IngestConfig;
use graph::heading_parser::extract_headings;
use graph::hierarchy::build_hierarchy;
use storage::Database;
use uuid::Uuid;

/// Ingest handles document extraction, chunking, and embedding
pub struct Ingest;

/// Ingest a document: chunk text, store chunks, and optionally build hierarchy.
///
/// When `config.sentence_chunking` is true, uses sentence-based chunking
/// (via `sentence_chunker::chunk_by_sentence`) instead of fixed-size chunking.
///
/// When `config.build_hierarchy` is true, extracts headings from markdown
/// and builds a topic/concept hierarchy, or creates a single "Untitled" topic
/// for non-markdown files.
///
/// When both flags are false (the default), behavior is identical to v0.1.0.
///
/// Returns the list of stored chunk IDs.
pub fn ingest_document(
    db: &Database,
    document_id: &str,
    text: &str,
    file_type: &str,
    config: &IngestConfig,
) -> Result<Vec<String>, CiteError> {
    // 1. Chunk the text based on config
    let raw_chunks: Vec<chunker::ChunkInput> = if config.sentence_chunking {
        let sentence_chunks = sentence_chunker::chunk_by_sentence(text, config.min_chunk_chars);
        sentence_chunks
            .into_iter()
            .enumerate()
            .map(|(i, sc)| chunker::ChunkInput {
                chunk_index: i as u32,
                text: sc.text,
                page: None,
                offset_start: sc.offset_start as u32,
                offset_end: sc.offset_end as u32,
            })
            .collect()
    } else {
        let pages = vec![chunker::PageText {
            page: 1,
            text: text.to_string(),
        }];
        chunker::chunk_text(
            &pages,
            config.chunk_size_chars,
            config.chunk_overlap_chars,
            config.min_chunk_chars,
        )?
    };

    if raw_chunks.is_empty() {
        return Ok(Vec::new());
    }

    // 2. Convert to storage Chunks with generated IDs
    let now = Utc::now();
    let storage_chunks: Vec<Chunk> = raw_chunks
        .iter()
        .map(|c| Chunk {
            chunk_id: Uuid::new_v4().to_string(),
            document_id: document_id.to_string(),
            section_id: None,
            chunk_index: c.chunk_index,
            text: c.text.clone(),
            page: c.page,
            offset_start: Some(c.offset_start),
            offset_end: Some(c.offset_end),
            created_at: now,
        })
        .collect();

    let chunk_ids: Vec<String> = storage_chunks.iter().map(|c| c.chunk_id.clone()).collect();

    db.insert_chunks(document_id, &storage_chunks)?;

    // 3. Build and store hierarchy if enabled
    if config.build_hierarchy {
        if file_type == "md" || file_type == "markdown" {
            let headings = extract_headings(text);
            let chunk_offsets: Vec<usize> =
                raw_chunks.iter().map(|c| c.offset_start as usize).collect();
            let hierarchy = build_hierarchy(document_id, &headings, &chunk_offsets);

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

            // Assign all chunks to topics/concepts.
            // build_hierarchy stores chunk indices per concept, but when a
            // topic has no H3 concepts the chunk indices are only reflected in
            // topic.chunk_count. We recompute topic boundaries from headings
            // to ensure every chunk gets a topic_id.

            // 1) Collect concept-level assignments
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

            // 2) Assign remaining chunks to topics via heading offsets
            let mut topic_boundaries: Vec<(usize, String)> = Vec::new();
            for twc in &hierarchy.topics {
                if let Some(h) = headings
                    .iter()
                    .find(|h| h.level == 2 && h.title == twc.topic.name)
                {
                    topic_boundaries.push((h.char_offset, twc.topic.topic_id.clone()));
                }
            }
            topic_boundaries.sort_by_key(|b| b.0);

            // Fallback: no H2 boundaries (e.g. only H1 headings)
            if topic_boundaries.is_empty() && !hierarchy.topics.is_empty() {
                topic_boundaries.push((0, hierarchy.topics[0].topic.topic_id.clone()));
            }

            let mut bp = 0usize;
            let mut current_topic_id: Option<String> =
                topic_boundaries.first().map(|b| b.1.clone());

            for (ci, c) in raw_chunks.iter().enumerate() {
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
            let topic_id = format!("topic_{}_0", document_id);
            db.insert_topic(&topic_id, document_id, "Untitled", None)?;

            for chunk_id in &chunk_ids {
                db.set_chunk_hierarchy(chunk_id, &topic_id, None)?;
            }
        }
    }

    Ok(chunk_ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use common::types::{Document, DocumentStatus, FileType as CommonFileType};
    use config::IngestConfig;
    use std::path::PathBuf;

    fn insert_doc(db: &Database, id: &str) {
        let doc = Document {
            document_id: id.to_string(),
            display_name: format!("{}.txt", id),
            file_path: PathBuf::from(format!("/docs/{}.txt", id)),
            file_type: CommonFileType::Txt,
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
    }

    fn default_config() -> IngestConfig {
        IngestConfig::default()
    }

    #[test]
    fn test_ingest_document_basic() {
        let db = Database::open_memory().unwrap();
        insert_doc(&db, "doc-1");
        let config = default_config();

        let chunk_ids = ingest_document(
            &db,
            "doc-1",
            "Hello world. This is test text.",
            "txt",
            &config,
        )
        .unwrap();
        assert!(!chunk_ids.is_empty());

        // Verify chunks were stored
        let count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM chunks WHERE document_id = 'doc-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, chunk_ids.len() as i64);
    }

    #[test]
    fn test_ingest_document_empty_text() {
        let db = Database::open_memory().unwrap();
        insert_doc(&db, "doc-1");
        let config = default_config();

        let chunk_ids = ingest_document(&db, "doc-1", "", "txt", &config).unwrap();
        assert!(chunk_ids.is_empty());
    }

    #[test]
    fn test_ingest_document_default_no_hierarchy() {
        let db = Database::open_memory().unwrap();
        insert_doc(&db, "doc-1");
        let config = default_config();
        assert!(!config.build_hierarchy);

        let chunk_ids = ingest_document(
            &db,
            "doc-1",
            "Some long enough text to produce chunks. More text follows here.",
            "txt",
            &config,
        )
        .unwrap();

        // No topics should be created
        let topic_count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM topics", [], |row| row.get(0))
            .unwrap();
        assert_eq!(topic_count, 0);

        // Chunks should have no topic_id set
        let topic_on_chunk: Option<String> = db
            .conn()
            .query_row(
                "SELECT topic_id FROM chunks WHERE chunk_id = ?1",
                [chunk_ids[0].as_str()],
                |row| row.get(0),
            )
            .unwrap();
        assert!(topic_on_chunk.is_none());
    }

    #[test]
    fn test_ingest_document_sentence_chunking() {
        let db = Database::open_memory().unwrap();
        insert_doc(&db, "doc-sc");
        let config = IngestConfig {
            sentence_chunking: true,
            min_chunk_chars: 5,
            max_chunk_chars: 200,
            ..default_config()
        };

        let text = "Hello world. This is a test. How are you doing today?";
        let chunk_ids = ingest_document(&db, "doc-sc", text, "txt", &config).unwrap();

        // Should produce sentence-based chunks
        assert!(!chunk_ids.is_empty());

        let count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM chunks WHERE document_id = 'doc-sc'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, chunk_ids.len() as i64);
    }

    #[test]
    fn test_ingest_document_hierarchy_markdown() {
        let db = Database::open_memory().unwrap();
        insert_doc(&db, "doc-md");
        let config = IngestConfig {
            build_hierarchy: true,
            ..default_config()
        };

        let md_text = "## API Gateway\n\nThe gateway handles routing.\n\n### Routing\n\nRoutes are configured via YAML.\n\n### Auth\n\nAuthentication uses JWT tokens.\n\n## Database\n\nPostgreSQL is used for storage.";

        let chunk_ids = ingest_document(&db, "doc-md", md_text, "md", &config).unwrap();
        assert!(!chunk_ids.is_empty());

        // Verify topics were created
        let topics = db.list_topics_by_document("doc-md").unwrap();
        assert!(
            topics.len() >= 2,
            "Expected at least 2 topics, got {}",
            topics.len()
        );
        let topic_names: Vec<&str> = topics.iter().map(|t| t.name.as_str()).collect();
        assert!(topic_names.contains(&"API Gateway"));
        assert!(topic_names.contains(&"Database"));

        // Verify concepts were created for API Gateway
        let api_topic = topics.iter().find(|t| t.name == "API Gateway").unwrap();
        let concepts = db.list_concepts_by_topic(&api_topic.topic_id).unwrap();
        assert_eq!(concepts.len(), 2);
        let concept_names: Vec<&str> = concepts.iter().map(|c| c.name.as_str()).collect();
        assert!(concept_names.contains(&"Routing"));
        assert!(concept_names.contains(&"Auth"));

        // Verify chunks have topic_id set
        let first_chunk_topic: Option<String> = db
            .conn()
            .query_row(
                "SELECT topic_id FROM chunks WHERE chunk_id = ?1",
                [chunk_ids[0].as_str()],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            first_chunk_topic.is_some(),
            "Chunk should have topic_id set when hierarchy is enabled"
        );
    }

    #[test]
    fn test_ingest_document_hierarchy_markdown_ext() {
        let db = Database::open_memory().unwrap();
        insert_doc(&db, "doc-mkdn");
        let config = IngestConfig {
            build_hierarchy: true,
            ..default_config()
        };

        let md_text = "## Section One\n\nContent here.\n\n## Section Two\n\nMore content.";

        // Test with "markdown" file type (not just "md")
        let chunk_ids = ingest_document(&db, "doc-mkdn", md_text, "markdown", &config).unwrap();
        assert!(!chunk_ids.is_empty());

        let topics = db.list_topics_by_document("doc-mkdn").unwrap();
        assert_eq!(topics.len(), 2);
    }

    #[test]
    fn test_ingest_document_hierarchy_non_markdown() {
        let db = Database::open_memory().unwrap();
        insert_doc(&db, "doc-txt");
        let config = IngestConfig {
            build_hierarchy: true,
            ..default_config()
        };

        let text = "This is plain text content. It does not have any markdown headings at all.";

        let chunk_ids = ingest_document(&db, "doc-txt", text, "txt", &config).unwrap();
        assert!(!chunk_ids.is_empty());

        // Should create a single "Untitled" topic
        let topics = db.list_topics_by_document("doc-txt").unwrap();
        assert_eq!(topics.len(), 1);
        assert_eq!(topics[0].name, "Untitled");

        // All chunks should be assigned to this topic
        for chunk_id in &chunk_ids {
            let topic_id: Option<String> = db
                .conn()
                .query_row(
                    "SELECT topic_id FROM chunks WHERE chunk_id = ?1",
                    [chunk_id.as_str()],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(
                topic_id.as_deref(),
                Some(topics[0].topic_id.as_str()),
                "All chunks should be assigned to the Untitled topic"
            );
        }
    }

    #[test]
    fn test_ingest_document_sentence_chunking_with_hierarchy() {
        let db = Database::open_memory().unwrap();
        insert_doc(&db, "doc-both");
        let config = IngestConfig {
            sentence_chunking: true,
            build_hierarchy: true,
            min_chunk_chars: 5,
            max_chunk_chars: 200,
            ..default_config()
        };

        let md_text = "## Introduction\n\nWelcome to the project. This is a brief overview of what we do.\n\n## Details\n\nHere are the technical details. Everything is explained clearly.";

        let chunk_ids = ingest_document(&db, "doc-both", md_text, "md", &config).unwrap();
        assert!(!chunk_ids.is_empty());

        // Verify topics exist
        let topics = db.list_topics_by_document("doc-both").unwrap();
        assert!(
            topics.len() >= 2,
            "Expected at least 2 topics from markdown headings"
        );

        // Verify chunks are linked to topics
        for chunk_id in &chunk_ids {
            let topic_id: Option<String> = db
                .conn()
                .query_row(
                    "SELECT topic_id FROM chunks WHERE chunk_id = ?1",
                    [chunk_id.as_str()],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(
                topic_id.is_some(),
                "Chunk should have topic_id when hierarchy is enabled"
            );
        }
    }
}
