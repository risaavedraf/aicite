use common::CiteError;
use rusqlite::params;

use crate::util::storage_err;
use crate::Database;

// ---------------------------------------------------------------------------
// Row type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ConceptRow {
    pub concept_id: String,
    pub topic_id: String,
    pub name: String,
    pub summary: Option<String>,
    pub chunk_count: i64,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// CRUD operations
// ---------------------------------------------------------------------------

impl Database {
    /// Insert a new concept.
    pub fn insert_concept(
        &self,
        concept_id: &str,
        topic_id: &str,
        name: &str,
        summary: Option<&str>,
    ) -> Result<(), CiteError> {
        self.conn
            .execute(
                "INSERT INTO concepts (concept_id, topic_id, name, summary) VALUES (?1, ?2, ?3, ?4)",
                params![concept_id, topic_id, name, summary],
            )
            .map_err(storage_err)?;
        Ok(())
    }

    /// Get a concept by ID. Returns `None` when not found.
    pub fn get_concept(&self, concept_id: &str) -> Result<Option<ConceptRow>, CiteError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT concept_id, topic_id, name, summary, chunk_count, created_at
                 FROM concepts WHERE concept_id = ?1",
            )
            .map_err(storage_err)?;

        let mut rows = stmt.query(params![concept_id]).map_err(storage_err)?;

        match rows.next().map_err(storage_err)? {
            Some(row) => Ok(Some(ConceptRow {
                concept_id: row.get(0).map_err(storage_err)?,
                topic_id: row.get(1).map_err(storage_err)?,
                name: row.get(2).map_err(storage_err)?,
                summary: row.get(3).map_err(storage_err)?,
                chunk_count: row.get(4).map_err(storage_err)?,
                created_at: row.get(5).map_err(storage_err)?,
            })),
            None => Ok(None),
        }
    }

    /// List all concepts for a topic, ordered by creation time.
    pub fn list_concepts_by_topic(&self, topic_id: &str) -> Result<Vec<ConceptRow>, CiteError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT concept_id, topic_id, name, summary, chunk_count, created_at
                 FROM concepts WHERE topic_id = ?1 ORDER BY created_at",
            )
            .map_err(storage_err)?;

        let mut rows = stmt.query(params![topic_id]).map_err(storage_err)?;
        let mut result = Vec::new();

        while let Some(row) = rows.next().map_err(storage_err)? {
            result.push(ConceptRow {
                concept_id: row.get(0).map_err(storage_err)?,
                topic_id: row.get(1).map_err(storage_err)?,
                name: row.get(2).map_err(storage_err)?,
                summary: row.get(3).map_err(storage_err)?,
                chunk_count: row.get(4).map_err(storage_err)?,
                created_at: row.get(5).map_err(storage_err)?,
            });
        }
        Ok(result)
    }

    /// Recalculate chunk_count for a concept from the chunks table.
    pub fn update_concept_chunk_count(&self, concept_id: &str) -> Result<(), CiteError> {
        self.conn
            .execute(
                "UPDATE concepts SET chunk_count = (
                    SELECT COUNT(*) FROM chunks WHERE concept_id = ?1
                 ) WHERE concept_id = ?1",
                params![concept_id],
            )
            .map_err(storage_err)?;
        Ok(())
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

    fn insert_doc(db: &Database, id: &str) {
        let doc = Document {
            document_id: id.into(),
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

    #[test]
    fn test_insert_and_get_concept() {
        let db = Database::open_memory().unwrap();
        insert_doc(&db, "doc-1");
        db.insert_topic("t1", "doc-1", "Topic", None).unwrap();

        db.insert_concept("c1", "t1", "Ownership", Some("Rust ownership model"))
            .unwrap();

        let concept = db.get_concept("c1").unwrap().expect("concept missing");
        assert_eq!(concept.concept_id, "c1");
        assert_eq!(concept.topic_id, "t1");
        assert_eq!(concept.name, "Ownership");
        assert_eq!(concept.summary.as_deref(), Some("Rust ownership model"));
        assert_eq!(concept.chunk_count, 0);
    }

    #[test]
    fn test_get_concept_not_found() {
        let db = Database::open_memory().unwrap();
        assert!(db.get_concept("nonexistent").unwrap().is_none());
    }

    #[test]
    fn test_insert_concept_with_null_summary() {
        let db = Database::open_memory().unwrap();
        insert_doc(&db, "doc-1");
        db.insert_topic("t1", "doc-1", "Topic", None).unwrap();

        db.insert_concept("c2", "t1", "No Summary", None).unwrap();

        let concept = db.get_concept("c2").unwrap().unwrap();
        assert!(concept.summary.is_none());
    }

    #[test]
    fn test_list_concepts_by_topic() {
        let db = Database::open_memory().unwrap();
        insert_doc(&db, "doc-1");
        db.insert_topic("t1", "doc-1", "A", None).unwrap();
        db.insert_topic("t2", "doc-1", "B", None).unwrap();

        db.insert_concept("c1", "t1", "X", None).unwrap();
        db.insert_concept("c2", "t1", "Y", None).unwrap();
        db.insert_concept("c3", "t2", "Z", None).unwrap();

        let concepts = db.list_concepts_by_topic("t1").unwrap();
        assert_eq!(concepts.len(), 2);
        assert!(concepts.iter().all(|c| c.topic_id == "t1"));
    }

    #[test]
    fn test_list_concepts_by_topic_empty() {
        let db = Database::open_memory().unwrap();
        let concepts = db.list_concepts_by_topic("ghost").unwrap();
        assert!(concepts.is_empty());
    }

    #[test]
    fn test_update_concept_chunk_count() {
        let db = Database::open_memory().unwrap();
        insert_doc(&db, "doc-1");
        db.insert_topic("t1", "doc-1", "Topic", None).unwrap();
        db.insert_concept("c1", "t1", "Concept", None).unwrap();

        // Insert chunks belonging to this concept
        let chunks: Vec<Chunk> = (0..2)
            .map(|i| Chunk {
                chunk_id: format!("cc{i}").into(),
                document_id: "doc-1".into(),
                section_id: None,
                chunk_index: i,
                text: format!("text {i}"),
                page: None,
                offset_start: None,
                offset_end: None,
                created_at: Utc::now(),
            })
            .collect();
        db.insert_chunks("doc-1", &chunks).unwrap();

        for i in 0..2 {
            db.set_chunk_hierarchy(&format!("cc{i}"), "t1", Some("c1"))
                .unwrap();
        }

        db.update_concept_chunk_count("c1").unwrap();

        let concept = db.get_concept("c1").unwrap().unwrap();
        assert_eq!(concept.chunk_count, 2);
    }

    #[test]
    fn test_insert_duplicate_concept_fails() {
        let db = Database::open_memory().unwrap();
        insert_doc(&db, "doc-1");
        db.insert_topic("t1", "doc-1", "A", None).unwrap();
        db.insert_concept("c1", "t1", "X", None).unwrap();
        assert!(db.insert_concept("c1", "t1", "Y", None).is_err());
    }
}
