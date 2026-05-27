use common::HarnessError;
use rusqlite::params;

use crate::util::storage_err;
use crate::Database;

// ---------------------------------------------------------------------------
// CRUD operations
// ---------------------------------------------------------------------------

impl Database {
    /// Bulk-insert embeddings inside a single transaction.
    ///
    /// Each tuple is `(chunk_id, vector, model_id, provider_id)`.
    /// The vector is stored as a BLOB of little-endian f32 values.
    pub fn insert_embeddings(
        &self,
        embeddings: &[(String, Vec<f32>, &str, &str)],
    ) -> Result<(), HarnessError> {
        let tx = self.conn.unchecked_transaction().map_err(storage_err)?;

        for (chunk_id, vector, model_id, provider_id) in embeddings {
            let blob: Vec<u8> = vector.iter().flat_map(|f| f.to_le_bytes()).collect();

            tx.execute(
                "INSERT INTO embeddings (chunk_id, vector, model_id, provider_id)
                 VALUES (?1, ?2, ?3, ?4)",
                params![chunk_id, blob, model_id, provider_id],
            )
            .map_err(storage_err)?;
        }

        tx.commit().map_err(storage_err)?;
        Ok(())
    }

    /// Delete all embeddings whose chunk belongs to the given document.
    /// Returns the number of rows deleted.
    pub fn delete_embeddings_for_document(&self, document_id: &str) -> Result<u64, HarnessError> {
        let count = self
            .conn
            .execute(
                "DELETE FROM embeddings WHERE chunk_id IN (
                    SELECT chunk_id FROM chunks WHERE document_id = ?1
                 )",
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
    use common::types::{Chunk, Document, DocumentStatus, FileType};
    use std::path::PathBuf;

    fn insert_parent_doc(db: &Database, id: &str) {
        let doc = Document {
            document_id: id.to_string(),
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

    fn insert_chunks_for_doc(db: &Database, doc_id: &str, count: u32) {
        let chunks: Vec<Chunk> = (0..count)
            .map(|i| Chunk {
                chunk_id: format!("{doc_id}-c{i}"),
                document_id: doc_id.to_string(),
                section_id: None,
                chunk_index: i,
                text: format!("chunk {i}"),
                page: None,
                offset_start: None,
                offset_end: None,
                created_at: Utc::now(),
            })
            .collect();
        db.insert_chunks(doc_id, &chunks).unwrap();
    }

    #[test]
    fn test_insert_embeddings() {
        let db = Database::open_memory().unwrap();
        insert_parent_doc(&db, "doc-1");
        insert_chunks_for_doc(&db, "doc-1", 2);

        let vec1: Vec<f32> = vec![1.0, 2.0, 3.0];
        let vec2: Vec<f32> = vec![4.0, 5.0, 6.0];
        let embeddings = vec![
            ("doc-1-c0".to_string(), vec1, "text-ada-002", "openai"),
            ("doc-1-c1".to_string(), vec2, "text-ada-002", "openai"),
        ];
        db.insert_embeddings(&embeddings).unwrap();

        // Verify stored BLOBs
        let conn = db.conn();
        let blob: Vec<u8> = conn
            .query_row(
                "SELECT vector FROM embeddings WHERE chunk_id = 'doc-1-c0'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        // Reconstruct f32 values
        let floats: Vec<f32> = blob
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        assert_eq!(floats, vec![1.0, 2.0, 3.0]);

        // Verify model/provider
        let (model, provider): (String, String) = conn
            .query_row(
                "SELECT model_id, provider_id FROM embeddings WHERE chunk_id = 'doc-1-c0'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(model, "text-ada-002");
        assert_eq!(provider, "openai");
    }

    #[test]
    fn test_insert_embeddings_rolls_back_on_failure() {
        let db = Database::open_memory().unwrap();
        insert_parent_doc(&db, "doc-1");
        insert_chunks_for_doc(&db, "doc-1", 1);

        // First insert
        db.insert_embeddings(&[("doc-1-c0".to_string(), vec![1.0], "m", "p")])
            .unwrap();

        // Duplicate chunk_id should fail and roll back
        let result = db.insert_embeddings(&[
            ("doc-1-c0".to_string(), vec![9.0], "m2", "p2"), // dup PK
        ]);
        assert!(result.is_err());

        // Original embedding should still be there
        let conn = db.conn();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM embeddings", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_delete_embeddings_for_document() {
        let db = Database::open_memory().unwrap();
        insert_parent_doc(&db, "doc-1");
        insert_parent_doc(&db, "doc-2");
        insert_chunks_for_doc(&db, "doc-1", 2);
        insert_chunks_for_doc(&db, "doc-2", 1);

        let embeddings = vec![
            ("doc-1-c0".to_string(), vec![1.0], "m", "p"),
            ("doc-1-c1".to_string(), vec![2.0], "m", "p"),
            ("doc-2-c0".to_string(), vec![3.0], "m", "p"),
        ];
        db.insert_embeddings(&embeddings).unwrap();

        let deleted = db.delete_embeddings_for_document("doc-1").unwrap();
        assert_eq!(deleted, 2);

        // doc-2's embedding should remain
        let conn = db.conn();
        let remaining: i64 = conn
            .query_row("SELECT COUNT(*) FROM embeddings", [], |r| r.get(0))
            .unwrap();
        assert_eq!(remaining, 1);
    }

    #[test]
    fn test_delete_embeddings_returns_zero_for_unknown_doc() {
        let db = Database::open_memory().unwrap();
        let deleted = db.delete_embeddings_for_document("ghost").unwrap();
        assert_eq!(deleted, 0);
    }

    #[test]
    fn test_empty_vector_stored_correctly() {
        let db = Database::open_memory().unwrap();
        insert_parent_doc(&db, "doc-1");
        insert_chunks_for_doc(&db, "doc-1", 1);

        db.insert_embeddings(&[("doc-1-c0".to_string(), vec![], "m", "p")])
            .unwrap();

        let conn = db.conn();
        let blob: Vec<u8> = conn
            .query_row(
                "SELECT vector FROM embeddings WHERE chunk_id = 'doc-1-c0'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(blob.is_empty());
    }
}
