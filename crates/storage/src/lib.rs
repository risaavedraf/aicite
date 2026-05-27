use common::HarnessError;
use rusqlite::Connection;
use std::path::Path;

mod migrations;

/// Database handle
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create the database at the given path
    pub fn open(data_dir: &Path) -> Result<Self, HarnessError> {
        let db_path = data_dir.join("harness.db");
        let conn = Connection::open(&db_path).map_err(|e| HarnessError::StorageError {
            message: format!("Failed to open database: {e}"),
        })?;

        // Enable WAL mode for concurrent reads
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| HarnessError::StorageError {
                message: format!("Failed to set WAL mode: {e}"),
            })?;

        // Set busy timeout to avoid immediate lock failures
        conn.pragma_update(None, "busy_timeout", 5000)
            .map_err(|e| HarnessError::StorageError {
                message: format!("Failed to set busy timeout: {e}"),
            })?;

        let mut db = Self { conn };
        db.run_migrations()?;

        Ok(db)
    }

    /// Run pending migrations
    fn run_migrations(&mut self) -> Result<(), HarnessError> {
        migrations::run(&self.conn)
    }

    /// Check database health
    pub fn check_health(&self) -> Result<(), HarnessError> {
        self.conn
            .execute_batch("SELECT 1")
            .map_err(|e| HarnessError::StorageError {
                message: format!("Health check failed: {e}"),
            })
    }

    /// Get the underlying connection
    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        // Placeholder for storage tests
        // Real tests will use a temp directory with SQLite
    }
}
