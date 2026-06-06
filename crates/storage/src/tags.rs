//! Tag persistence and validation helpers.
//!
//! Tags are flat `key:value` metadata rows scoped to either a document or a
//! chunk. This module intentionally contains storage-level behavior only; CLI
//! command wiring and retrieval filters are later SDD slices.

use common::CiteError;
use rusqlite::params;

use crate::util::storage_err;
use crate::Database;

const RESERVED_KEYS: &[&str] = &["workspace", "type", "session", "source_kind"];

/// Entity kind that owns a local tag row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagEntityType {
    Document,
    Chunk,
}

impl TagEntityType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::Chunk => "chunk",
        }
    }
}

/// A concrete tag mutation or result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagRecord {
    pub key: String,
    pub value: String,
}

impl TagRecord {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Result<Self, CiteError> {
        let tag = Self {
            key: key.into(),
            value: value.into(),
        };
        validate_tag(&tag)?;
        Ok(tag)
    }

    /// Parse a mutation tag. Mutations require explicit `key:value` input.
    pub fn parse_mutation(input: &str) -> Result<Self, CiteError> {
        let (key, value) = input
            .split_once(':')
            .ok_or_else(|| CiteError::InvalidParameter {
                message: "Tag mutations require key:value syntax".to_string(),
            })?;
        Self::new(key, value)
    }
}

/// A tag filter. Key-only filters are allowed here, but not for mutations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagFilter {
    pub key: String,
    pub value: Option<String>,
}

impl TagFilter {
    pub fn parse(input: &str) -> Result<Self, CiteError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(invalid_tag("Tag filter key cannot be empty"));
        }

        match trimmed.split_once(':') {
            Some((key, value)) => {
                let tag = TagRecord::new(key, value)?;
                Ok(Self {
                    key: tag.key,
                    value: Some(tag.value),
                })
            }
            None => {
                validate_component("key", trimmed)?;
                Ok(Self {
                    key: trimmed.to_string(),
                    value: None,
                })
            }
        }
    }
}

impl Database {
    pub fn set_tag_user(
        &self,
        entity_type: TagEntityType,
        entity_id: &str,
        tag: &TagRecord,
    ) -> Result<(), CiteError> {
        validate_user_tag(entity_type, tag)?;
        self.insert_tag(entity_type, entity_id, tag)
    }

    pub fn set_tag_engine(
        &self,
        entity_type: TagEntityType,
        entity_id: &str,
        tag: &TagRecord,
    ) -> Result<(), CiteError> {
        validate_engine_tag(entity_type, tag)?;
        self.insert_tag(entity_type, entity_id, tag)
    }

    pub fn remove_tag_user(
        &self,
        entity_type: TagEntityType,
        entity_id: &str,
        tag: &TagRecord,
    ) -> Result<u64, CiteError> {
        validate_user_tag(entity_type, tag)?;
        self.delete_tag(entity_type, entity_id, tag)
    }

    pub fn remove_tag_engine(
        &self,
        entity_type: TagEntityType,
        entity_id: &str,
        tag: &TagRecord,
    ) -> Result<u64, CiteError> {
        validate_engine_tag(entity_type, tag)?;
        self.delete_tag(entity_type, entity_id, tag)
    }

    pub fn list_tags(
        &self,
        entity_type: TagEntityType,
        entity_id: &str,
    ) -> Result<Vec<TagRecord>, CiteError> {
        let mut stmt = self
            .conn()
            .prepare(
                "SELECT key, value FROM tags
                 WHERE entity_type = ?1 AND entity_id = ?2
                 ORDER BY key, value",
            )
            .map_err(storage_err)?;

        let rows = stmt
            .query_map(params![entity_type.as_str(), entity_id], |row| {
                Ok(TagRecord {
                    key: row.get(0)?,
                    value: row.get(1)?,
                })
            })
            .map_err(storage_err)?;

        rows.collect::<Result<Vec<_>, _>>().map_err(storage_err)
    }

    pub fn clear_chunk_status_changed_for_document(
        &self,
        document_id: &str,
    ) -> Result<u64, CiteError> {
        let deleted = self
            .conn()
            .execute(
                "DELETE FROM tags
                 WHERE entity_type = 'chunk'
                   AND key = 'status'
                   AND value = 'changed'
                   AND entity_id IN (
                       SELECT chunk_id FROM chunks WHERE document_id = ?1
                   )",
                params![document_id],
            )
            .map_err(storage_err)?;
        Ok(deleted as u64)
    }

