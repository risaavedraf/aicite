use chrono::{DateTime, NaiveDateTime, Utc};
use common::types::Chunk;
use common::CiteError;

/// Convert any Display error into CiteError::StorageError.
pub fn storage_err(e: impl std::fmt::Display) -> CiteError {
    CiteError::StorageError {
        message: e.to_string(),
    }
}

/// Format a DateTime<Utc> as SQLite-compatible datetime string.
pub fn format_dt(dt: &DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Parse a SQLite datetime string into DateTime<Utc>.
pub fn parse_dt(s: &str) -> Result<DateTime<Utc>, CiteError> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .map(|ndt| ndt.and_utc())
        .map_err(|e| CiteError::StorageError {
            message: format!("Failed to parse datetime '{s}': {e}"),
        })
}

/// Convert a database row into a `Chunk`.
///
/// Expects columns: chunk_id, document_id, section_id, chunk_index, text,
/// page, offset_start, offset_end, created_at.
pub(crate) fn row_to_chunk(row: &rusqlite::Row<'_>) -> Result<Chunk, CiteError> {
    let created_at_str: String = row.get("created_at").map_err(storage_err)?;

    Ok(Chunk {
        chunk_id: row.get("chunk_id").map_err(storage_err)?,
        document_id: row.get("document_id").map_err(storage_err)?,
        section_id: row.get("section_id").map_err(storage_err)?,
        chunk_index: u32::try_from(row.get::<_, i64>("chunk_index").map_err(storage_err)?)
            .map_err(|e| storage_err(format!("chunk_index overflow: {e}")))?,
        text: row.get("text").map_err(storage_err)?,
        page: row
            .get::<_, Option<i64>>("page")
            .map_err(storage_err)?
            .map(u32::try_from)
            .transpose()
            .map_err(|e| storage_err(format!("page overflow: {e}")))?,
        offset_start: row
            .get::<_, Option<i64>>("offset_start")
            .map_err(storage_err)?
            .map(u32::try_from)
            .transpose()
            .map_err(|e| storage_err(format!("offset_start overflow: {e}")))?,
        offset_end: row
            .get::<_, Option<i64>>("offset_end")
            .map_err(storage_err)?
            .map(u32::try_from)
            .transpose()
            .map_err(|e| storage_err(format!("offset_end overflow: {e}")))?,
        created_at: parse_dt(&created_at_str)?,
    })
}
