CREATE TABLE IF NOT EXISTS trace_citations (
    trace_id TEXT NOT NULL,
    citation_id TEXT NOT NULL,
    document_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    chunk_id TEXT NOT NULL,
    page INTEGER,
    offset_start INTEGER,
    offset_end INTEGER,
    text TEXT NOT NULL,
    score REAL,
    confidence_label TEXT,
    PRIMARY KEY (trace_id, citation_id),
    FOREIGN KEY (trace_id) REFERENCES traces (trace_id)
);

CREATE INDEX IF NOT EXISTS idx_trace_citations_trace_id ON trace_citations (
    trace_id
);
CREATE INDEX IF NOT EXISTS idx_trace_citations_document_chunk ON trace_citations (
    document_id, chunk_id
);
