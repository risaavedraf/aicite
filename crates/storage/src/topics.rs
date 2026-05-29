use common::CiteError;
use rusqlite::params;

use crate::util::storage_err;
use crate::Database;

// ---------------------------------------------------------------------------
// Row type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct TopicRow {
    pub topic_id: String,
    pub document_id: String,
    pub name: String,
    pub summary: Option<String>,
    pub chunk_count: i64,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// CRUD operations
// ---------------------------------------------------------------------------

impl Database {
    /// Insert a new topic.
    pub fn insert_topic(
        &self,
        topic_id: &str,
        document_id: &str,
        name: &str,
        summary: Option<&str>,
    ) -> Result<(), CiteError> {
        self.conn
            .execute(
                "INSERT INTO topics (topic_id, document_id, name, summary) VALUES (?1, ?2, ?3, ?4)",
                params![topic_id, document_id, name, summary],
            )
            .map_err(storage_err)?;
        Ok(())
    }

    /// Get a topic by ID. Returns `None` when not found.
    pub fn get_topic(&self, topic_id: &str) -> Result<Option<TopicRow>, CiteError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT topic_id, document_id, name, summary, chunk_count, created_at
                 FROM topics WHERE topic_id = ?1",
            )
            .map_err(storage_err)?;

        let mut rows = stmt.query(params![topic_id]).map_err(storage_err)?;

        match rows.next().map_err(storage_err)? {
            Some(row) => Ok(Some(TopicRow {
                topic_id: row.get(0).map_err(storage_err)?,
                document_id: row.get(1).map_err(storage_err)?,
                name: row.get(2).map_err(storage_err)?,
                summary: row.get(3).map_err(storage_err)?,
                chunk_count: row.get(4).map_err(storage_err)?,
                created_at: row.get(5).map_err(storage_err)?,
            })),
            None => Ok(None),
        }
    }

    /// List all topics for a document, ordered by creation time.
    pub fn list_topics_by_document(&self, document_id: &str) -> Result<Vec<TopicRow>, CiteError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT topic_id, document_id, name, summary, chunk_count, created_at
                 FROM topics WHERE document_id = ?1 ORDER BY created_at",
            )
            .map_err(storage_err)?;

        let mut rows = stmt.query(params![document_id]).map_err(storage_err)?;
        let mut result = Vec::new();

        while let Some(row) = rows.next().map_err(storage_err)? {
            result.push(TopicRow {
                topic_id: row.get(0).map_err(storage_err)?,
                document_id: row.get(1).map_err(storage_err)?,
                name: row.get(2).map_err(storage_err)?,
                summary: row.get(3).map_err(storage_err)?,
                chunk_count: row.get(4).map_err(storage_err)?,
                created_at: row.get(5).map_err(storage_err)?,
            });
        }
        Ok(result)
    }

    /// Recalculate chunk_count for a topic from the chunks table.
    pub fn update_topic_chunk_count(&self, topic_id: &str) -> Result<(), CiteError> {
        self.conn
            .execute(
                "UPDATE topics SET chunk_count = (
                    SELECT COUNT(*) FROM chunks WHERE topic_id = ?1
                 ) WHERE topic_id = ?1",
                params![topic_id],
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

    #[test]
    fn test_insert_and_get_topic() {
        let db = Database::open_memory().unwrap();
        insert_doc(&db, "doc-1");

        db.insert_topic("t1", "doc-1", "Rust Basics", Some("Intro to Rust"))
            .unwrap();

        let topic = db.get_topic("t1").unwrap().expect("topic missing");
        assert_eq!(topic.topic_id, "t1");
        assert_eq!(topic.document_id, "doc-1");
        assert_eq!(topic.name, "Rust Basics");
        assert_eq!(topic.summary.as_deref(), Some("Intro to Rust"));
        assert_eq!(topic.chunk_count, 0);
    }

    #[test]
    fn test_get_topic_not_found() {
        let db = Database::open_memory().unwrap();
        assert!(db.get_topic("nonexistent").unwrap().is_none());
    }

    #[test]
    fn test_insert_topic_with_null_summary() {
        let db = Database::open_memory().unwrap();
        insert_doc(&db, "doc-1");

        db.insert_topic("t2", "doc-1", "No Summary", None).unwrap();

        let topic = db.get_topic("t2").unwrap().unwrap();
        assert!(topic.summary.is_none());
    }

    #[test]
    fn test_list_topics_by_document() {
        let db = Database::open_memory().unwrap();
        insert_doc(&db, "doc-1");
        insert_doc(&db, "doc-2");

        db.insert_topic("t1", "doc-1", "A", None).unwrap();
        db.insert_topic("t2", "doc-1", "B", None).unwrap();
        db.insert_topic("t3", "doc-2", "C", None).unwrap();

        let topics = db.list_topics_by_document("doc-1").unwrap();
        assert_eq!(topics.len(), 2);
        assert!(topics.iter().all(|t| t.document_id == "doc-1"));
    }

    #[test]
    fn test_list_topics_by_document_empty() {
        let db = Database::open_memory().unwrap();
        let topics = db.list_topics_by_document("ghost").unwrap();
        assert!(topics.is_empty());
    }

    #[test]
    fn test_update_topic_chunk_count() {
        let db = Database::open_memory().unwrap();
        insert_doc(&db, "doc-1");
        db.insert_topic("t1", "doc-1", "Topic", None).unwrap();

        // Insert chunks belonging to this topic
        let chunks: Vec<Chunk> = (0..3)
            .map(|i| Chunk {
                chunk_id: format!("tc{i}"),
                document_id: "doc-1".to_string(),
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

        // Set topic_id on those chunks
        for i in 0..3 {
            db.set_chunk_hierarchy(&format!("tc{i}"), "t1", None)
                .unwrap();
        }

        db.update_topic_chunk_count("t1").unwrap();

        let topic = db.get_topic("t1").unwrap().unwrap();
        assert_eq!(topic.chunk_count, 3);
    }

    #[test]
    fn test_insert_duplicate_topic_fails() {
        let db = Database::open_memory().unwrap();
        insert_doc(&db, "doc-1");
        db.insert_topic("t1", "doc-1", "A", None).unwrap();
        assert!(db.insert_topic("t1", "doc-1", "B", None).is_err());
    }
}
