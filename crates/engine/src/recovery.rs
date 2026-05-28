use common::types::ErrorInfo;
use common::CiteError;
use storage::Database;

pub const RECOVERY_ERROR_CODE: &str = "interrupted_processing_recovered";
pub const RECOVERY_ERROR_MESSAGE: &str =
    "Document was in processing state during startup recovery and was moved to failed";
const INGEST_LOCK_NAME: &str = "ingest_pipeline";

/// Recover interrupted processing documents.
///
/// Policy: every document currently in `processing` is deterministically moved to
/// `failed` with a stable recovery error code/message.
pub fn recover_interrupted_processing(db: &Database) -> Result<u32, CiteError> {
    db.recover_processing_documents_if_lock_free(
        INGEST_LOCK_NAME,
        &ErrorInfo {
            code: RECOVERY_ERROR_CODE.to_string(),
            message: RECOVERY_ERROR_MESSAGE.to_string(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use common::types::{Document, DocumentStatus, FileType};
    use std::path::PathBuf;

    fn make_doc(id: &str, status: DocumentStatus) -> Document {
        Document {
            document_id: id.to_string(),
            display_name: format!("{id}.txt"),
            file_path: PathBuf::from(format!("/docs/{id}.txt")),
            file_type: FileType::Txt,
            file_size_bytes: 10,
            status,
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
    fn test_recovery_moves_processing_to_failed() {
        let db = Database::open_memory().unwrap();
        db.insert_document(&make_doc("doc-processing", DocumentStatus::Processing))
            .unwrap();
        db.insert_document(&make_doc("doc-ready", DocumentStatus::Ready))
            .unwrap();

        let recovered = recover_interrupted_processing(&db).unwrap();
        assert_eq!(recovered, 1);

        let processing = db.get_document("doc-processing").unwrap().unwrap();
        assert_eq!(processing.status, DocumentStatus::Failed);
        let error = processing.error.expect("expected recovery error");
        assert_eq!(error.code, RECOVERY_ERROR_CODE);

        let ready = db.get_document("doc-ready").unwrap().unwrap();
        assert_eq!(ready.status, DocumentStatus::Ready);
    }

    #[test]
    fn test_recovery_is_idempotent() {
        let db = Database::open_memory().unwrap();
        db.insert_document(&make_doc("doc-processing", DocumentStatus::Processing))
            .unwrap();

        let first = recover_interrupted_processing(&db).unwrap();
        let second = recover_interrupted_processing(&db).unwrap();

        assert_eq!(first, 1);
        assert_eq!(second, 0);
    }

    #[test]
    fn test_recovery_skips_when_ingest_lock_is_held() {
        let db = Database::open_memory().unwrap();
        db.insert_document(&make_doc("doc-processing", DocumentStatus::Processing))
            .unwrap();
        db.try_acquire_lock("ingest_pipeline", "owner-a").unwrap();

        let recovered = recover_interrupted_processing(&db).unwrap();
        assert_eq!(recovered, 0);

        let doc = db.get_document("doc-processing").unwrap().unwrap();
        assert_eq!(doc.status, DocumentStatus::Processing);
    }
}
