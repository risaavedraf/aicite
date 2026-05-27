use std::path::Path;

use chrono::Utc;
use common::HarnessError;
use rusqlite::{params, OptionalExtension};

use crate::util::{format_dt, storage_err};
use crate::Database;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestBacklogItem {
    pub queue_id: String,
    pub source_path: String,
    pub display_name_override: Option<String>,
    pub status: String,
}

fn strip_windows_extended_prefix(path_str: String) -> String {
    if let Some(stripped) = path_str.strip_prefix(r"\\?\") {
        stripped.to_string()
    } else {
        path_str
    }
}

fn normalize_source_path(path: &Path) -> String {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    strip_windows_extended_prefix(canonical.to_string_lossy().to_string()).replace('\\', "/")
}

fn source_path_for_storage(path: &Path) -> String {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    strip_windows_extended_prefix(canonical.to_string_lossy().to_string())
}

fn build_idempotency_key(path: &Path) -> String {
    normalize_source_path(path)
}

impl Database {
    /// Upsert a durable ingest backlog item for a source path.
    pub fn upsert_ingest_backlog(
        &self,
        source_path: &Path,
        display_name_override: Option<&str>,
    ) -> Result<(), HarnessError> {
        let idempotency_key = build_idempotency_key(source_path);
        let queue_id = format!("queue:{idempotency_key}");
        let source_path_raw = source_path_for_storage(source_path);
        let now = format_dt(&Utc::now());

        self.conn
            .execute(
                "INSERT INTO ingest_backlog (
                    queue_id, idempotency_key, source_path, display_name_override, status, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, 'queued', ?5, ?6)
                 ON CONFLICT(idempotency_key) DO UPDATE SET
                    source_path = excluded.source_path,
                    display_name_override = excluded.display_name_override,
                    status = 'queued',
                    updated_at = excluded.updated_at",
                params![
                    queue_id,
                    idempotency_key,
                    source_path_raw,
                    display_name_override.map(str::to_string),
                    now,
                    now,
                ],
            )
            .map_err(storage_err)?;

