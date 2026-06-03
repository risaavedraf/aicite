use common::CiteError;
use rusqlite::Connection;
use std::path::Path;

pub mod backlog;
pub mod chunks;
pub mod concepts;
pub mod documents;
pub mod embeddings;
pub mod locks;
mod migrations;
pub mod rate_limits;
pub mod semantic_links;
pub mod snapshots;
pub mod topics;
pub mod traces;
mod util;

/// Database handle
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create the database at the given path
    pub fn open(data_dir: &Path) -> Result<Self, CiteError> {
        let db_path = data_dir.join("cite.db");
        let conn = Connection::open(&db_path).map_err(|e| CiteError::StorageError {
            message: format!("Failed to open database: {e}"),
        })?;

        // Enable WAL mode for concurrent reads
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| CiteError::StorageError {
                message: format!("Failed to set WAL mode: {e}"),
            })?;

        // Set busy timeout to avoid immediate lock failures
        conn.pragma_update(None, "busy_timeout", 5000)
            .map_err(|e| CiteError::StorageError {
                message: format!("Failed to set busy timeout: {e}"),
            })?;

        // Enable foreign key enforcement
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| CiteError::StorageError {
                message: format!("Failed to enable foreign keys: {e}"),
            })?;

        let mut db = Self { conn };
        db.run_migrations()?;

        Ok(db)
    }

    /// Run pending migrations
    fn run_migrations(&mut self) -> Result<(), CiteError> {
        migrations::run(&self.conn)
    }

    /// Check database health
    pub fn check_health(&self) -> Result<(), CiteError> {
        self.conn
            .execute_batch("SELECT 1")
            .map_err(|e| CiteError::StorageError {
                message: format!("Health check failed: {e}"),
            })
    }

    /// Get the underlying connection
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Open an in-memory database for testing.
    pub fn open_memory() -> Result<Self, CiteError> {
        let conn = Connection::open_in_memory().map_err(|e| CiteError::StorageError {
            message: format!("Failed to open in-memory database: {e}"),
        })?;

        let mut db = Self { conn };

        // Enable foreign key enforcement
        db.conn
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| CiteError::StorageError {
                message: format!("Failed to enable foreign keys: {e}"),
            })?;

        db.run_migrations()?;
        Ok(db)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use common::types::{Chunk, Document, DocumentStatus, FileType};
    use std::path::PathBuf;

    #[test]
    fn test_open_memory_and_health_check() {
        let db = Database::open_memory().unwrap();
        db.check_health().unwrap();
    }

    #[test]
    fn test_fk_pragma_returns_1() {
        let db = Database::open_memory().unwrap();
        let fk_value: i32 = db
            .conn()
            .pragma_query_value(None, "foreign_keys", |row| row.get(0))
            .expect("PRAGMA foreign_keys query failed");
        assert_eq!(fk_value, 1, "foreign_keys should be ON");
    }

    #[test]
    fn test_fk_rejects_orphan_chunk() {
        let db = Database::open_memory().unwrap();

        // Try to insert a chunk referencing a non-existent document
        let chunk = Chunk {
            chunk_id: "chunk-orphan".to_string(),
            document_id: "nonexistent-doc".to_string(),
            section_id: None,
            chunk_index: 0,
            text: "orphan chunk".to_string(),
            page: None,
            offset_start: None,
            offset_end: None,
            created_at: Utc::now(),
        };

        let result = db.insert_chunks("nonexistent-doc", &[chunk]);
        assert!(result.is_err(), "Expected FK violation, got Ok");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("FOREIGN KEY") || err_msg.contains("foreign key"),
            "Error should mention FK violation: {err_msg}"
        );
    }

    #[test]
    fn test_fk_allows_valid_insert() {
        let db = Database::open_memory().unwrap();

        // Insert a document first
        let doc = Document {
            document_id: "doc-valid".to_string(),
            display_name: "test.txt".to_string(),
            file_path: PathBuf::from("/test.txt"),
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

        // Now insert a chunk referencing the document
        let chunk = Chunk {
            chunk_id: "chunk-valid".to_string(),
            document_id: "doc-valid".to_string(),
            section_id: None,
            chunk_index: 0,
            text: "valid chunk".to_string(),
            page: None,
            offset_start: None,
            offset_end: None,
            created_at: Utc::now(),
        };

        let result = db.insert_chunks("doc-valid", &[chunk]);
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result.err());
    }

    #[test]
    fn test_row_to_chunk_valid_index() {
        let db = Database::open_memory().unwrap();

        let doc = Document {
            document_id: "doc-cast".to_string(),
            display_name: "cast-test.txt".to_string(),
            file_path: PathBuf::from("/cast-test.txt"),
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

        let chunk = Chunk {
            chunk_id: "chunk-cast-42".to_string(),
            document_id: "doc-cast".to_string(),
            section_id: None,
            chunk_index: 42,
            text: "test chunk".to_string(),
            page: Some(3),
            offset_start: Some(100),
            offset_end: Some(200),
            created_at: Utc::now(),
        };
        db.insert_chunks("doc-cast", &[chunk]).unwrap();

        // Verify via raw query that try_from conversion works correctly
        let (chunk_index, page, offset_start, offset_end): (i64, Option<i64>, Option<i64>, Option<i64>) = db
            .conn()
            .query_row(
                "SELECT chunk_index, page, offset_start, offset_end FROM chunks WHERE chunk_id = 'chunk-cast-42'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(chunk_index, 42i64);
        assert_eq!(page, Some(3i64));
        assert_eq!(offset_start, Some(100i64));
        assert_eq!(offset_end, Some(200i64));
    }
}
