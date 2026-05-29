-- Migration 006: Hierarchical graph structure
-- Adds topics, concepts, semantic_links tables and hierarchy FKs on chunks

-- Topics: semantic sections within documents
CREATE TABLE IF NOT EXISTS topics (
    topic_id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL REFERENCES documents(document_id),
    name TEXT NOT NULL,
    summary TEXT,
    embedding BLOB,
    chunk_count INTEGER DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Concepts: atomic knowledge units within topics
CREATE TABLE IF NOT EXISTS concepts (
    concept_id TEXT PRIMARY KEY,
    topic_id TEXT NOT NULL REFERENCES topics(topic_id),
    name TEXT NOT NULL,
    summary TEXT,
    embedding BLOB,
    chunk_count INTEGER DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Cross-document semantic links (created empty, populated in Phase 11+)
CREATE TABLE IF NOT EXISTS semantic_links (
    link_id TEXT PRIMARY KEY,
    source_chunk_id TEXT NOT NULL REFERENCES chunks(chunk_id),
    target_chunk_id TEXT NOT NULL REFERENCES chunks(chunk_id),
    similarity_score REAL NOT NULL,
    link_type TEXT NOT NULL DEFAULT 'semantic',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(source_chunk_id, target_chunk_id)
);

-- Add hierarchy references to chunks (nullable for backward compat)
ALTER TABLE chunks ADD COLUMN concept_id TEXT REFERENCES concepts(concept_id);
ALTER TABLE chunks ADD COLUMN topic_id TEXT REFERENCES topics(topic_id);

-- Indexes for hierarchy traversal
CREATE INDEX IF NOT EXISTS idx_chunks_concept ON chunks(concept_id);
CREATE INDEX IF NOT EXISTS idx_chunks_topic ON chunks(topic_id);
CREATE INDEX IF NOT EXISTS idx_topics_document ON topics(document_id);
CREATE INDEX IF NOT EXISTS idx_concepts_topic ON concepts(topic_id);
CREATE INDEX IF NOT EXISTS idx_semantic_links_source ON semantic_links(source_chunk_id);
CREATE INDEX IF NOT EXISTS idx_semantic_links_target ON semantic_links(target_chunk_id);
CREATE INDEX IF NOT EXISTS idx_chunks_topic_concept ON chunks(topic_id, concept_id);