    fn insert_tag(
        &self,
        entity_type: TagEntityType,
        entity_id: &str,
        tag: &TagRecord,
    ) -> Result<(), CiteError> {
        validate_entity_id(entity_id)?;
        self.conn()
            .execute(
                "INSERT OR IGNORE INTO tags (tag_id, entity_type, entity_id, key, value)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    tag_id(entity_type, entity_id, tag),
                    entity_type.as_str(),
                    entity_id,
                    tag.key,
                    tag.value,
                ],
            )
            .map_err(storage_err)?;
        Ok(())
    }

    fn delete_tag(
        &self,
        entity_type: TagEntityType,
        entity_id: &str,
        tag: &TagRecord,
    ) -> Result<u64, CiteError> {
        validate_entity_id(entity_id)?;
        let deleted = self
            .conn()
            .execute(
                "DELETE FROM tags
                 WHERE entity_type = ?1 AND entity_id = ?2 AND key = ?3 AND value = ?4",
                params![entity_type.as_str(), entity_id, tag.key, tag.value],
            )
            .map_err(storage_err)?;
        Ok(deleted as u64)
    }
}

fn validate_user_tag(entity_type: TagEntityType, tag: &TagRecord) -> Result<(), CiteError> {
    validate_tag(tag)?;
    reject_document_changed(entity_type, tag)?;
    if RESERVED_KEYS.contains(&tag.key.as_str()) {
        return Err(invalid_tag(format!(
            "Tag key '{}' is reserved for engine-managed metadata",
            tag.key
        )));
    }
    Ok(())
}

fn validate_engine_tag(entity_type: TagEntityType, tag: &TagRecord) -> Result<(), CiteError> {
    validate_tag(tag)?;
    reject_document_changed(entity_type, tag)
}

fn reject_document_changed(entity_type: TagEntityType, tag: &TagRecord) -> Result<(), CiteError> {
    if entity_type == TagEntityType::Document && tag.key == "status" && tag.value == "changed" {
        return Err(invalid_tag(
            "status:changed is chunk-only and cannot be stored on documents",
        ));
    }
    Ok(())
}

fn validate_tag(tag: &TagRecord) -> Result<(), CiteError> {
    validate_component("key", &tag.key)?;
    validate_component("value", &tag.value)
}

fn validate_component(label: &str, value: &str) -> Result<(), CiteError> {
    if value.trim().is_empty() {
        return Err(invalid_tag(format!("Tag {label} cannot be empty")));
    }
    if value.trim() != value {
        return Err(invalid_tag(format!(
            "Tag {label} cannot contain leading or trailing whitespace"
        )));
    }
    Ok(())
}

fn validate_entity_id(entity_id: &str) -> Result<(), CiteError> {
    if entity_id.trim().is_empty() {
        return Err(invalid_tag("Tag entity id cannot be empty"));
    }
    Ok(())
}

fn invalid_tag(message: impl Into<String>) -> CiteError {
    CiteError::InvalidParameter {
        message: message.into(),
    }
}

