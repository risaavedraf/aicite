use common::types::Chunk;
use common::CiteError;
use rusqlite::params;

use crate::util::{format_dt, storage_err};
use crate::Database;

// ---------------------------------------------------------------------------
// CRUD operations
// ---------------------------------------------------------------------------

impl Database {
    /// Bulk-insert chunks for a document inside a single transaction.
    pub fn insert_chunks(&self, document_id: &str, chunks: &[Chunk]) -> Result<(), CiteError> {
        let tx = self.conn.unchecked_transaction().map_err(storage_err)?;

        for chunk in chunks {
            if chunk.document_id.as_ref() != document_id {
                return Err(CiteError::StorageError {
                    message: format!(
                        "Chunk {} belongs to document {}, expected {}",
                        chunk.chunk_id, chunk.document_id, document_id
                    ),
                });
            }

            tx.execute(
                "INSERT INTO chunks (
                    chunk_id, document_id, section_id, chunk_index,
                    text, page, offset_start, offset_end, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    chunk.chunk_id.as_ref(),
                    chunk.document_id.as_ref(),
                    chunk.section_id,
                    chunk.chunk_index as i64,
                    chunk.text,
                    chunk.page.map(|p| p as i64),
                    chunk.offset_start.map(|o| o as i64),
                    chunk.offset_end.map(|o| o as i64),
                    format_dt(&chunk.created_at),
                ],
            )
            .map_err(storage_err)?;
        }