        Ok(())
    }

    pub fn claim_next_ingest_backlog(&self) -> Result<Option<IngestBacklogItem>, HarnessError> {
        let now = format_dt(&Utc::now());
        let mut stmt = self
            .conn
            .prepare(
                "WITH next AS (
                    SELECT queue_id
                    FROM ingest_backlog
                    WHERE status = 'queued'
                    ORDER BY created_at ASC, queue_id ASC
                    LIMIT 1
                )
                UPDATE ingest_backlog
                SET status = 'claimed', updated_at = ?1
                WHERE queue_id = (SELECT queue_id FROM next)
                RETURNING queue_id, source_path, display_name_override, status",
            )
            .map_err(storage_err)?;

        let maybe_item = stmt
            .query_row(params![now], |row| {
                Ok(IngestBacklogItem {
                    queue_id: row.get(0)?,
                    source_path: row.get(1)?,
                    display_name_override: row.get(2)?,
                    status: row.get(3)?,
                })
            })
            .optional()
            .map_err(storage_err)?;

        Ok(maybe_item)
    }

    pub fn mark_ingest_backlog_done(&self, queue_id: &str) -> Result<(), HarnessError> {
        let now = format_dt(&Utc::now());
        self.conn
            .execute(
                "UPDATE ingest_backlog SET status = 'done', updated_at = ?1 WHERE queue_id = ?2",
                params![now, queue_id],
            )
            .map_err(storage_err)?;
        Ok(())
    }

    pub fn mark_ingest_backlog_failed(&self, queue_id: &str) -> Result<(), HarnessError> {
        let now = format_dt(&Utc::now());
        self.conn
            .execute(
                "UPDATE ingest_backlog SET status = 'failed', updated_at = ?1 WHERE queue_id = ?2",
                params![now, queue_id],
            )
            .map_err(storage_err)?;
        Ok(())
    }

    pub fn requeue_ingest_backlog(&self, queue_id: &str) -> Result<(), HarnessError> {
        let now = format_dt(&Utc::now());
        self.conn
            .execute(
                "UPDATE ingest_backlog SET status = 'queued', updated_at = ?1 WHERE queue_id = ?2",
                params![now, queue_id],
            )
            .map_err(storage_err)?;
        Ok(())
    }

    pub fn ingest_backlog_count(&self) -> Result<u64, HarnessError> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM ingest_backlog", [], |row| row.get(0))
            .map_err(storage_err)?;
        Ok(count as u64)
    }

    pub fn ingest_backlog_display_name_for_source(
        &self,
        source_path: &Path,
    ) -> Result<Option<String>, HarnessError> {
        let idempotency_key = build_idempotency_key(source_path);
        let mut stmt = self
            .conn
            .prepare("SELECT display_name_override FROM ingest_backlog WHERE idempotency_key = ?1")
            .map_err(storage_err)?;

        let mut rows = stmt.query(params![idempotency_key]).map_err(storage_err)?;
        match rows.next().map_err(storage_err)? {
            Some(row) => row.get(0).map(Some).map_err(storage_err),
            None => Ok(None),
        }
    }

    pub fn ingest_backlog_status_for_source(
        &self,
        source_path: &Path,
    ) -> Result<Option<String>, HarnessError> {
        let idempotency_key = build_idempotency_key(source_path);
        let mut stmt = self
            .conn
            .prepare("SELECT status FROM ingest_backlog WHERE idempotency_key = ?1")
            .map_err(storage_err)?;

        let mut rows = stmt.query(params![idempotency_key]).map_err(storage_err)?;
        match rows.next().map_err(storage_err)? {
            Some(row) => row.get(0).map(Some).map_err(storage_err),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn unique_test_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "aiharness_backlog_{}_{}.txt",
            name,
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ))
    }

    #[test]
    fn test_backlog_upsert_is_idempotent() {
        let db = Database::open_memory().unwrap();
        let path = unique_test_path("idem");

        db.upsert_ingest_backlog(&path, Some("doc")).unwrap();
        db.upsert_ingest_backlog(&path, Some("doc")).unwrap();

        assert_eq!(db.ingest_backlog_count().unwrap(), 1);
    }

    #[test]
    fn test_backlog_upsert_updates_display_name_override() {
        let db = Database::open_memory().unwrap();
        let path = unique_test_path("rename");

        db.upsert_ingest_backlog(&path, Some("v1")).unwrap();
        db.upsert_ingest_backlog(&path, Some("v2")).unwrap();

        assert_eq!(
            db.ingest_backlog_display_name_for_source(&path).unwrap(),
            Some("v2".to_string())
        );
        assert_eq!(db.ingest_backlog_count().unwrap(), 1);
    }

    #[test]
    fn test_claim_next_ingest_backlog_claims_fifo() {
        let db = Database::open_memory().unwrap();
        let first = unique_test_path("first");
        let second = unique_test_path("second");

        db.upsert_ingest_backlog(&first, Some("first-doc")).unwrap();
        db.upsert_ingest_backlog(&second, Some("second-doc"))
            .unwrap();

        let claimed = db.claim_next_ingest_backlog().unwrap().unwrap();
        assert_eq!(Path::new(&claimed.source_path), first.as_path());
        assert_eq!(claimed.status, "claimed");

        let claimed_second = db.claim_next_ingest_backlog().unwrap().unwrap();
        assert_eq!(Path::new(&claimed_second.source_path), second.as_path());

        let none_left = db.claim_next_ingest_backlog().unwrap();
        assert!(none_left.is_none());
    }

    #[test]
    fn test_mark_done_and_requeue_update_status() {
        let db = Database::open_memory().unwrap();
        let path = unique_test_path("status");

        db.upsert_ingest_backlog(&path, Some("doc")).unwrap();
        let claimed = db.claim_next_ingest_backlog().unwrap().unwrap();

        db.requeue_ingest_backlog(&claimed.queue_id).unwrap();
        assert_eq!(
            db.ingest_backlog_status_for_source(&path).unwrap(),
            Some("queued".to_string())
        );

        let claimed_again = db.claim_next_ingest_backlog().unwrap().unwrap();
        db.mark_ingest_backlog_done(&claimed_again.queue_id)
            .unwrap();
        assert_eq!(
            db.ingest_backlog_status_for_source(&path).unwrap(),
            Some("done".to_string())
        );

        db.mark_ingest_backlog_failed(&claimed_again.queue_id)
            .unwrap();
        assert_eq!(
            db.ingest_backlog_status_for_source(&path).unwrap(),
            Some("failed".to_string())
        );
    }
}