fn tag_id(entity_type: TagEntityType, entity_id: &str, tag: &TagRecord) -> String {
    format!(
        "tag:{}:{}:{}:{}",
        entity_type.as_str(),
        entity_id,
        tag.key,
        tag.value
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use common::types::{Chunk, Document, DocumentStatus, FileType};
    use std::path::PathBuf;

    fn make_doc(id: &str) -> Document {
        Document {
            document_id: id.into(),
            display_name: format!("{id}.md"),
            file_path: PathBuf::from(format!("/docs/{id}.md")),
            file_type: FileType::Md,
            file_size_bytes: 10,
            status: DocumentStatus::Ready,
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

    fn make_chunk(id: &str, document_id: &str) -> Chunk {
        Chunk {
            chunk_id: id.into(),
            document_id: document_id.into(),
            section_id: None,
            chunk_index: 0,
            text: "chunk text".to_string(),
            page: None,
            offset_start: None,
            offset_end: None,
            created_at: Utc::now(),
        }
    }

    fn tag(input: &str) -> TagRecord {
        TagRecord::parse_mutation(input).unwrap()
    }

    #[test]
    fn duplicate_tag_set_is_idempotent() {
        let db = Database::open_memory().unwrap();
        db.set_tag_engine(TagEntityType::Chunk, "chunk-1", &tag("tag:auth"))
            .unwrap();
        db.set_tag_engine(TagEntityType::Chunk, "chunk-1", &tag("tag:auth"))
            .unwrap();

        let tags = db.list_tags(TagEntityType::Chunk, "chunk-1").unwrap();
        assert_eq!(tags, vec![tag("tag:auth")]);
    }

    #[test]
    fn entity_type_scopes_same_entity_id() {
        let db = Database::open_memory().unwrap();
        db.set_tag_engine(TagEntityType::Document, "same", &tag("tag:doc"))
            .unwrap();
        db.set_tag_engine(TagEntityType::Chunk, "same", &tag("tag:chunk"))
            .unwrap();

        assert_eq!(
            db.list_tags(TagEntityType::Document, "same").unwrap(),
            vec![tag("tag:doc")]
        );
        assert_eq!(
            db.list_tags(TagEntityType::Chunk, "same").unwrap(),
            vec![tag("tag:chunk")]
        );
    }

    #[test]
    fn user_rejects_reserved_keys_but_engine_accepts_them() {
        let db = Database::open_memory().unwrap();
        let workspace = tag("workspace:aiharness");

        let user_result = db.set_tag_user(TagEntityType::Document, "doc-1", &workspace);
        assert!(matches!(
            user_result,
            Err(CiteError::InvalidParameter { .. })
        ));

        db.set_tag_engine(TagEntityType::Document, "doc-1", &workspace)
            .unwrap();
        assert_eq!(
            db.list_tags(TagEntityType::Document, "doc-1").unwrap(),
            vec![workspace]
        );
    }

    #[test]
    fn malformed_mutation_tags_are_rejected() {
        assert!(TagRecord::parse_mutation("status").is_err());
        assert!(TagRecord::parse_mutation("status:").is_err());
        assert!(TagRecord::parse_mutation(":changed").is_err());
        assert!(TagRecord::parse_mutation(" status:changed").is_err());
    }

    #[test]
    fn key_only_filters_are_allowed_only_for_filters() {
        assert_eq!(
            TagFilter::parse("status").unwrap(),
            TagFilter {
                key: "status".to_string(),
                value: None,
            }
        );
        assert!(TagRecord::parse_mutation("status").is_err());
    }

    #[test]
    fn document_status_changed_is_rejected_for_user_and_engine() {
        let db = Database::open_memory().unwrap();
        let changed = tag("status:changed");

        assert!(matches!(
            db.set_tag_user(TagEntityType::Document, "doc-1", &changed),
            Err(CiteError::InvalidParameter { .. })
        ));
        assert!(matches!(
            db.set_tag_engine(TagEntityType::Document, "doc-1", &changed),
            Err(CiteError::InvalidParameter { .. })
        ));
        assert!(db
            .list_tags(TagEntityType::Document, "doc-1")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn chunk_status_changed_is_allowed_and_clearable_by_document() {
        let db = Database::open_memory().unwrap();
        let doc = make_doc("doc-1");
        db.insert_document(&doc).unwrap();
        db.insert_chunks(
            "doc-1",
            &[
                make_chunk("chunk-1", "doc-1"),
                make_chunk("chunk-2", "doc-1"),
            ],
        )
        .unwrap();

        db.set_tag_engine(TagEntityType::Chunk, "chunk-1", &tag("status:changed"))
            .unwrap();
        db.set_tag_engine(TagEntityType::Chunk, "chunk-2", &tag("tag:auth"))
            .unwrap();

        assert_eq!(
            db.clear_chunk_status_changed_for_document("doc-1").unwrap(),
            1
        );
        assert!(db
            .list_tags(TagEntityType::Chunk, "chunk-1")
            .unwrap()
            .is_empty());
        assert_eq!(
            db.list_tags(TagEntityType::Chunk, "chunk-2").unwrap(),
            vec![tag("tag:auth")]
        );
    }

    #[test]
    fn remove_tag_requires_exact_local_pair() {
        let db = Database::open_memory().unwrap();
        db.set_tag_engine(TagEntityType::Chunk, "chunk-1", &tag("status:planned"))
            .unwrap();
        db.set_tag_engine(TagEntityType::Chunk, "chunk-1", &tag("status:implemented"))
            .unwrap();

        assert_eq!(
            db.remove_tag_engine(TagEntityType::Chunk, "chunk-1", &tag("status:planned"))
                .unwrap(),
            1
        );
        assert_eq!(
            db.list_tags(TagEntityType::Chunk, "chunk-1").unwrap(),
            vec![tag("status:implemented")]
        );
    }
}