        tx.commit().map_err(storage_err)?;
        Ok(())
    }

    /// Set the hierarchy (topic and optional concept) on an existing chunk.
    pub fn set_chunk_hierarchy(
        &self,
        chunk_id: &str,
        topic_id: &str,
        concept_id: Option<&str>,
    ) -> Result<(), CiteError> {
        self.conn
            .execute(
                "UPDATE chunks SET topic_id = ?1, concept_id = ?2 WHERE chunk_id = ?3",
                params![topic_id, concept_id, chunk_id],
            )
            .map_err(storage_err)?;
        Ok(())
    }

    /// Retrieve all chunks belonging to a document, ordered by chunk_index.
    /// Used during changed-source re-ingest to build a text-hash map for
    /// detecting which chunks changed.
    pub fn get_chunks_for_document(&self, document_id: &str) -> Result<Vec<Chunk>, CiteError> {
        let mut stmt = self
            .conn()
            .prepare(
                "SELECT chunk_id, document_id, section_id, chunk_index,
                        text, page, offset_start, offset_end, created_at
                 FROM chunks WHERE document_id = ?1 ORDER BY chunk_index ASC",
            )
            .map_err(storage_err)?;

        let rows = stmt
            .query_map(params![document_id], |row| {
                let created_at_str: String = row.get(8)?;
                let created_at = crate::util::parse_dt(&created_at_str)
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
                Ok(Chunk {
                    chunk_id: row.get::<_, String>(0)?.into(),
                    document_id: row.get::<_, String>(1)?.into(),
                    section_id: row.get(2)?,
                    chunk_index: row.get::<_, i64>(3)? as u32,
                    text: row.get(4)?,
                    page: row.get::<_, Option<i64>>(5)?.map(|p| p as u32),
                    offset_start: row.get::<_, Option<i64>>(6)?.map(|o| o as u32),
                    offset_end: row.get::<_, Option<i64>>(7)?.map(|o| o as u32),
                    created_at,
                })
            })
            .map_err(storage_err)?;

        let mut chunks = Vec::new();
        for row in rows {
            chunks.push(row.map_err(storage_err)?);
        }
        Ok(chunks)
    }

    /// Atomically replace all chunks for a document: delete old embeddings,
    /// chunk tags, and chunks, then insert new chunks and embeddings.
    /// Returns the old chunk IDs that were deleted.
    ///
    /// `new_embeddings` has the same shape as [`Database::insert_embeddings`]:
    /// `(chunk_id, vector, model_id, provider_id)`.
    pub fn replace_chunks_for_document(
        &self,
        document_id: &str,
        new_chunks: &[Chunk],
        new_embeddings: &[(String, Vec<f32>, &str, &str)],
    ) -> Result<Vec<String>, CiteError> {
        // Fetch old chunk IDs before deletion.
        let old_chunk_ids: Vec<String> = self
            .get_chunks_for_document(document_id)?
            .iter()
            .map(|c| c.chunk_id.to_string())
            .collect();

        let tx = self.conn().unchecked_transaction().map_err(storage_err)?;

        // Delete old embeddings (via chunk_id subquery).
        tx.execute(
            "DELETE FROM embeddings WHERE chunk_id IN (
                 SELECT chunk_id FROM chunks WHERE document_id = ?1
             )",
            params![document_id],
        )
        .map_err(storage_err)?;

        // Delete semantic links that reference old chunks before chunk deletion
        // so foreign-key enforcement cannot leave the replacement half-applied.
        tx.execute(
            "DELETE FROM semantic_links
             WHERE source_chunk_id IN (
                 SELECT chunk_id FROM chunks WHERE document_id = ?1
             )
             OR target_chunk_id IN (
                 SELECT chunk_id FROM chunks WHERE document_id = ?1
             )",
            params![document_id],
        )
        .map_err(storage_err)?;

        // Delete old chunk tags.
        tx.execute(
            "DELETE FROM tags WHERE entity_type = 'chunk'
               AND entity_id IN (
                 SELECT chunk_id FROM chunks WHERE document_id = ?1
               )",
            params![document_id],
        )
        .map_err(storage_err)?;

        // Delete old chunks.
        tx.execute(
            "DELETE FROM chunks WHERE document_id = ?1",
            params![document_id],
        )
        .map_err(storage_err)?;

        // Delete stale hierarchy rows for this document. New hierarchy rows may
        // reuse deterministic topic/concept IDs, so stale rows must be removed
        // inside the same replacement transaction.
        tx.execute(
            "DELETE FROM concepts
             WHERE topic_id IN (
                 SELECT topic_id FROM topics WHERE document_id = ?1
             )",
            params![document_id],
        )
        .map_err(storage_err)?;
        tx.execute(
            "DELETE FROM topics WHERE document_id = ?1",
            params![document_id],
        )
        .map_err(storage_err)?;

        // Insert new chunks.
        for chunk in new_chunks {
            if chunk.document_id.as_ref() != document_id {
                return Err(CiteError::StorageError {
                    message: format!(
                        "Chunk {} belongs to document {}, expected {}",
                        chunk.chunk_id, chunk.document_id, document_id
                    ),
                });
            }
            tx.execute(
                "INSERT INTO chunks (
                    chunk_id, document_id, section_id, chunk_index,
                    text, page, offset_start, offset_end, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    chunk.chunk_id.as_ref(),
                    chunk.document_id.as_ref(),
                    chunk.section_id,
                    chunk.chunk_index as i64,
                    chunk.text,
                    chunk.page.map(|p| p as i64),
                    chunk.offset_start.map(|o| o as i64),
                    chunk.offset_end.map(|o| o as i64),
                    format_dt(&chunk.created_at),
                ],
            )
            .map_err(storage_err)?;
        }

        // Insert new embeddings.
        for (chunk_id, vector, model_id, provider_id) in new_embeddings {
            let blob: Vec<u8> = vector.iter().flat_map(|f| f.to_le_bytes()).collect();
            tx.execute(
                "INSERT INTO embeddings (chunk_id, vector, model_id, provider_id)
                 VALUES (?1, ?2, ?3, ?4)",
                params![chunk_id, blob, model_id, provider_id],
            )
            .map_err(storage_err)?;
        }

        tx.commit().map_err(storage_err)?;
        Ok(old_chunk_ids)
    }

    /// Delete all chunks belonging to a document. Returns the number deleted.
    pub fn delete_chunks_for_document(&self, document_id: &str) -> Result<u64, CiteError> {
        let count = self
            .conn
            .execute(
                "DELETE FROM chunks WHERE document_id = ?1",
                params![document_id],
            )
            .map_err(storage_err)?;

        Ok(count as u64)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use common::types::{Document, DocumentStatus, FileType};
    use std::path::PathBuf;

    fn insert_parent_doc(db: &Database, id: &str) {
        let doc = Document {
            document_id: id.to_string().into(),
            display_name: format!("{id}.txt"),
            file_path: PathBuf::from(format!("/docs/{id}.txt")),
            file_type: FileType::Txt,
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
    }

    fn make_chunk(id: &str, doc_id: &str, index: u32, text: &str) -> Chunk {
        Chunk {
            chunk_id: id.to_string().into(),
            document_id: doc_id.to_string().into(),
            section_id: None,
            chunk_index: index,
            text: text.to_string(),
            page: None,
            offset_start: None,
            offset_end: None,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_insert_and_query_chunks() {
        let db = Database::open_memory().unwrap();
        insert_parent_doc(&db, "doc-1");

        let chunks = vec![
            make_chunk("c1", "doc-1", 0, "first"),
            make_chunk("c2", "doc-1", 1, "second"),
            make_chunk("c3", "doc-1", 2, "third"),
        ];
        db.insert_chunks("doc-1", &chunks).unwrap();

        // Verify via raw query
        let count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM chunks WHERE document_id = 'doc-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_insert_chunks_with_all_fields() {
        let db = Database::open_memory().unwrap();
        insert_parent_doc(&db, "doc-1");

        let chunk = Chunk {
            chunk_id: "full-c".to_string().into(),
            document_id: "doc-1".to_string().into(),
            section_id: Some("sec-A".to_string()),
            chunk_index: 5,
            text: "hello world".to_string(),
            page: Some(3),
            offset_start: Some(100),
            offset_end: Some(200),
            created_at: Utc::now(),
        };
        db.insert_chunks("doc-1", &[chunk]).unwrap();

        // Read back via raw query
        let conn = db.conn();
        let row = conn
            .query_row(
                "SELECT section_id, page, offset_start, offset_end FROM chunks WHERE chunk_id = 'full-c'",
                [],
                |r| {
                    Ok((
                        r.get::<_, Option<String>>(0)?,
                        r.get::<_, Option<i64>>(1)?,
                        r.get::<_, Option<i64>>(2)?,
                        r.get::<_, Option<i64>>(3)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(row.0.as_deref(), Some("sec-A"));
        assert_eq!(row.1, Some(3));
        assert_eq!(row.2, Some(100));
        assert_eq!(row.3, Some(200));
    }

    #[test]
    fn test_insert_chunks_mismatched_document_id_fails() {
        let db = Database::open_memory().unwrap();
        insert_parent_doc(&db, "doc-1");

        let chunks = vec![make_chunk("c1", "WRONG", 0, "text")];
        let result = db.insert_chunks("doc-1", &chunks);
        assert!(result.is_err());
    }

    #[test]
    fn test_insert_chunks_rolls_back_on_error() {
        let db = Database::open_memory().unwrap();
        insert_parent_doc(&db, "doc-1");

        // First insert succeeds
        db.insert_chunks("doc-1", &[make_chunk("c1", "doc-1", 0, "ok")])
            .unwrap();

        // Second insert has a duplicate chunk_id — should roll back the whole batch
        let batch = vec![
            make_chunk("c2", "doc-1", 1, "fine"),
            make_chunk("c1", "doc-1", 2, "dup"), // duplicate PK
        ];
        let result = db.insert_chunks("doc-1", &batch);
        assert!(result.is_err());

        // Only the first chunk should exist; c2 should NOT have been committed
        let count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM chunks WHERE document_id = 'doc-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_delete_chunks_for_document() {
        let db = Database::open_memory().unwrap();
        insert_parent_doc(&db, "doc-1");

        let chunks = vec![
            make_chunk("c1", "doc-1", 0, "a"),
            make_chunk("c2", "doc-1", 1, "b"),
        ];
        db.insert_chunks("doc-1", &chunks).unwrap();

        let deleted = db.delete_chunks_for_document("doc-1").unwrap();
        assert_eq!(deleted, 2);

        let count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM chunks WHERE document_id = 'doc-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_delete_chunks_returns_zero_for_unknown_doc() {
        let db = Database::open_memory().unwrap();
        let deleted = db.delete_chunks_for_document("ghost").unwrap();
        assert_eq!(deleted, 0);
    }

    // -----------------------------------------------------------------------
    // set_chunk_hierarchy direct
    // -----------------------------------------------------------------------

    #[test]
    fn test_set_chunk_hierarchy_sets_topic_and_concept() {
        let db = Database::open_memory().unwrap();
        insert_parent_doc(&db, "doc-hier");

        let chunks = vec![make_chunk("ch-0", "doc-hier", 0, "text")];
        db.insert_chunks("doc-hier", &chunks).unwrap();

        // Insert topic and concept
        db.insert_topic("t1", "doc-hier", "Topic A", None).unwrap();
        db.insert_concept("c1", "t1", "Concept X", None).unwrap();

        // Set hierarchy on the chunk
        db.set_chunk_hierarchy("ch-0", "t1", Some("c1")).unwrap();

        // Verify via raw query
        let (topic_id, concept_id): (Option<String>, Option<String>) = db
            .conn()
            .query_row(
                "SELECT topic_id, concept_id FROM chunks WHERE chunk_id = 'ch-0'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(topic_id.as_deref(), Some("t1"));
        assert_eq!(concept_id.as_deref(), Some("c1"));
    }

    #[test]
    fn test_set_chunk_hierarchy_topic_only() {
        let db = Database::open_memory().unwrap();
        insert_parent_doc(&db, "doc-hier");

        let chunks = vec![make_chunk("ch-0", "doc-hier", 0, "text")];
        db.insert_chunks("doc-hier", &chunks).unwrap();

        db.insert_topic("t1", "doc-hier", "Topic A", None).unwrap();

        // Set hierarchy with concept = None
        db.set_chunk_hierarchy("ch-0", "t1", None).unwrap();

        let (topic_id, concept_id): (Option<String>, Option<String>) = db
            .conn()
            .query_row(
                "SELECT topic_id, concept_id FROM chunks WHERE chunk_id = 'ch-0'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(topic_id.as_deref(), Some("t1"));
        assert_eq!(concept_id, None);
    }

    #[test]
    fn test_get_chunks_for_document_returns_ordered_chunks() {
        let db = Database::open_memory().unwrap();
        insert_parent_doc(&db, "doc-g");

        let chunks = vec![
            make_chunk("gc-2", "doc-g", 1, "second"),
            make_chunk("gc-1", "doc-g", 0, "first"),
            make_chunk("gc-3", "doc-g", 2, "third"),
        ];
        db.insert_chunks("doc-g", &chunks).unwrap();

        let result = db.get_chunks_for_document("doc-g").unwrap();
        assert_eq!(result.len(), 3);
        // Should be ordered by chunk_index ASC.
        assert_eq!(result[0].chunk_id.as_ref(), "gc-1");
        assert_eq!(result[1].chunk_id.as_ref(), "gc-2");
        assert_eq!(result[2].chunk_id.as_ref(), "gc-3");
        assert_eq!(result[0].text, "first");
        assert_eq!(result[1].text, "second");
        assert_eq!(result[2].text, "third");
    }

    // -----------------------------------------------------------------------
    // replace_chunks_for_document
    // -----------------------------------------------------------------------

    #[test]
    fn test_replace_chunks_for_document_replaces_atomically() {
        let db = Database::open_memory().unwrap();
        insert_parent_doc(&db, "doc-r");
        insert_parent_doc(&db, "doc-other");

        // Insert original chunks.
        let old_chunks = vec![
            make_chunk("old-1", "doc-r", 0, "first old"),
            make_chunk("old-2", "doc-r", 1, "second old"),
        ];
        db.insert_chunks("doc-r", &old_chunks).unwrap();
        db.insert_chunks(
            "doc-other",
            &[make_chunk("other-1", "doc-other", 0, "external")],
        )
        .unwrap();

        // Add tag, hierarchy, and semantic links for old chunks.
        db.set_tag_engine(
            crate::tags::TagEntityType::Chunk,
            "old-1",
            &crate::tags::TagRecord::new("status", "changed").unwrap(),
        )
        .unwrap();
        db.insert_topic("topic-doc-r", "doc-r", "Old Topic", None)
            .unwrap();
        db.insert_concept("concept-doc-r", "topic-doc-r", "Old Concept", None)
            .unwrap();
        db.set_chunk_hierarchy("old-1", "topic-doc-r", Some("concept-doc-r"))
            .unwrap();
        db.insert_semantic_link("link-from-old", "old-1", "other-1", 0.9, "semantic")
            .unwrap();
        db.insert_semantic_link("link-to-old", "other-1", "old-2", 0.8, "semantic")
            .unwrap();

        // New chunks.
        let new_chunks = vec![
            make_chunk("new-1", "doc-r", 0, "first new"),
            make_chunk("new-2", "doc-r", 1, "second new"),
            make_chunk("new-3", "doc-r", 2, "third new"),
        ];
        let new_embeddings: Vec<(String, Vec<f32>, &str, &str)> = new_chunks
            .iter()
            .map(|c| (c.chunk_id.to_string(), vec![0.1, 0.2], "m", "p"))
            .collect();

        let old_ids = db
            .replace_chunks_for_document("doc-r", &new_chunks, &new_embeddings)
            .unwrap();

        // Old chunk IDs returned.
        assert_eq!(old_ids.len(), 2);
        assert!(old_ids.contains(&"old-1".to_string()));
        assert!(old_ids.contains(&"old-2".to_string()));

        // New chunks exist.
        let count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM chunks WHERE document_id = 'doc-r'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 3);

        // Old chunk tags and hierarchy/links are gone.
        let old_tags = db
            .list_tags(crate::tags::TagEntityType::Chunk, "old-1")
            .unwrap();
        assert!(old_tags.is_empty());
        let topic_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM topics WHERE document_id = 'doc-r'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(topic_count, 0);
        let link_count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM semantic_links", [], |row| row.get(0))
            .unwrap();
        assert_eq!(link_count, 0);

        // New embeddings exist.
        let emb_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM embeddings WHERE chunk_id IN ('new-1', 'new-2', 'new-3')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(emb_count, 3);
    }

    #[test]
    fn test_replace_chunks_for_document_rolls_back_on_insert_error() {
        let db = Database::open_memory().unwrap();
        insert_parent_doc(&db, "doc-rb");
        insert_parent_doc(&db, "doc-other-rb");

        let old_chunks = vec![
            make_chunk("old-rb-1", "doc-rb", 0, "first old"),
            make_chunk("old-rb-2", "doc-rb", 1, "second old"),
        ];
        db.insert_chunks("doc-rb", &old_chunks).unwrap();
        db.insert_chunks(
            "doc-other-rb",
            &[make_chunk("other-rb-1", "doc-other-rb", 0, "external")],
        )
        .unwrap();
        db.set_tag_engine(
            crate::tags::TagEntityType::Chunk,
            "old-rb-1",
            &crate::tags::TagRecord::new("status", "changed").unwrap(),
        )
        .unwrap();
        db.insert_topic("topic-doc-rb", "doc-rb", "Old Topic", None)
            .unwrap();
        db.insert_concept("concept-doc-rb", "topic-doc-rb", "Old Concept", None)
            .unwrap();
        db.set_chunk_hierarchy("old-rb-1", "topic-doc-rb", Some("concept-doc-rb"))
            .unwrap();
        db.insert_semantic_link("link-rb", "old-rb-1", "other-rb-1", 0.9, "semantic")
            .unwrap();

        let invalid_new_chunks = vec![make_chunk("new-rb-1", "wrong-doc", 0, "new")];
        let result = db.replace_chunks_for_document("doc-rb", &invalid_new_chunks, &[]);
        assert!(result.is_err());

        let chunk_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM chunks WHERE document_id = 'doc-rb'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(chunk_count, 2);
        assert_eq!(
            db.list_tags(crate::tags::TagEntityType::Chunk, "old-rb-1")
                .unwrap(),
            vec![crate::tags::TagRecord::new("status", "changed").unwrap()]
        );
        let topic_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM topics WHERE document_id = 'doc-rb'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(topic_count, 1);
        assert_eq!(db.get_links_from("old-rb-1").unwrap().len(), 1);
    }
}
