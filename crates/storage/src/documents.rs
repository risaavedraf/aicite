use std::path::PathBuf;

use chrono::Utc;
use common::types::{Document, DocumentStatus, ErrorInfo, FileType};
use common::HarnessError;
use rusqlite::{params, Row};

use crate::util::{format_dt, parse_dt, storage_err};
use crate::Database;

// ---------------------------------------------------------------------------
// Enum <-> string helpers
// ---------------------------------------------------------------------------

fn status_to_str(s: &DocumentStatus) -> &'static str {
    match s {
        DocumentStatus::Pending => "pending",
        DocumentStatus::Processing => "processing",
        DocumentStatus::Ready => "ready",
        DocumentStatus::Failed => "failed",
    }
}

fn parse_status(s: &str) -> Result<DocumentStatus, HarnessError> {
    match s {
        "pending" => Ok(DocumentStatus::Pending),
        "processing" => Ok(DocumentStatus::Processing),
        "ready" => Ok(DocumentStatus::Ready),
        "failed" => Ok(DocumentStatus::Failed),
        other => Err(HarnessError::StorageError {
            message: format!("Unknown document status: {other}"),
        }),
    }
}

fn file_type_to_str(ft: &FileType) -> &'static str {
    match ft {
        FileType::Pdf => "pdf",
        FileType::Txt => "txt",
        FileType::Md => "md",
    }
}

fn parse_file_type(s: &str) -> Result<FileType, HarnessError> {
    match s {
        "pdf" => Ok(FileType::Pdf),
        "txt" => Ok(FileType::Txt),
        "md" => Ok(FileType::Md),
        other => Err(HarnessError::StorageError {
            message: format!("Unknown file type: {other}"),
        }),
    }
}

// ---------------------------------------------------------------------------
// Row -> Document conversion
// ---------------------------------------------------------------------------

fn row_to_document(row: &Row<'_>) -> Result<Document, HarnessError> {
    let document_id: String = row.get("document_id").map_err(storage_err)?;
    let display_name: String = row.get("display_name").map_err(storage_err)?;
    let file_path_str: String = row.get("file_path").map_err(storage_err)?;
    let file_type_str: String = row.get("file_type").map_err(storage_err)?;
    let file_size_bytes: i64 = row.get("file_size_bytes").map_err(storage_err)?;
    let status_str: String = row.get("status").map_err(storage_err)?;
    let chunk_count: i64 = row.get("chunk_count").map_err(storage_err)?;
    let retry_count: i64 = row.get("retry_count").map_err(storage_err)?;
    let max_retry_count: i64 = row.get("max_retry_count").map_err(storage_err)?;
    let next_retry_at_str: Option<String> = row.get("next_retry_at").map_err(storage_err)?;
    let error_code: Option<String> = row.get("error_code").map_err(storage_err)?;
    let error_message: Option<String> = row.get("error_message").map_err(storage_err)?;
    let created_at_str: String = row.get("created_at").map_err(storage_err)?;
    let updated_at_str: String = row.get("updated_at").map_err(storage_err)?;

    let next_retry_at = match next_retry_at_str {
        Some(s) => Some(parse_dt(&s)?),
        None => None,
    };

    let error = match (error_code, error_message) {
        (Some(code), Some(message)) => Some(ErrorInfo { code, message }),
        _ => None,
    };

    Ok(Document {
        document_id,
        display_name,
        file_path: PathBuf::from(file_path_str),
        file_type: parse_file_type(&file_type_str)?,
        file_size_bytes: file_size_bytes as u64,
        status: parse_status(&status_str)?,
        chunk_count: chunk_count as u32,
        retry_count: retry_count as u32,
        max_retry_count: max_retry_count as u32,
        next_retry_at,
        error,
        created_at: parse_dt(&created_at_str)?,
        updated_at: parse_dt(&updated_at_str)?,
    })
}

// ---------------------------------------------------------------------------
// CRUD operations
// ---------------------------------------------------------------------------

