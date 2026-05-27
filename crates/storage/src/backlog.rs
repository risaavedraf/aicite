use std::path::Path;

use chrono::Utc;
use common::HarnessError;
use rusqlite::params;

use crate::util::{format_dt, storage_err};
use crate::Database;

fn normalize_source_path(path: &Path) -> String {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    canonical.to_string_lossy().replace('\\', "/")
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
        let source_path_normalized = normalize_source_path(source_path);
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
                    source_path_normalized,
                    display_name_override.map(str::to_string),
                    now,
                    now,
                ],
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
        let source_path_normalized = normalize_source_path(source_path);
        let mut stmt = self
            .conn
            .prepare("SELECT display_name_override FROM ingest_backlog WHERE source_path = ?1")
            .map_err(storage_err)?;

        let mut rows = stmt
            .query(params![source_path_normalized])
            .map_err(storage_err)?;
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
}
