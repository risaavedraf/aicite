use common::HarnessError;
use rusqlite::Connection;

const INITIAL_SCHEMA: &str = include_str!("001_initial.sql");
const TRACE_CITATIONS_SCHEMA: &str = include_str!("002_trace_citations.sql");
const DURABLE_INGEST_SCHEMA: &str = include_str!("003_durable_ingest.sql");
const RATE_LIMITS_SCHEMA: &str = include_str!("004_rate_limits.sql");
const SNAPSHOTS_SCHEMA: &str = include_str!("005_snapshots.sql");

/// Run pending migrations
pub fn run(conn: &Connection) -> Result<(), HarnessError> {
    // Create migrations table if it doesn't exist
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )
    .map_err(|e| HarnessError::StorageError {
        message: format!("Failed to create migrations table: {e}"),
    })?;

    // Check current version
    let current_version: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM _migrations",
            [],
            |row| row.get(0),
        )
        .map_err(|e| HarnessError::StorageError {
            message: format!("Failed to get migration version: {e}"),
        })?;

    // Run pending migrations
    if current_version < 1 {
        run_migration(conn, 1, INITIAL_SCHEMA)?;
    }

    if current_version < 2 {
        run_migration(conn, 2, TRACE_CITATIONS_SCHEMA)?;
    }

    if current_version < 3 {
        run_migration(conn, 3, DURABLE_INGEST_SCHEMA)?;
    }

    if current_version < 4 {
        run_migration(conn, 4, RATE_LIMITS_SCHEMA)?;
    }

    if current_version < 5 {
        run_migration(conn, 5, SNAPSHOTS_SCHEMA)?;
    }

    Ok(())
}

fn run_migration(conn: &Connection, version: i32, sql: &str) -> Result<(), HarnessError> {
    conn.execute_batch(sql)
        .map_err(|e| HarnessError::StorageError {
            message: format!("Migration {version} failed: {e}"),
        })?;

    conn.execute("INSERT INTO _migrations (version) VALUES (?1)", [version])
        .map_err(|e| HarnessError::StorageError {
            message: format!("Failed to record migration {version}: {e}"),
        })?;

    Ok(())
}
