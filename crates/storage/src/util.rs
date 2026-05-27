use chrono::{DateTime, NaiveDateTime, Utc};
use common::HarnessError;

/// Convert any Display error into HarnessError::StorageError.
pub fn storage_err(e: impl std::fmt::Display) -> HarnessError {
    HarnessError::StorageError {
        message: e.to_string(),
    }
}

/// Format a DateTime<Utc> as SQLite-compatible datetime string.
pub fn format_dt(dt: &DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Parse a SQLite datetime string into DateTime<Utc>.
pub fn parse_dt(s: &str) -> Result<DateTime<Utc>, HarnessError> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .map(|ndt| ndt.and_utc())
        .map_err(|e| HarnessError::StorageError {
            message: format!("Failed to parse datetime '{s}': {e}"),
        })
}
