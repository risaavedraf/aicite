//! Document CRUD operations for the SQLite-backed document store.
//!
//! Provides insert, query, update, and recovery methods for [`Document`]
//! records. All methods operate on the [`Database`] handle
//! and return [`Result<T, CiteError>`].

use std::path::PathBuf;

use chrono::Utc;
use common::types::{Document, DocumentStatus, ErrorInfo, FileType};
use common::CiteError;
use rusqlite::{params, Row};

use crate::util::{format_dt, i64_to_u32, parse_dt, storage_err, usize_to_u32};
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

fn parse_status(s: &str) -> Result<DocumentStatus, CiteError> {
    match s {
        "pending" => Ok(DocumentStatus::Pending),
        "processing" => Ok(DocumentStatus::Processing),
        "ready" => Ok(DocumentStatus::Ready),
        "failed" => Ok(DocumentStatus::Failed),
        other => Err(CiteError::StorageError {
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

fn parse_file_type(s: &str) -> Result<FileType, CiteError> {
    match s {
        "pdf" => Ok(FileType::Pdf),
        "txt" => Ok(FileType::Txt),
        "md" => Ok(FileType::Md),
        other => Err(CiteError::StorageError {
            message: format!("Unknown file type: {other}"),
        }),
    }
}

// ---------------------------------------------------------------------------
// Row -> Document conversion
// ---------------------------------------------------------------------------

fn row_to_document(row: &Row<'_>) -> Result<Document, CiteError> {
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
    let source_hash: Option<String> = row.get("source_hash").map_err(storage_err)?;
    let ingested_at_str: Option<String> = row.get("ingested_at").map_err(storage_err)?;
    let file_modified_at_str: Option<String> = row.get("file_modified_at").map_err(storage_err)?;

    let next_retry_at = match next_retry_at_str {
        Some(s) => Some(parse_dt(&s)?),
        None => None,
    };
    let ingested_at = match ingested_at_str {
        Some(s) => Some(parse_dt(&s)?),
        None => None,
    };
    let file_modified_at = match file_modified_at_str {
        Some(s) => Some(parse_dt(&s)?),
        None => None,
    };

    let error = match (error_code, error_message) {
        (Some(code), Some(message)) => Some(ErrorInfo { code, message }),
        _ => None,
    };

    Ok(Document {
        document_id: document_id.into(),
        display_name,
        file_path: PathBuf::from(file_path_str),
        file_type: parse_file_type(&file_type_str)?,
        file_size_bytes: file_size_bytes as u64,
        status: parse_status(&status_str)?,
        chunk_count: i64_to_u32("chunk_count", chunk_count)?,
        retry_count: i64_to_u32("retry_count", retry_count)?,
        max_retry_count: i64_to_u32("max_retry_count", max_retry_count)?,
        next_retry_at,
        error,
        created_at: parse_dt(&created_at_str)?,
        updated_at: parse_dt(&updated_at_str)?,
        source_hash,
        ingested_at,
        file_modified_at,
    })
}

// ---------------------------------------------------------------------------
// CRUD operations
// ---------------------------------------------------------------------------

impl Database {
    /// Insert a new document into the store.
    ///
    /// Fails with a SQLite constraint error if a document with the same
    /// `document_id` already exists.
    ///
    /// # Arguments
    ///
    /// * `doc` - The document metadata to persist.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or [`CiteError::StorageError`] on failure.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Requires an open Database instance
    /// use common::types::{Document, DocumentStatus, FileType};
    /// use chrono::Utc;
    /// use std::path::PathBuf;
    ///
    /// let doc = Document {
    ///     document_id: "doc-1".to_string(),
    ///     display_name: "test.txt".to_string(),
    ///     file_path: PathBuf::from("/test.txt"),
    ///     file_type: FileType::Txt,
    ///     file_size_bytes: 100,
    ///     status: DocumentStatus::Pending,
    ///     chunk_count: 0,
    ///     retry_count: 0,
    ///     max_retry_count: 3,
    ///     next_retry_at: None,
    ///     error: None,
    ///     created_at: Utc::now(),
    ///     updated_at: Utc::now(),
    ///     source_hash: None,
    ///     ingested_at: None,
    ///     file_modified_at: None,
    /// };
    /// db.insert_document(&doc).unwrap();
    /// ```
    pub fn insert_document(&self, doc: &Document) -> Result<(), CiteError> {
        self.conn
            .execute(
                "INSERT INTO documents (
                    document_id, display_name, file_path, file_type, file_size_bytes,
                    status, chunk_count, retry_count, max_retry_count,
                    next_retry_at, error_code, error_message,
                    created_at, updated_at, source_hash, ingested_at, file_modified_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
                params![
                    doc.document_id.as_ref(),
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
                    doc.source_hash.as_deref(),
                    doc.ingested_at.as_ref().map(format_dt),
                    doc.file_modified_at.as_ref().map(format_dt),
                ],
            )
            .map_err(storage_err)?;
        Ok(())
    }

    /// Retrieve a single document by its identifier.
    ///
    /// # Arguments
    ///
    /// * `id` - The document identifier to look up.
    ///
    /// # Returns
    ///
    /// `Ok(Some(doc))` if found, `Ok(None)` if no document matches, or
    /// [`CiteError::StorageError`] on database failure.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let doc = db.get_document("doc-1").unwrap();
    /// assert!(doc.is_some());
    /// ```
    pub fn get_document(&self, id: &str) -> Result<Option<Document>, CiteError> {
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
    ///
    /// # Returns
    ///
    /// `Ok(docs)` with all documents, or [`CiteError::StorageError`] on
    /// database failure. The returned `Vec` may be empty.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let all = db.list_documents().unwrap();
    /// println!("{} documents stored", all.len());
    /// ```
    pub fn list_documents(&self) -> Result<Vec<Document>, CiteError> {
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

    /// List documents filtered by their pipeline status.
    ///
    /// Results are ordered by creation time ascending (oldest first),
    /// which is useful for FIFO processing of pending work.
    ///
    /// # Arguments
    ///
    /// * `status` - The [`DocumentStatus`] to filter on.
    ///
    /// # Returns
    ///
    /// `Ok(docs)` matching the status, or [`CiteError::StorageError`].
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use common::types::DocumentStatus;
    ///
    /// let pending = db.list_documents_by_status(DocumentStatus::Pending).unwrap();
    /// ```
    pub fn list_documents_by_status(
        &self,
        status: DocumentStatus,
    ) -> Result<Vec<Document>, CiteError> {
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

    /// List all documents currently marked as `Processing`.
    ///
    /// Convenience shorthand equivalent to
    /// `list_documents_by_status(DocumentStatus::Processing)`.
    ///
    /// # Returns
    ///
    /// `Ok(docs)` with in-flight documents, or [`CiteError::StorageError`].
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let in_flight = db.list_processing_documents().unwrap();
    /// assert!(in_flight.iter().all(|d| d.status == DocumentStatus::Processing));
    /// ```
    pub fn list_processing_documents(&self) -> Result<Vec<Document>, CiteError> {
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

    /// Update the status and optional error information of a document.
    ///
    /// Also refreshes the `updated_at` timestamp to the current time.
    ///
    /// # Arguments
    ///
    /// * `id` - Document identifier to update.
    /// * `status` - New [`DocumentStatus`].
    /// * `error` - Optional [`ErrorInfo`] to persist alongside a `Failed` status.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or [`CiteError::DocumentNotFound`] if the
    /// document does not exist.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use common::types::{DocumentStatus, ErrorInfo};
    ///
    /// db.update_document_status("doc-1", DocumentStatus::Ready, None).unwrap();
    ///
    /// db.update_document_status(
    ///     "doc-2",
    ///     DocumentStatus::Failed,
    ///     Some(ErrorInfo {
    ///         code: "PARSE_FAILED".to_string(),
    ///         message: "Corrupt PDF".to_string(),
    ///     }),
    /// ).unwrap();
    /// ```
    pub fn update_document_status(
        &self,
        id: &str,
        status: DocumentStatus,
        error: Option<ErrorInfo>,
    ) -> Result<(), CiteError> {
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
            return Err(CiteError::DocumentNotFound {
                document_id: id.to_string(),
            });
        }
        Ok(())
    }

    /// Set the chunk count on a document after ingestion.
    ///
    /// Also refreshes the `updated_at` timestamp.
    ///
    /// # Arguments
    ///
    /// * `id` - Document identifier.
    /// * `count` - Number of chunks produced during ingestion.
    ///
    /// # Returns
    ///
    /// `Ok(())`, or [`CiteError::DocumentNotFound`] if the ID is unknown.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// db.update_document_chunk_count("doc-1", 42).unwrap();
    /// ```
    pub fn update_document_chunk_count(&self, id: &str, count: u32) -> Result<(), CiteError> {
        let n = self
            .conn
            .execute(
                "UPDATE documents SET chunk_count = ?1, updated_at = ?2 WHERE document_id = ?3",
                params![count as i64, format_dt(&Utc::now()), id],
            )
            .map_err(storage_err)?;

        if n == 0 {
            return Err(CiteError::DocumentNotFound {
                document_id: id.to_string(),
            });
        }
        Ok(())
    }

    /// Increment the retry counter on a document by one.
    ///
    /// Called when a transient failure triggers a retry. Also refreshes
    /// the `updated_at` timestamp.
    ///
    /// # Arguments
    ///
    /// * `id` - Document identifier.
    ///
    /// # Returns
    ///
    /// `Ok(())`, or [`CiteError::DocumentNotFound`] if the ID is unknown.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// db.increment_retry_count("doc-1").unwrap();
    /// db.increment_retry_count("doc-1").unwrap();
    /// let doc = db.get_document("doc-1").unwrap().unwrap();
    /// assert_eq!(doc.retry_count, 2);
    /// ```
    pub fn increment_retry_count(&self, id: &str) -> Result<(), CiteError> {
        let n = self
            .conn
            .execute(
                "UPDATE documents SET retry_count = retry_count + 1, updated_at = ?1 WHERE document_id = ?2",
                params![format_dt(&Utc::now()), id],
            )
            .map_err(storage_err)?;

        if n == 0 {
            return Err(CiteError::DocumentNotFound {
                document_id: id.to_string(),
            });
        }
        Ok(())
    }

    /// Reset the retry counter on a document back to zero.
    ///
    /// Typically called after a successful processing run. Also refreshes
    /// the `updated_at` timestamp.
    ///
    /// # Arguments
    ///
    /// * `id` - Document identifier.
    ///
    /// # Returns
    ///
    /// `Ok(())`, or [`CiteError::DocumentNotFound`] if the ID is unknown.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// db.reset_retry_count("doc-1").unwrap();
    /// ```
    pub fn reset_retry_count(&self, id: &str) -> Result<(), CiteError> {
        let n = self
            .conn
            .execute(
                "UPDATE documents SET retry_count = 0, updated_at = ?1 WHERE document_id = ?2",
                params![format_dt(&Utc::now()), id],
            )
            .map_err(storage_err)?;

        if n == 0 {
            return Err(CiteError::DocumentNotFound {
                document_id: id.to_string(),
            });
        }
        Ok(())
    }

    /// Recover orphaned "processing" documents when no active ingest lock
    /// is held.
    ///
    /// If the pipeline crashed mid-processing, some documents may be stuck
    /// in `Processing` status. This method marks them as `Failed` with the
    /// provided error info **only** when the named lock is not currently
    /// acquired — preventing interference with a legitimately running ingest.
    ///
    /// # Arguments
    ///
    /// * `lock_name` - The durable lock to check (e.g. `"ingest_pipeline"`).
    /// * `error` - [`ErrorInfo`] to record on recovered documents.
    ///
    /// # Returns
    ///
    /// The number of documents recovered, or [`CiteError::StorageError`].
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use common::types::ErrorInfo;
    ///
    /// let recovered = db.recover_processing_documents_if_lock_free(
    ///     "ingest_pipeline",
    ///     &ErrorInfo {
    ///         code: "interrupted_processing_recovered".to_string(),
    ///         message: "Recovered".to_string(),
    ///     },
    /// ).unwrap();
    /// println!("Recovered {} documents", recovered);
    /// ```
    pub fn recover_processing_documents_if_lock_free(
        &self,
        lock_name: &str,
        error: &ErrorInfo,
    ) -> Result<u32, CiteError> {
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

        usize_to_u32("document_count", n)
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
            document_id: id.to_string().into(),
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
            source_hash: None,
            ingested_at: None,
            file_modified_at: None,
        }
    }

    #[test]
    fn test_insert_and_get_document() {
        let db = Database::open_memory().unwrap();
        let doc = make_doc("doc-1");
        db.insert_document(&doc).unwrap();

        let fetched = db.get_document("doc-1").unwrap().expect("document missing");
        assert_eq!(fetched.document_id, "doc-1".into());
        assert_eq!(fetched.display_name, "doc-1.txt");
        assert_eq!(fetched.file_type, FileType::Txt);
        assert_eq!(fetched.status, DocumentStatus::Pending);
        assert_eq!(fetched.file_size_bytes, 1024);
        assert_eq!(fetched.retry_count, 0);
        assert_eq!(fetched.max_retry_count, 3);
        assert!(fetched.error.is_none());
        assert!(fetched.next_retry_at.is_none());
        assert!(fetched.source_hash.is_none());
        assert!(fetched.ingested_at.is_none());
        assert!(fetched.file_modified_at.is_none());
    }

    #[test]
    fn test_insert_and_get_document_lifecycle_fields() {
        let db = Database::open_memory().unwrap();
        let mut doc = make_doc("doc-life");
        let ingested_at = Utc::now();
        let file_modified_at = Utc::now();
        doc.source_hash = Some("sha256:abc".to_string());
        doc.ingested_at = Some(ingested_at);
        doc.file_modified_at = Some(file_modified_at);

        db.insert_document(&doc).unwrap();

        let fetched = db
            .get_document("doc-life")
            .unwrap()
            .expect("document missing");
        assert_eq!(fetched.source_hash.as_deref(), Some("sha256:abc"));
        assert!(fetched.ingested_at.is_some());
        assert!(fetched.file_modified_at.is_some());
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
        assert_eq!(processing_docs[0].document_id, "processing".into());
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
            CiteError::DocumentNotFound { .. }
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
            document_id: "full-doc".to_string().into(),
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
            source_hash: None,
            ingested_at: None,
            file_modified_at: None,
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

    // -----------------------------------------------------------------------
    // list_documents_by_status
    // -----------------------------------------------------------------------

    #[test]
    fn test_list_documents_by_status_filters_correctly() {
        let db = Database::open_memory().unwrap();

        let mut ready = make_doc("doc-ready");
        ready.status = DocumentStatus::Ready;
        db.insert_document(&ready).unwrap();

        let mut failed = make_doc("doc-failed");
        failed.status = DocumentStatus::Failed;
        db.insert_document(&failed).unwrap();

        db.insert_document(&make_doc("doc-pending")).unwrap();

        let ready_docs = db.list_documents_by_status(DocumentStatus::Ready).unwrap();
        assert_eq!(ready_docs.len(), 1);
        assert_eq!(ready_docs[0].document_id, "doc-ready".into());

        let failed_docs = db.list_documents_by_status(DocumentStatus::Failed).unwrap();
        assert_eq!(failed_docs.len(), 1);
        assert_eq!(failed_docs[0].document_id, "doc-failed".into());

        let pending_docs = db
            .list_documents_by_status(DocumentStatus::Pending)
            .unwrap();
        assert_eq!(pending_docs.len(), 1);
        assert_eq!(pending_docs[0].document_id, "doc-pending".into());

        let processing_docs = db
            .list_documents_by_status(DocumentStatus::Processing)
            .unwrap();
        assert!(processing_docs.is_empty());
    }

    // -----------------------------------------------------------------------
    // Empty string ID inputs
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_document_empty_id() {
        let db = Database::open_memory().unwrap();
        let result = db.get_document("");
        // Should return None (not found), not panic
        assert!(result.unwrap().is_none());
    }

    // -----------------------------------------------------------------------
    // retry boundary: retry_count >= max_retry_count
    // -----------------------------------------------------------------------

    #[test]
    fn test_increment_retry_count_beyond_max() {
        let db = Database::open_memory().unwrap();
        let mut doc = make_doc("doc-retry");
        doc.max_retry_count = 3;
        db.insert_document(&doc).unwrap();

        // Increment up to and beyond max_retry_count
        db.increment_retry_count("doc-retry").unwrap();
        db.increment_retry_count("doc-retry").unwrap();
        db.increment_retry_count("doc-retry").unwrap();
        db.increment_retry_count("doc-retry").unwrap();

        let fetched = db.get_document("doc-retry").unwrap().unwrap();
        // increment_retry_count does NOT enforce max — it just increments
        // This test documents that boundary behavior
        assert_eq!(fetched.retry_count, 4);
        assert_eq!(fetched.max_retry_count, 3);
    }

    #[test]
    fn test_increment_retry_count_not_found() {
        let db = Database::open_memory().unwrap();
        let result = db.increment_retry_count("nonexistent");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CiteError::DocumentNotFound { .. }
        ));
    }
}
