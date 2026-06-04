use common::CiteError;
use rusqlite::params;

use crate::util::storage_err;
use crate::Database;

#[derive(Debug, Clone)]
pub struct ChunkEmbeddingRecord {
    pub chunk_id: String,
    pub document_id: String,
    pub display_name: String,
    pub section_id: Option<String>,
    pub chunk_index: u32,
    pub text: String,
    pub page: Option<u32>,
    pub offset_start: Option<u32>,
    pub offset_end: Option<u32>,
    pub vector: Vec<f32>,
}

/// Chunk embedding enriched with hierarchy metadata (topic/concept).
#[derive(Debug, Clone)]
pub struct HierarchicalChunkEmbedding {
    pub chunk: ChunkEmbeddingRecord,
    pub topic_id: Option<String>,
    pub topic_name: Option<String>,
    pub concept_id: Option<String>,
    pub concept_name: Option<String>,
}

fn decode_vector_blob(blob: &[u8]) -> Result<Vec<f32>, CiteError> {
    if !blob.len().is_multiple_of(4) {
        return Err(CiteError::StorageError {
            message: format!(
                "Corrupt vector blob: length {} is not a multiple of 4",
                blob.len()
            ),
        });
    }

    Ok(blob
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect())
}

/// Map a database row (columns 0..=9) into a [`ChunkEmbeddingRecord`].
///
/// Expected column order:
///   0 chunk_id, 1 document_id, 2 display_name, 3 section_id,
///   4 chunk_index (i64), 5 text, 6 page (Option<i64>),
///   7 offset_start (Option<i64>), 8 offset_end (Option<i64>),
///   9 vector (blob).
fn row_to_chunk_embedding(row: &rusqlite::Row<'_>) -> Result<ChunkEmbeddingRecord, CiteError> {
    let blob: Vec<u8> = row.get(9).map_err(storage_err)?;
    let vector = decode_vector_blob(&blob)?;

    Ok(ChunkEmbeddingRecord {
        chunk_id: row.get(0).map_err(storage_err)?,
        document_id: row.get(1).map_err(storage_err)?,
        display_name: row.get(2).map_err(storage_err)?,
        section_id: row.get(3).map_err(storage_err)?,
        chunk_index: u32::try_from(row.get::<_, i64>(4).map_err(storage_err)?)
            .map_err(|e| storage_err(format!("chunk_index overflow: {e}")))?,
        text: row.get(5).map_err(storage_err)?,
        page: row
            .get::<_, Option<i64>>(6)
            .map_err(storage_err)?
            .map(u32::try_from)
            .transpose()
            .map_err(|e| storage_err(format!("page overflow: {e}")))?,
        offset_start: row
            .get::<_, Option<i64>>(7)
            .map_err(storage_err)?
            .map(u32::try_from)
            .transpose()
            .map_err(|e| storage_err(format!("offset_start overflow: {e}")))?,
        offset_end: row
            .get::<_, Option<i64>>(8)
            .map_err(storage_err)?
            .map(u32::try_from)
            .transpose()
            .map_err(|e| storage_err(format!("offset_end overflow: {e}")))?,
        vector,
    })
}

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
    ) -> Result<(), CiteError> {
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
    pub fn delete_embeddings_for_document(&self, document_id: &str) -> Result<u64, CiteError> {
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

    /// Check whether any chunk in the database has hierarchy data (non-NULL topic_id).
    pub fn has_hierarchy_data(&self) -> Result<bool, CiteError> {
        let has: bool = self
            .conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM chunks WHERE topic_id IS NOT NULL LIMIT 1)",
                [],
                |row| row.get(0),
            )
            .map_err(storage_err)?;
        Ok(has)
    }

    /// List chunk embeddings enriched with topic/concept hierarchy metadata.
    ///
    /// When `topic_filter` or `concept_filter` is provided, results are
    /// restricted to chunks belonging to that topic or concept respectively.
    /// Chunks with NULL topic_id/concept_id are included when no filter is
    /// specified, so the caller still gets flat-only documents in mixed corpora.
    pub fn list_chunk_embeddings_hierarchical(
        &self,
        topic_filter: Option<&str>,
        concept_filter: Option<&str>,
    ) -> Result<Vec<HierarchicalChunkEmbedding>, CiteError> {
        let sql = "
            SELECT
                c.chunk_id,
                c.document_id,
                d.display_name,
                c.section_id,
                c.chunk_index,
                c.text,
                c.page,
                c.offset_start,
                c.offset_end,
                e.vector,
                c.topic_id,
                t.name,
                c.concept_id,
                cp.name
            FROM embeddings e
            INNER JOIN chunks c ON c.chunk_id = e.chunk_id
            INNER JOIN documents d ON d.document_id = c.document_id
            LEFT JOIN topics t ON t.topic_id = c.topic_id
            LEFT JOIN concepts cp ON cp.concept_id = c.concept_id
            WHERE d.status = 'ready'
              AND (?1 IS NULL OR c.topic_id = ?1)
              AND (?2 IS NULL OR c.concept_id = ?2)
            ORDER BY d.created_at DESC, c.chunk_index ASC
        ";

        let mut stmt = self.conn.prepare(sql).map_err(storage_err)?;
        let mut rows = stmt
            .query(rusqlite::params![topic_filter, concept_filter])
            .map_err(storage_err)?;
        let mut out = Vec::new();

        while let Some(row) = rows.next().map_err(storage_err)? {
            out.push(HierarchicalChunkEmbedding {
                chunk: row_to_chunk_embedding(row)?,
                topic_id: row.get(10).map_err(storage_err)?,
                topic_name: row.get(11).map_err(storage_err)?,
                concept_id: row.get(12).map_err(storage_err)?,
                concept_name: row.get(13).map_err(storage_err)?,
            });
        }

        Ok(out)
    }

    /// List chunk embeddings for documents with status='ready'.
    pub fn list_ready_chunk_embeddings(&self) -> Result<Vec<ChunkEmbeddingRecord>, CiteError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT
                    c.chunk_id,
                    c.document_id,
                    d.display_name,
                    c.section_id,
                    c.chunk_index,
                    c.text,
                    c.page,
                    c.offset_start,
                    c.offset_end,
                    e.vector
                 FROM embeddings e
                 INNER JOIN chunks c ON c.chunk_id = e.chunk_id
                 INNER JOIN documents d ON d.document_id = c.document_id
                 WHERE d.status = 'ready'
                 ORDER BY d.created_at DESC, c.chunk_index ASC",
            )
            .map_err(storage_err)?;

        let mut rows = stmt.query([]).map_err(storage_err)?;
        let mut out = Vec::new();

        while let Some(row) = rows.next().map_err(storage_err)? {
            out.push(row_to_chunk_embedding(row)?);
        }

        Ok(out)
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
        let result = db.insert_embeddings(&[(
            "doc-1-c0".to_string(),
            vec![9.0],
            "m2",
            "p2", // dup PK
        )]);
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

    #[test]
    fn test_list_ready_chunk_embeddings_filters_by_document_status() {
        let db = Database::open_memory().unwrap();
        insert_parent_doc(&db, "doc-ready");
        insert_parent_doc(&db, "doc-failed");

        db.update_document_status("doc-ready", DocumentStatus::Ready, None)
            .unwrap();
        db.update_document_status("doc-failed", DocumentStatus::Failed, None)
            .unwrap();

        insert_chunks_for_doc(&db, "doc-ready", 1);
        insert_chunks_for_doc(&db, "doc-failed", 1);

        db.insert_embeddings(&[
            ("doc-ready-c0".to_string(), vec![0.1, 0.2], "m", "p"),
            ("doc-failed-c0".to_string(), vec![0.9, 1.0], "m", "p"),
        ])
        .unwrap();

        let rows = db.list_ready_chunk_embeddings().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].document_id, "doc-ready");
        assert_eq!(rows[0].chunk_id, "doc-ready-c0");
    }

    #[test]
    fn test_list_ready_chunk_embeddings_decodes_vector() {
        let db = Database::open_memory().unwrap();
        insert_parent_doc(&db, "doc-ready");
        db.update_document_status("doc-ready", DocumentStatus::Ready, None)
            .unwrap();
        insert_chunks_for_doc(&db, "doc-ready", 1);

        db.insert_embeddings(&[("doc-ready-c0".to_string(), vec![1.5, 2.5, 3.5], "m", "p")])
            .unwrap();

        let rows = db.list_ready_chunk_embeddings().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].vector, vec![1.5, 2.5, 3.5]);
        assert_eq!(rows[0].display_name, "doc-ready.txt");
    }

    #[test]
    fn test_corrupt_vector_blob_returns_error() {
        let db = Database::open_memory().unwrap();
        insert_parent_doc(&db, "doc-corrupt");
        db.update_document_status("doc-corrupt", DocumentStatus::Ready, None)
            .unwrap();
        insert_chunks_for_doc(&db, "doc-corrupt", 2);

        // Insert one valid embedding and one with a corrupt (odd-length) blob
        db.insert_embeddings(&[("doc-corrupt-c0".to_string(), vec![1.0, 2.0], "m", "p")])
            .unwrap();

        // Manually insert a corrupt 3-byte blob (not a multiple of 4)
        let conn = db.conn();
        conn.execute(
            "INSERT INTO embeddings (chunk_id, vector, model_id, provider_id) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["doc-corrupt-c1", vec![0u8, 1u8, 2u8], "m", "p"],
        )
        .unwrap();

        // The call should now return an error instead of silently skipping
        let result = db.list_ready_chunk_embeddings();
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("Corrupt vector blob"),
            "expected corrupt blob error, got: {msg}"
        );
    }

    // -----------------------------------------------------------------------
    // Hierarchical query tests
    // -----------------------------------------------------------------------

    fn setup_hierarchy(db: &Database) {
        // doc with hierarchy
        insert_parent_doc(db, "doc-hier");
        db.update_document_status("doc-hier", DocumentStatus::Ready, None)
            .unwrap();
        insert_chunks_for_doc(db, "doc-hier", 3);

        // insert topic and concept
        db.insert_topic("t1", "doc-hier", "Authentication", None)
            .unwrap();
        db.insert_concept("c1", "t1", "JWT Tokens", None).unwrap();

        // assign chunks: c0 and c1 to topic+concept, c2 to topic only
        db.set_chunk_hierarchy("doc-hier-c0", "t1", Some("c1"))
            .unwrap();
        db.set_chunk_hierarchy("doc-hier-c1", "t1", Some("c1"))
            .unwrap();
        db.set_chunk_hierarchy("doc-hier-c2", "t1", None).unwrap();

        // insert embeddings
        db.insert_embeddings(&[
            ("doc-hier-c0".to_string(), vec![1.0, 0.0], "m", "p"),
            ("doc-hier-c1".to_string(), vec![0.9, 0.1], "m", "p"),
            ("doc-hier-c2".to_string(), vec![0.0, 1.0], "m", "p"),
        ])
        .unwrap();
    }

    #[test]
    fn test_has_hierarchy_data_returns_false_when_none() {
        let db = Database::open_memory().unwrap();
        insert_parent_doc(&db, "doc-flat");
        db.update_document_status("doc-flat", DocumentStatus::Ready, None)
            .unwrap();
        insert_chunks_for_doc(&db, "doc-flat", 1);
        db.insert_embeddings(&[("doc-flat-c0".to_string(), vec![1.0], "m", "p")])
            .unwrap();

        assert!(!db.has_hierarchy_data().unwrap());
    }

    #[test]
    fn test_has_hierarchy_data_returns_true_when_present() {
        let db = Database::open_memory().unwrap();
        setup_hierarchy(&db);

        assert!(db.has_hierarchy_data().unwrap());
    }

    #[test]
    fn test_list_chunk_embeddings_hierarchical_no_filter() {
        let db = Database::open_memory().unwrap();
        setup_hierarchy(&db);

        let rows = db.list_chunk_embeddings_hierarchical(None, None).unwrap();
        assert_eq!(rows.len(), 3);

        // rows are ordered by chunk_index ASC (same doc)
        let r0 = &rows[0];
        assert_eq!(r0.chunk.chunk_id, "doc-hier-c0");
        assert_eq!(r0.topic_id.as_deref(), Some("t1"));
        assert_eq!(r0.topic_name.as_deref(), Some("Authentication"));
        assert_eq!(r0.concept_id.as_deref(), Some("c1"));
        assert_eq!(r0.concept_name.as_deref(), Some("JWT Tokens"));

        let r2 = &rows[2];
        assert_eq!(r2.chunk.chunk_id, "doc-hier-c2");
        assert_eq!(r2.topic_id.as_deref(), Some("t1"));
        assert_eq!(r2.concept_id, None);
        assert_eq!(r2.concept_name, None);
    }

    #[test]
    fn test_list_chunk_embeddings_hierarchical_topic_filter() {
        let db = Database::open_memory().unwrap();
        setup_hierarchy(&db);

        let rows = db
            .list_chunk_embeddings_hierarchical(Some("t1"), None)
            .unwrap();
        assert_eq!(rows.len(), 3); // all 3 belong to t1
    }

    #[test]
    fn test_list_chunk_embeddings_hierarchical_concept_filter() {
        let db = Database::open_memory().unwrap();
        setup_hierarchy(&db);

        let rows = db
            .list_chunk_embeddings_hierarchical(None, Some("c1"))
            .unwrap();
        assert_eq!(rows.len(), 2); // only c0 and c1 belong to concept c1
        assert!(rows.iter().all(|r| r.concept_id.as_deref() == Some("c1")));
    }

    #[test]
    fn test_list_chunk_embeddings_hierarchical_no_match() {
        let db = Database::open_memory().unwrap();
        setup_hierarchy(&db);

        let rows = db
            .list_chunk_embeddings_hierarchical(Some("nonexistent"), None)
            .unwrap();
        assert!(rows.is_empty());
    }
}
