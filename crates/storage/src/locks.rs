use chrono::Utc;
use common::CiteError;
use rusqlite::params;

use crate::util::{format_dt, storage_err};
use crate::Database;

impl Database {
    /// Try to acquire a named durable lock.
    ///
    /// Returns `true` when lock was acquired, `false` when it is already held.
    pub fn try_acquire_lock(&self, lock_name: &str, owner_id: &str) -> Result<bool, CiteError> {
        let now = format_dt(&Utc::now());
        let inserted = self
            .conn
            .execute(
                "INSERT OR IGNORE INTO durable_locks (lock_name, owner_id, acquired_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![lock_name, owner_id, now, now],
            )
            .map_err(storage_err)?;

        Ok(inserted > 0)
    }

    /// Returns true when a named lock currently exists.
    pub fn is_lock_held(&self, lock_name: &str) -> Result<bool, CiteError> {
        let held: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM durable_locks WHERE lock_name = ?1",
                params![lock_name],
                |row| row.get(0),
            )
            .map_err(storage_err)?;

        Ok(held > 0)
    }

    /// Release a named durable lock only when owned by `owner_id`.
    pub fn release_lock(&self, lock_name: &str, owner_id: &str) -> Result<(), CiteError> {
        self.conn
            .execute(
                "DELETE FROM durable_locks WHERE lock_name = ?1 AND owner_id = ?2",
                params![lock_name, owner_id],
            )
            .map_err(storage_err)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_acquire_lock_serializes_by_lock_name() {
        let db = Database::open_memory().unwrap();

        assert!(db.try_acquire_lock("ingest_pipeline", "owner-a").unwrap());
        assert!(!db.try_acquire_lock("ingest_pipeline", "owner-b").unwrap());

        db.release_lock("ingest_pipeline", "owner-a").unwrap();
        assert!(db.try_acquire_lock("ingest_pipeline", "owner-b").unwrap());
    }

    #[test]
    fn test_release_lock_requires_owner() {
        let db = Database::open_memory().unwrap();

        assert!(db.try_acquire_lock("ingest_pipeline", "owner-a").unwrap());
        db.release_lock("ingest_pipeline", "owner-b").unwrap();

        assert!(!db.try_acquire_lock("ingest_pipeline", "owner-c").unwrap());

        db.release_lock("ingest_pipeline", "owner-a").unwrap();
        assert!(db.try_acquire_lock("ingest_pipeline", "owner-c").unwrap());
    }

    #[test]
    fn test_is_lock_held() {
        let db = Database::open_memory().unwrap();
        assert!(!db.is_lock_held("ingest_pipeline").unwrap());

        db.try_acquire_lock("ingest_pipeline", "owner-a").unwrap();
        assert!(db.is_lock_held("ingest_pipeline").unwrap());
    }
}