impl Database {
    /// Insert a new document.
    pub fn insert_document(&self, doc: &Document) -> Result<(), HarnessError> {
        self.conn
            .execute(
                "INSERT INTO documents (
                    document_id, display_name, file_path, file_type, file_size_bytes,
                    status, chunk_count, retry_count, max_retry_count,
                    next_retry_at, error_code, error_message,
                    created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    doc.document_id,
                    doc.display_name,
                    doc.file_path.to_string_lossy().as_ref(),
                    file_type_to_str(&doc.file_type),
                    doc.file_size_bytes as i64,
                    status_to_str(&doc.status),
                    doc.chunk_count as i64,
                    doc.retry_count as i64,
                    doc.max_retry_count as i64,
                    doc.next_retry_at.as_ref().map(format_dt),
                    doc.error.as_ref().map(|e| e.code.as_str()),
                    doc.error.as_ref().map(|e| e.message.as_str()),
                    format_dt(&doc.created_at),
                    format_dt(&doc.updated_at),
                ],
            )
            .map_err(storage_err)?;
        Ok(())
    }

    /// Get a document by ID. Returns `None` when not found.
    pub fn get_document(&self, id: &str) -> Result<Option<Document>, HarnessError> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM documents WHERE document_id = ?1")
            .map_err(storage_err)?;

        let mut rows = stmt.query(params![id]).map_err(storage_err)?;

        match rows.next().map_err(storage_err)? {
            Some(row) => Ok(Some(row_to_document(row)?)),
            None => Ok(None),
        }
    }

    /// List all documents ordered by creation time (newest first).
    pub fn list_documents(&self) -> Result<Vec<Document>, HarnessError> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM documents ORDER BY created_at DESC")
            .map_err(storage_err)?;

        let mut rows = stmt.query([]).map_err(storage_err)?;

        let mut documents = Vec::new();
        while let Some(row) = rows.next().map_err(storage_err)? {
            documents.push(row_to_document(row)?);
        }
        Ok(documents)
    }

    /// List documents filtered by status.
    pub fn list_documents_by_status(
        &self,
        status: DocumentStatus,
    ) -> Result<Vec<Document>, HarnessError> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM documents WHERE status = ?1 ORDER BY created_at ASC")
            .map_err(storage_err)?;

        let mut rows = stmt
            .query(params![status_to_str(&status)])
            .map_err(storage_err)?;

        let mut documents = Vec::new();
        while let Some(row) = rows.next().map_err(storage_err)? {
            documents.push(row_to_document(row)?);
        }
        Ok(documents)
    }

    /// List all documents currently marked as processing.
    pub fn list_processing_documents(&self) -> Result<Vec<Document>, HarnessError> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM documents WHERE status = 'processing' ORDER BY created_at ASC")
            .map_err(storage_err)?;

        let mut rows = stmt.query([]).map_err(storage_err)?;

        let mut documents = Vec::new();
        while let Some(row) = rows.next().map_err(storage_err)? {
            documents.push(row_to_document(row)?);
        }
        Ok(documents)
    }

    /// Update the status (and optional error info) of a document.
    pub fn update_document_status(
        &self,
        id: &str,
        status: DocumentStatus,
        error: Option<ErrorInfo>,
    ) -> Result<(), HarnessError> {
        let n = self
            .conn
            .execute(
                "UPDATE documents SET status = ?1, error_code = ?2, error_message = ?3, updated_at = ?4
                 WHERE document_id = ?5",
                params![
                    status_to_str(&status),
                    error.as_ref().map(|e| e.code.as_str()),
                    error.as_ref().map(|e| e.message.as_str()),
                    format_dt(&Utc::now()),
                    id,
                ],
            )
            .map_err(storage_err)?;

        if n == 0 {
            return Err(HarnessError::DocumentNotFound {
                document_id: id.to_string(),
            });
        }
        Ok(())
    }

    /// Set the chunk count on a document.
    pub fn update_document_chunk_count(&self, id: &str, count: u32) -> Result<(), HarnessError> {
        let n = self
            .conn
            .execute(
                "UPDATE documents SET chunk_count = ?1, updated_at = ?2 WHERE document_id = ?3",
                params![count as i64, format_dt(&Utc::now()), id],
            )
            .map_err(storage_err)?;

        if n == 0 {
            return Err(HarnessError::DocumentNotFound {
                document_id: id.to_string(),
            });
        }
        Ok(())
    }

    /// Increment retry_count by 1.
    pub fn increment_retry_count(&self, id: &str) -> Result<(), HarnessError> {
        let n = self
            .conn
            .execute(
                "UPDATE documents SET retry_count = retry_count + 1, updated_at = ?1 WHERE document_id = ?2",
                params![format_dt(&Utc::now()), id],
            )
            .map_err(storage_err)?;

        if n == 0 {
            return Err(HarnessError::DocumentNotFound {
                document_id: id.to_string(),
            });
        }
        Ok(())
    }

    /// Reset retry_count back to 0.
    pub fn reset_retry_count(&self, id: &str) -> Result<(), HarnessError> {
        let n = self
            .conn
            .execute(
                "UPDATE documents SET retry_count = 0, updated_at = ?1 WHERE document_id = ?2",
                params![format_dt(&Utc::now()), id],
            )
            .map_err(storage_err)?;

        if n == 0 {
            return Err(HarnessError::DocumentNotFound {
                document_id: id.to_string(),
            });
        }
        Ok(())
    }

    /// Recover processing documents only when the ingest lock is not currently held.
    pub fn recover_processing_documents_if_lock_free(
        &self,
        lock_name: &str,
        error: &ErrorInfo,
    ) -> Result<u32, HarnessError> {
        let n = self
            .conn
            .execute(
                "UPDATE documents
                 SET status = 'failed',
                     error_code = ?1,
                     error_message = ?2,
                     updated_at = ?3
                 WHERE status = 'processing'
                   AND NOT EXISTS (
                       SELECT 1 FROM durable_locks WHERE lock_name = ?4
                   )",
                params![error.code, error.message, format_dt(&Utc::now()), lock_name],
            )
            .map_err(storage_err)?;

        Ok(n as u32)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_doc(id: &str) -> Document {
        Document {
            document_id: id.to_string(),
            display_name: format!("{id}.txt"),
            file_path: PathBuf::from(format!("/docs/{id}.txt")),
            file_type: FileType::Txt,
            file_size_bytes: 1024,
            status: DocumentStatus::Pending,
            chunk_count: 0,
            retry_count: 0,
            max_retry_count: 3,
            next_retry_at: None,
            error: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_insert_and_get_document() {
        let db = Database::open_memory().unwrap();
        let doc = make_doc("doc-1");
        db.insert_document(&doc).unwrap();

        let fetched = db.get_document("doc-1").unwrap().expect("document missing");
        assert_eq!(fetched.document_id, "doc-1");
        assert_eq!(fetched.display_name, "doc-1.txt");
        assert_eq!(fetched.file_type, FileType::Txt);
        assert_eq!(fetched.status, DocumentStatus::Pending);
        assert_eq!(fetched.file_size_bytes, 1024);
        assert_eq!(fetched.retry_count, 0);
        assert_eq!(fetched.max_retry_count, 3);
        assert!(fetched.error.is_none());
        assert!(fetched.next_retry_at.is_none());
    }

    #[test]
    fn test_get_document_not_found() {
        let db = Database::open_memory().unwrap();
        let result = db.get_document("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_list_documents() {
        let db = Database::open_memory().unwrap();
        db.insert_document(&make_doc("a")).unwrap();
        db.insert_document(&make_doc("b")).unwrap();
        db.insert_document(&make_doc("c")).unwrap();

        let docs = db.list_documents().unwrap();
        assert_eq!(docs.len(), 3);
    }

    #[test]
    fn test_list_processing_documents_filters_correctly() {
        let db = Database::open_memory().unwrap();
        db.insert_document(&make_doc("pending")).unwrap();

        let mut processing = make_doc("processing");
        processing.status = DocumentStatus::Processing;
        db.insert_document(&processing).unwrap();

        let mut failed = make_doc("failed");
        failed.status = DocumentStatus::Failed;
        db.insert_document(&failed).unwrap();

        let processing_docs = db.list_processing_documents().unwrap();
        assert_eq!(processing_docs.len(), 1);
        assert_eq!(processing_docs[0].document_id, "processing");
    }

    #[test]
    fn test_update_document_status() {
        let db = Database::open_memory().unwrap();
        db.insert_document(&make_doc("doc-1")).unwrap();

        db.update_document_status("doc-1", DocumentStatus::Processing, None)
            .unwrap();

        let doc = db.get_document("doc-1").unwrap().unwrap();
        assert_eq!(doc.status, DocumentStatus::Processing);
        assert!(doc.error.is_none());
    }

    #[test]
    fn test_update_document_status_with_error() {
        let db = Database::open_memory().unwrap();
        db.insert_document(&make_doc("doc-1")).unwrap();

        let error = ErrorInfo {
            code: "PARSE_FAILED".to_string(),
            message: "Corrupt PDF".to_string(),
        };
        db.update_document_status("doc-1", DocumentStatus::Failed, Some(error))
            .unwrap();

        let doc = db.get_document("doc-1").unwrap().unwrap();
        assert_eq!(doc.status, DocumentStatus::Failed);
        let err = doc.error.unwrap();
        assert_eq!(err.code, "PARSE_FAILED");
        assert_eq!(err.message, "Corrupt PDF");
    }

    #[test]
    fn test_update_document_status_not_found() {
        let db = Database::open_memory().unwrap();
        let result = db.update_document_status("nope", DocumentStatus::Ready, None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            HarnessError::DocumentNotFound { .. }
        ));
    }

    #[test]
    fn test_update_document_chunk_count() {
        let db = Database::open_memory().unwrap();
        db.insert_document(&make_doc("doc-1")).unwrap();

        db.update_document_chunk_count("doc-1", 42).unwrap();

        let doc = db.get_document("doc-1").unwrap().unwrap();
        assert_eq!(doc.chunk_count, 42);
    }

    #[test]
    fn test_increment_retry_count() {
        let db = Database::open_memory().unwrap();
        db.insert_document(&make_doc("doc-1")).unwrap();

        db.increment_retry_count("doc-1").unwrap();
        db.increment_retry_count("doc-1").unwrap();

        let doc = db.get_document("doc-1").unwrap().unwrap();
        assert_eq!(doc.retry_count, 2);
    }

    #[test]
    fn test_reset_retry_count() {
        let db = Database::open_memory().unwrap();
        db.insert_document(&make_doc("doc-1")).unwrap();

        db.increment_retry_count("doc-1").unwrap();
        db.increment_retry_count("doc-1").unwrap();
        db.reset_retry_count("doc-1").unwrap();

        let doc = db.get_document("doc-1").unwrap().unwrap();
        assert_eq!(doc.retry_count, 0);
    }

    #[test]
    fn test_insert_document_with_all_fields() {
        let db = Database::open_memory().unwrap();

        let next = Utc::now();
        let doc = Document {
            document_id: "full-doc".to_string(),
            display_name: "full.pdf".to_string(),
            file_path: PathBuf::from("/tmp/full.pdf"),
            file_type: FileType::Pdf,
            file_size_bytes: 9999,
            status: DocumentStatus::Failed,
            chunk_count: 10,
            retry_count: 2,
            max_retry_count: 5,
            next_retry_at: Some(next),
            error: Some(ErrorInfo {
                code: "E001".to_string(),
                message: "something broke".to_string(),
            }),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        db.insert_document(&doc).unwrap();

        let fetched = db.get_document("full-doc").unwrap().unwrap();
        assert_eq!(fetched.file_type, FileType::Pdf);
        assert_eq!(fetched.status, DocumentStatus::Failed);
        assert_eq!(fetched.file_size_bytes, 9999);
        assert_eq!(fetched.chunk_count, 10);
        assert_eq!(fetched.retry_count, 2);
        assert_eq!(fetched.max_retry_count, 5);
        assert!(fetched.next_retry_at.is_some());
        let err = fetched.error.unwrap();
        assert_eq!(err.code, "E001");
    }

    #[test]
    fn test_insert_duplicate_document_fails() {
        let db = Database::open_memory().unwrap();
        db.insert_document(&make_doc("dup")).unwrap();
        let result = db.insert_document(&make_doc("dup"));
        assert!(result.is_err());
    }

    #[test]
    fn test_recover_processing_documents_if_lock_free() {
        let db = Database::open_memory().unwrap();

        let mut processing = make_doc("processing");
        processing.status = DocumentStatus::Processing;
        db.insert_document(&processing).unwrap();

        let updated = db
            .recover_processing_documents_if_lock_free(
                "ingest_pipeline",
                &ErrorInfo {
                    code: "interrupted_processing_recovered".to_string(),
                    message: "Recovered".to_string(),
                },
            )
            .unwrap();
        assert_eq!(updated, 1);

        let doc = db.get_document("processing").unwrap().unwrap();
        assert_eq!(doc.status, DocumentStatus::Failed);
    }

    #[test]
    fn test_recover_processing_documents_skips_when_lock_exists() {
        let db = Database::open_memory().unwrap();

        let mut processing = make_doc("processing");
        processing.status = DocumentStatus::Processing;
        db.insert_document(&processing).unwrap();
        db.try_acquire_lock("ingest_pipeline", "owner-a").unwrap();

        let updated = db
            .recover_processing_documents_if_lock_free(
                "ingest_pipeline",
                &ErrorInfo {
                    code: "interrupted_processing_recovered".to_string(),
                    message: "Recovered".to_string(),
                },
            )
            .unwrap();
        assert_eq!(updated, 0);

        let doc = db.get_document("processing").unwrap().unwrap();
        assert_eq!(doc.status, DocumentStatus::Processing);
    }
}
