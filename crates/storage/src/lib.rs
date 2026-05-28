use common::CiteError;
use rusqlite::Connection;
use std::path::Path;

pub mod backlog;
pub mod chunks;
pub mod documents;
pub mod embeddings;
pub mod locks;
mod migrations;
pub mod rate_limits;
pub mod snapshots;
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
        db.run_migrations()?;
        Ok(db)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_memory_and_health_check() {
        let db = Database::open_memory().unwrap();
        db.check_health().unwrap();
    }
}
