use common::CiteError;
use rusqlite::params;

use crate::util::storage_err;
use crate::Database;

// ---------------------------------------------------------------------------
// Row type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SemanticLinkRow {
    pub link_id: String,
    pub source_chunk_id: String,
    pub target_chunk_id: String,
    pub similarity_score: f64,
    pub link_type: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// CRUD operations
// ---------------------------------------------------------------------------

impl Database {
    /// Insert a new semantic link.
    pub fn insert_semantic_link(
        &self,
        link_id: &str,
        source_chunk_id: &str,
        target_chunk_id: &str,
        similarity_score: f64,
        link_type: &str,
    ) -> Result<(), CiteError> {
        self.conn
            .execute(
                "INSERT INTO semantic_links (link_id, source_chunk_id, target_chunk_id, similarity_score, link_type)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![link_id, source_chunk_id, target_chunk_id, similarity_score, link_type],
            )
            .map_err(storage_err)?;
        Ok(())
    }

    /// Get all links where the given chunk is the source.
    pub fn get_links_from(&self, chunk_id: &str) -> Result<Vec<SemanticLinkRow>, CiteError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT link_id, source_chunk_id, target_chunk_id, similarity_score, link_type, created_at
                 FROM semantic_links WHERE source_chunk_id = ?1 ORDER BY similarity_score DESC",
            )
            .map_err(storage_err)?;

        let mut rows = stmt.query(params![chunk_id]).map_err(storage_err)?;
        let mut result = Vec::new();

        while let Some(row) = rows.next().map_err(storage_err)? {
            result.push(SemanticLinkRow {
                link_id: row.get(0).map_err(storage_err)?,
                source_chunk_id: row.get(1).map_err(storage_err)?,
                target_chunk_id: row.get(2).map_err(storage_err)?,
                similarity_score: row.get(3).map_err(storage_err)?,
                link_type: row.get(4).map_err(storage_err)?,
                created_at: row.get(5).map_err(storage_err)?,
            });
        }
        Ok(result)
    }

    /// Get all links where the given chunk is the target.
    pub fn get_links_to(&self, chunk_id: &str) -> Result<Vec<SemanticLinkRow>, CiteError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT link_id, source_chunk_id, target_chunk_id, similarity_score, link_type, created_at
                 FROM semantic_links WHERE target_chunk_id = ?1 ORDER BY similarity_score DESC",
            )
            .map_err(storage_err)?;

        let mut rows = stmt.query(params![chunk_id]).map_err(storage_err)?;
        let mut result = Vec::new();

        while let Some(row) = rows.next().map_err(storage_err)? {
            result.push(SemanticLinkRow {
                link_id: row.get(0).map_err(storage_err)?,
                source_chunk_id: row.get(1).map_err(storage_err)?,
                target_chunk_id: row.get(2).map_err(storage_err)?,
                similarity_score: row.get(3).map_err(storage_err)?,
                link_type: row.get(4).map_err(storage_err)?,
                created_at: row.get(5).map_err(storage_err)?,
            });
        }
        Ok(result)
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

    fn setup_two_docs_with_chunks(db: &Database) {
        for doc_id in &["doc-1", "doc-2"] {
            let doc = Document {
                document_id: (*doc_id).into(),
                display_name: format!("{doc_id}.txt"),
                file_path: PathBuf::from(format!("/docs/{doc_id}.txt")),
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

        let chunks = [
            Chunk {
                chunk_id: "c-a".into(),
                document_id: "doc-1".into(),
                section_id: None,
                chunk_index: 0,
                text: "chunk a".to_string(),
                page: None,
                offset_start: None,
                offset_end: None,
                created_at: Utc::now(),
            },
            Chunk {
                chunk_id: "c-b".into(),
                document_id: "doc-2".into(),
                section_id: None,
                chunk_index: 0,
                text: "chunk b".to_string(),
                page: None,
                offset_start: None,
                offset_end: None,
                created_at: Utc::now(),
            },
            Chunk {
                chunk_id: "c-c".into(),
                document_id: "doc-2".into(),
                section_id: None,
                chunk_index: 1,
                text: "chunk c".to_string(),
                page: None,
                offset_start: None,
                offset_end: None,
                created_at: Utc::now(),
            },
        ];
        db.insert_chunks("doc-1", &[chunks[0].clone()]).unwrap();
        db.insert_chunks("doc-2", &[chunks[1].clone(), chunks[2].clone()])
            .unwrap();
    }

    #[test]
    fn test_insert_and_get_links_from() {
        let db = Database::open_memory().unwrap();
        setup_two_docs_with_chunks(&db);

        db.insert_semantic_link("l1", "c-a", "c-b", 0.85, "semantic")
            .unwrap();
        db.insert_semantic_link("l2", "c-a", "c-c", 0.72, "semantic")
            .unwrap();

        let links = db.get_links_from("c-a").unwrap();
        assert_eq!(links.len(), 2);
        // Should be ordered by similarity_score DESC
        assert_eq!(links[0].link_id, "l1");
        assert_eq!(links[0].similarity_score, 0.85);
        assert_eq!(links[1].link_id, "l2");
        assert_eq!(links[1].similarity_score, 0.72);
    }

    #[test]
    fn test_get_links_from_empty() {
        let db = Database::open_memory().unwrap();
        setup_two_docs_with_chunks(&db);

        let links = db.get_links_from("c-a").unwrap();
        assert!(links.is_empty());
    }

    #[test]
    fn test_get_links_to() {
        let db = Database::open_memory().unwrap();
        setup_two_docs_with_chunks(&db);

        db.insert_semantic_link("l1", "c-a", "c-b", 0.9, "semantic")
            .unwrap();
        db.insert_semantic_link("l2", "c-c", "c-b", 0.6, "citation")
            .unwrap();

        let links = db.get_links_to("c-b").unwrap();
        assert_eq!(links.len(), 2);
        // Ordered by similarity_score DESC
        assert_eq!(links[0].link_id, "l1");
        assert_eq!(links[0].link_type, "semantic");
        assert_eq!(links[1].link_id, "l2");
        assert_eq!(links[1].link_type, "citation");
    }

    #[test]
    fn test_get_links_to_empty() {
        let db = Database::open_memory().unwrap();
        setup_two_docs_with_chunks(&db);

        let links = db.get_links_to("c-b").unwrap();
        assert!(links.is_empty());
    }

    #[test]
    fn test_insert_duplicate_link_fails() {
        let db = Database::open_memory().unwrap();
        setup_two_docs_with_chunks(&db);

        db.insert_semantic_link("l1", "c-a", "c-b", 0.8, "semantic")
            .unwrap();
        // Same (source, target) pair violates UNIQUE constraint
        assert!(db
            .insert_semantic_link("l2", "c-a", "c-b", 0.9, "semantic")
            .is_err());
    }

    #[test]
    fn test_insert_link_with_custom_type() {
        let db = Database::open_memory().unwrap();
        setup_two_docs_with_chunks(&db);

        db.insert_semantic_link("l1", "c-a", "c-b", 0.75, "citation")
            .unwrap();

        let links = db.get_links_from("c-a").unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].link_type, "citation");
    }
}
