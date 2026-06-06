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
}
