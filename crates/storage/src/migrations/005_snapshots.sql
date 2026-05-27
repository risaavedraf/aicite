-- Snapshot metadata for atomic refresh
CREATE TABLE IF NOT EXISTS corpus_snapshots (
    snapshot_id TEXT PRIMARY KEY,
    state TEXT NOT NULL CHECK (
        state IN ('building', 'active', 'superseded', 'failed')
    ),
    created_at TEXT NOT NULL,
    activated_at TEXT,
    superseded_at TEXT,
    error_code TEXT,
    error_message TEXT
);

-- Documents belonging to a snapshot
CREATE TABLE IF NOT EXISTS snapshot_members (
    snapshot_id TEXT NOT NULL,
    document_id TEXT NOT NULL,
    PRIMARY KEY (snapshot_id, document_id)
);

-- Single-row pointer to the active snapshot
CREATE TABLE IF NOT EXISTS snapshot_pointer (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    active_snapshot_id TEXT NOT NULL
);

-- Index for membership lookups during retrieval
CREATE INDEX IF NOT EXISTS idx_snapshot_members_document
ON snapshot_members (document_id);
