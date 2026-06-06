use common::types::{DocumentStatus, ErrorInfo};
use common::CiteError;
use storage::Database;
use uuid::Uuid;

/// Result of a successful refresh
#[derive(Debug, Clone)]
pub struct RefreshResult {
    pub snapshot_id: String,
    pub document_count: u32,
    pub previous_snapshot_id: Option<String>,
}

/// Execute a corpus refresh: build a new snapshot from all ready documents
/// and atomically promote it as active.
///
/// Steps:
/// 1. Create a new building snapshot
/// 2. Attach all ready documents to it
/// 3. Activate the snapshot atomically (supersedes previous)
///
/// On failure at any step, the previous active snapshot remains intact.
pub fn refresh_corpus(db: &Database) -> Result<RefreshResult, CiteError> {
    let snapshot_id = format!(
        "snap_{}",
        &Uuid::new_v4().to_string().replace('-', "")[..12]
    );

    // 1. Create building snapshot
    db.begin_snapshot_build(&snapshot_id)?;

    // 2. Collect all ready documents and attach to snapshot
    let ready_docs = db.list_documents_by_status(DocumentStatus::Ready)?;

    if ready_docs.is_empty() {
        // Mark as failed — nothing to refresh
        db.mark_snapshot_failed(
            &snapshot_id,
            &ErrorInfo {
                code: "empty_corpus".to_string(),
                message: "No ready documents found to include in snapshot".to_string(),
            },
        )?;
        return Err(CiteError::InvalidParameter {
            message: "No ready documents found to include in snapshot".to_string(),
        });
    }

    for doc in &ready_docs {
        db.attach_document_to_snapshot(&snapshot_id, &doc.document_id)?;
    }

    // 3. Activate atomically — if this fails, previous snapshot stays active
    let activate_result = db.activate_snapshot(&snapshot_id)?;

    Ok(RefreshResult {
        snapshot_id,
        document_count: u32::try_from(ready_docs.len()).map_err(|e| CiteError::StorageError {
            message: format!("document_count overflow: {e}"),
        })?,
        previous_snapshot_id: activate_result.previous_snapshot_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use common::types::{Document, FileType};
    use common::DocumentId;
    use std::path::PathBuf;

    fn make_doc(id: &str, status: DocumentStatus) -> Document {
        Document {
            document_id: DocumentId::from(id),
            display_name: format!("{id}.txt"),
            file_path: PathBuf::from(format!("/docs/{id}.txt")),
            file_type: FileType::Txt,
            file_size_bytes: 100,
            status,
            chunk_count: 5,
            retry_count: 0,
            max_retry_count: 3,
            next_retry_at: None,
            error: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_refresh_corpus_creates_active_snapshot_from_ready_docs() {
        let db = Database::open_memory().unwrap();

        db.insert_document(&make_doc("doc-ready-1", DocumentStatus::Ready))
            .unwrap();
        db.insert_document(&make_doc("doc-ready-2", DocumentStatus::Ready))
            .unwrap();
        db.insert_document(&make_doc("doc-pending", DocumentStatus::Pending))
            .unwrap();
        db.insert_document(&make_doc("doc-failed", DocumentStatus::Failed))
            .unwrap();

        let result = refresh_corpus(&db).unwrap();

        assert_eq!(result.document_count, 2);
        assert!(result.previous_snapshot_id.is_none());

        let active = db.get_active_snapshot_id().unwrap();
        assert_eq!(active.as_deref(), Some(result.snapshot_id.as_str()));

        let members = db.get_active_snapshot_member_ids().unwrap().unwrap();
        assert_eq!(members.len(), 2);
        assert!(members.contains(&"doc-ready-1".to_string()));
        assert!(members.contains(&"doc-ready-2".to_string()));
    }

    #[test]
    fn test_refresh_corpus_with_no_ready_docs_fails() {
        let db = Database::open_memory().unwrap();

        db.insert_document(&make_doc("doc-pending", DocumentStatus::Pending))
            .unwrap();

        let result = refresh_corpus(&db);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CiteError::InvalidParameter { .. }
        ));

        // No active snapshot should exist
        assert!(db.get_active_snapshot_id().unwrap().is_none());
    }

    #[test]
    fn test_refresh_corpus_supersedes_previous_snapshot() {
        let db = Database::open_memory().unwrap();

        // First refresh
        db.insert_document(&make_doc("doc-v1", DocumentStatus::Ready))
            .unwrap();
        let first = refresh_corpus(&db).unwrap();
        assert!(first.previous_snapshot_id.is_none());

        // Second refresh with new document
        db.insert_document(&make_doc("doc-v2", DocumentStatus::Ready))
            .unwrap();
        let second = refresh_corpus(&db).unwrap();

        assert_eq!(
            second.previous_snapshot_id.as_deref(),
            Some(first.snapshot_id.as_str())
        );
        assert_eq!(second.document_count, 2);
    }

    #[test]
    fn test_refresh_corpus_failure_leaves_previous_active_intact() {
        let db = Database::open_memory().unwrap();

        // First successful refresh
        db.insert_document(&make_doc("doc-v1", DocumentStatus::Ready))
            .unwrap();
        let first = refresh_corpus(&db).unwrap();

        // Manually mark all docs as non-ready so second refresh fails
        db.update_document_status("doc-v1", DocumentStatus::Pending, None)
            .unwrap();

        let result = refresh_corpus(&db);
        assert!(result.is_err());

        // Previous snapshot should still be active
        let active = db.get_active_snapshot_id().unwrap();
        assert_eq!(active.as_deref(), Some(first.snapshot_id.as_str()));
    }
}
