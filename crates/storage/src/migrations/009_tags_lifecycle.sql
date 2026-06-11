-- Migration 009: Tags and document lifecycle metadata
-- Additive foundation for key:value metadata and source freshness tracking.

CREATE TABLE IF NOT EXISTS tags (
    tag_id TEXT PRIMARY KEY,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL CHECK (entity_type IN ('document', 'chunk')),
    key TEXT NOT NULL CHECK (length(trim(key)) > 0),
    value TEXT NOT NULL CHECK (length(trim(value)) > 0),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(entity_type, entity_id, key, value)
);

CREATE INDEX IF NOT EXISTS idx_tags_entity
    ON tags(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_tags_key
    ON tags(key);
CREATE INDEX IF NOT EXISTS idx_tags_key_value
    ON tags(key, value);
CREATE INDEX IF NOT EXISTS idx_tags_filter
    ON tags(entity_type, key, value, entity_id);

ALTER TABLE documents ADD COLUMN source_hash TEXT;
ALTER TABLE documents ADD COLUMN ingested_at TEXT;
ALTER TABLE documents ADD COLUMN file_modified_at TEXT;

CREATE INDEX IF NOT EXISTS idx_documents_file_path
    ON documents(file_path);
CREATE INDEX IF NOT EXISTS idx_documents_source_hash
    ON documents(source_hash);
