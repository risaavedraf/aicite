use chrono::Utc;
use common::types::ErrorInfo;
use common::CiteError;
use rusqlite::{params, OptionalExtension};

use crate::util::{format_dt, storage_err};
use crate::Database;

/// Snapshot state values
const STATE_BUILDING: &str = "building";
const STATE_ACTIVE: &str = "active";
const STATE_SUPERSEDED: &str = "superseded";
const STATE_FAILED: &str = "failed";

/// Result of snapshot activation
#[derive(Debug, Clone)]
pub struct ActivateSnapshotResult {
    pub snapshot_id: String,
    pub previous_snapshot_id: Option<String>,
}

impl Database {
    /// Create a new snapshot in `building` state.
    pub fn begin_snapshot_build(&self, snapshot_id: &str) -> Result<(), CiteError> {
        let now = format_dt(&Utc::now());
        self.conn
            .execute(
                "INSERT INTO corpus_snapshots (snapshot_id, state, created_at)
                 VALUES (?1, ?2, ?3)",
                params![snapshot_id, STATE_BUILDING, now],
            )
            .map_err(storage_err)?;
        Ok(())
    }

    /// Attach a document to a building snapshot.
    pub fn attach_document_to_snapshot(
        &self,
        snapshot_id: &str,
        document_id: &str,
    ) -> Result<(), CiteError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO snapshot_members (snapshot_id, document_id)
                 VALUES (?1, ?2)",
                params![snapshot_id, document_id],
            )
            .map_err(storage_err)?;
        Ok(())
    }

    /// Atomically activate a snapshot: set it active and supersede the previous one.
    ///
    /// This runs in a single transaction to guarantee no mixed visibility.
    pub fn activate_snapshot(
        &self,
        snapshot_id: &str,
    ) -> Result<ActivateSnapshotResult, CiteError> {
        let now = format_dt(&Utc::now());

        let tx = self.conn.unchecked_transaction().map_err(storage_err)?;

        // Verify snapshot exists and is in building state
        let state: String = tx
            .query_row(
                "SELECT state FROM corpus_snapshots WHERE snapshot_id = ?1",
                params![snapshot_id],
                |row| row.get(0),
            )
            .map_err(|e| CiteError::StorageError {
                message: format!("Snapshot not found: {e}"),
            })?;

        if state != STATE_BUILDING {
            return Err(CiteError::InvalidParameter {
                message: format!(
                    "Cannot activate snapshot in state '{state}'; must be '{STATE_BUILDING}'"
                ),
            });
        }

        // Read current active pointer (if any)
        let previous_snapshot_id: Option<String> = tx
            .query_row(
                "SELECT active_snapshot_id FROM snapshot_pointer WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_err)?;

        // Supersede previous snapshot if it exists
        if let Some(ref prev_id) = previous_snapshot_id {
            tx.execute(
                "UPDATE corpus_snapshots
                 SET state = ?1, superseded_at = ?2
                 WHERE snapshot_id = ?3 AND state = ?4",
                params![STATE_SUPERSEDED, now, prev_id, STATE_ACTIVE],
            )
            .map_err(storage_err)?;
        }

        // Activate the new snapshot
        tx.execute(
            "UPDATE corpus_snapshots
             SET state = ?1, activated_at = ?2
             WHERE snapshot_id = ?3",
            params![STATE_ACTIVE, now, snapshot_id],
        )
        .map_err(storage_err)?;

        // Upsert the active pointer (single-row table)
        tx.execute(
            "INSERT INTO snapshot_pointer (id, active_snapshot_id) VALUES (1, ?1)
             ON CONFLICT(id) DO UPDATE SET active_snapshot_id = excluded.active_snapshot_id",
            params![snapshot_id],
        )
        .map_err(storage_err)?;

        tx.commit().map_err(storage_err)?;

        Ok(ActivateSnapshotResult {
            snapshot_id: snapshot_id.to_string(),
            previous_snapshot_id,
        })
    }

    /// Mark a building snapshot as failed.
    pub fn mark_snapshot_failed(
        &self,
        snapshot_id: &str,
        error: &ErrorInfo,
    ) -> Result<(), CiteError> {
        let n = self
            .conn
            .execute(
                "UPDATE corpus_snapshots
                 SET state = ?1, error_code = ?2, error_message = ?3
                 WHERE snapshot_id = ?4 AND state = ?5",
                params![
                    STATE_FAILED,
                    error.code,
                    error.message,
                    snapshot_id,
                    STATE_BUILDING
                ],
            )
            .map_err(storage_err)?;

        if n == 0 {
            return Err(CiteError::StorageError {
                message: format!("Snapshot {snapshot_id} not found or not in building state"),
            });
        }
        Ok(())
    }

    /// Get the currently active snapshot ID, if any.
    pub fn get_active_snapshot_id(&self) -> Result<Option<String>, CiteError> {
        self.conn
            .query_row(
                "SELECT active_snapshot_id FROM snapshot_pointer WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_err)
    }

    /// Get the document IDs belonging to the active snapshot.
    ///
    /// Returns `None` if no active snapshot exists (meaning all ready docs are visible).
    pub fn get_active_snapshot_member_ids(&self) -> Result<Option<Vec<String>>, CiteError> {
        let Some(active_id) = self.get_active_snapshot_id()? else {
            return Ok(None);
        };

        let mut stmt = self
            .conn
            .prepare("SELECT document_id FROM snapshot_members WHERE snapshot_id = ?1")
            .map_err(storage_err)?;

        let ids = stmt
            .query_map(params![active_id], |row| row.get(0))
            .map_err(storage_err)?
            .collect::<Result<Vec<String>, _>>()
            .map_err(storage_err)?;

        Ok(Some(ids))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_begin_snapshot_build_creates_building_snapshot() {
        let db = Database::open_memory().unwrap();
        db.begin_snapshot_build("snap-1").unwrap();

        let active = db.get_active_snapshot_id().unwrap();
        assert!(active.is_none(), "building snapshot should not be active");
    }

    #[test]
    fn test_activate_snapshot_sets_active_pointer() {
        let db = Database::open_memory().unwrap();
        db.begin_snapshot_build("snap-1").unwrap();
        db.attach_document_to_snapshot("snap-1", "doc-1").unwrap();

        let result = db.activate_snapshot("snap-1").unwrap();
        assert_eq!(result.snapshot_id, "snap-1");
        assert!(result.previous_snapshot_id.is_none());

        let active = db.get_active_snapshot_id().unwrap();
        assert_eq!(active.as_deref(), Some("snap-1"));
    }

    #[test]
    fn test_activate_snapshot_supersedes_previous() {
        let db = Database::open_memory().unwrap();

        db.begin_snapshot_build("snap-1").unwrap();
        db.activate_snapshot("snap-1").unwrap();

        db.begin_snapshot_build("snap-2").unwrap();
        db.attach_document_to_snapshot("snap-2", "doc-2").unwrap();
        let result = db.activate_snapshot("snap-2").unwrap();

        assert_eq!(result.previous_snapshot_id.as_deref(), Some("snap-1"));
        assert_eq!(
            db.get_active_snapshot_id().unwrap().as_deref(),
            Some("snap-2")
        );
    }

    #[test]
    fn test_activate_non_building_snapshot_fails() {
        let db = Database::open_memory().unwrap();
        db.begin_snapshot_build("snap-1").unwrap();
        db.activate_snapshot("snap-1").unwrap();

        // Trying to activate an already-active snapshot should fail
        let result = db.activate_snapshot("snap-1");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CiteError::InvalidParameter { .. }
        ));
    }

    #[test]
    fn test_mark_snapshot_failed() {
        let db = Database::open_memory().unwrap();
        db.begin_snapshot_build("snap-1").unwrap();

        db.mark_snapshot_failed(
            "snap-1",
            &ErrorInfo {
                code: "BUILD_FAILED".to_string(),
                message: "embedding provider error".to_string(),
            },
        )
        .unwrap();

        // Should not appear as active
        assert!(db.get_active_snapshot_id().unwrap().is_none());
    }

    #[test]
    fn test_get_active_snapshot_member_ids_returns_none_when_no_active() {
        let db = Database::open_memory().unwrap();
        assert!(db.get_active_snapshot_member_ids().unwrap().is_none());
    }

    #[test]
    fn test_get_active_snapshot_member_ids_returns_correct_members() {
        let db = Database::open_memory().unwrap();

        db.begin_snapshot_build("snap-1").unwrap();
        db.attach_document_to_snapshot("snap-1", "doc-a").unwrap();
        db.attach_document_to_snapshot("snap-1", "doc-b").unwrap();
        db.attach_document_to_snapshot("snap-1", "doc-c").unwrap();
        db.activate_snapshot("snap-1").unwrap();

        let members = db.get_active_snapshot_member_ids().unwrap().unwrap();
        assert_eq!(members.len(), 3);
        assert!(members.contains(&"doc-a".to_string()));
        assert!(members.contains(&"doc-b".to_string()));
        assert!(members.contains(&"doc-c".to_string()));
    }

    #[test]
    fn test_attach_document_is_idempotent() {
        let db = Database::open_memory().unwrap();
        db.begin_snapshot_build("snap-1").unwrap();

        db.attach_document_to_snapshot("snap-1", "doc-1").unwrap();
        db.attach_document_to_snapshot("snap-1", "doc-1").unwrap(); // duplicate

        let _members = db.get_active_snapshot_member_ids().unwrap();
        // Snapshot not activated yet, so members query returns None
        // Instead, query directly
        let count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM snapshot_members WHERE snapshot_id = 'snap-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "duplicate attach should be ignored");
    }

    #[test]
    fn test_activate_snapshot_atomic_no_mixed_visibility() {
        let db = Database::open_memory().unwrap();

        // Build and activate first snapshot
        db.begin_snapshot_build("snap-1").unwrap();
        db.attach_document_to_snapshot("snap-1", "doc-v1").unwrap();
        db.activate_snapshot("snap-1").unwrap();

        // Build and activate second snapshot
        db.begin_snapshot_build("snap-2").unwrap();
        db.attach_document_to_snapshot("snap-2", "doc-v2").unwrap();
        db.activate_snapshot("snap-2").unwrap();

        // Active snapshot should be snap-2 with its members
        let active = db.get_active_snapshot_id().unwrap().unwrap();
        assert_eq!(active, "snap-2");

        let members = db.get_active_snapshot_member_ids().unwrap().unwrap();
        assert_eq!(members.len(), 1);
        assert!(members.contains(&"doc-v2".to_string()));
    }

    #[test]
    fn test_activate_nonexistent_snapshot_fails() {
        let db = Database::open_memory().unwrap();
        let result = db.activate_snapshot("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_activate_snapshot_returns_none_when_no_pointer() {
        let db = Database::open_memory().unwrap();
        // No snapshot_pointer row exists; reading it via optional() should return None
        let active = db.get_active_snapshot_id().unwrap();
        assert!(active.is_none());
    }
}
