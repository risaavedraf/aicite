# Phase 10 Proposal — Hierarchical Graph Foundation

## Problem statement

v0.1.0 uses a flat vector store: `Document → Chunk (500–1000 chars) → Embedding`. Chunks are large and contain mixed topics, which dilutes cosine similarity scores (observed 0.62–0.69) and causes frequent `insufficient_context` results even when the answer exists in the corpus.

```
Query: "How are passwords validated?"
Chunk: "The API gateway routes requests... JWT tokens... Passwords must be 12+ chars... Logging uses ELK..."
cosine(query, chunk) = 0.65   ← match diluted by noise
```

The fix is architectural: smaller, topic-scoped chunks (30–200 chars) organized under a `Document → Topic → Concept → Chunk` hierarchy. Phase 10 builds the data model layer only; retrieval changes follow in Phase 11.

## Why now

Phase 9 closed the installation/migration story. Phase 10 is the first v0.2.0 work item and the prerequisite for hierarchical retrieval (Phase 11) and topic management CLI (Phase 12). Delivering the schema + ingest wiring first keeps the change reviewable and avoids a single large PR across schema, retrieval, and CLI.

## Proposed solution

### 1. Schema migration 006

Add three new tables and two nullable FK columns on `chunks`:

```sql
-- New tables
CREATE TABLE IF NOT EXISTS topics (
    topic_id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL REFERENCES documents(document_id),
    name TEXT NOT NULL,
    summary TEXT,
    embedding BLOB,
    chunk_count INTEGER DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS concepts (
    concept_id TEXT PRIMARY KEY,
    topic_id TEXT NOT NULL REFERENCES topics(topic_id),
    name TEXT NOT NULL,
    summary TEXT,
    embedding BLOB,
    chunk_count INTEGER DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS semantic_links (
    link_id TEXT PRIMARY KEY,
    source_chunk_id TEXT NOT NULL REFERENCES chunks(chunk_id),
    target_chunk_id TEXT NOT NULL REFERENCES chunks(chunk_id),
    similarity_score REAL NOT NULL,
    link_type TEXT NOT NULL DEFAULT 'semantic',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Modify existing chunks table (nullable — backward compatible)
ALTER TABLE chunks ADD COLUMN concept_id TEXT REFERENCES concepts(concept_id);
ALTER TABLE chunks ADD COLUMN topic_id TEXT REFERENCES topics(topic_id);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_chunks_concept ON chunks(concept_id);
CREATE INDEX IF NOT EXISTS idx_chunks_topic ON chunks(topic_id);
CREATE INDEX IF NOT EXISTS idx_topics_document ON topics(document_id);
CREATE INDEX IF NOT EXISTS idx_concepts_topic ON concepts(topic_id);
CREATE INDEX IF NOT EXISTS idx_semantic_links_source ON semantic_links(source_chunk_id);
CREATE INDEX IF NOT EXISTS idx_semantic_links_target ON semantic_links(target_chunk_id);
```

Design note: `concept_id` and `topic_id` on `chunks` are nullable, so existing rows are untouched. Flat retrieval continues to work without modification.

### 2. Graph crate — domain types + hierarchy builder

The `graph` crate (currently a stub with `pub struct Graph;`) grows to own:

- **Domain types**: `Topic`, `Concept`, `SemanticLink` (Rust structs mapped to the new tables).
- **Hierarchy builder**: function that takes a document's parsed sections and produces a `Vec<Topic>` with nested `Vec<Concept>`, ready for storage.
- **Heading parser**: extracts `##` → topic, `###` → concept from markdown content. Falls back to a single "Untitled" topic for non-markdown files.

The graph crate does **not** own ingestion I/O or storage writes — `ingest` calls graph, then writes to storage.

### 3. Sentence-based chunking strategy

New function `chunk_by_sentence(text: &str) -> Vec<String>` in the `ingest` crate:

- Split on sentence boundaries (`.`, `!`, `?` followed by whitespace or EOF).
- Merge adjacent sentences if combined length < 30 chars (avoid tiny fragments).
- Hard cap at 200 chars per chunk; if a single sentence exceeds 200 chars, split on the nearest clause boundary.
- Activated by a config flag (`sentence_chunking: true`); existing fixed-size chunking remains the default until the flag is flipped.

This coexists with the existing `chunk_by_fixed_size` path. The config toggle lets us A/B test before making it the default.

### 4. Config additions

New fields in the config crate:

| Field | Type | Default | Description |
|---|---|---|---|
| `sentence_chunking` | `bool` | `false` | Enable sentence-based chunking instead of fixed-size |
| `min_chunk_chars` | `usize` | `30` | Minimum chunk length before merge |
| `max_chunk_chars` | `usize` | `200` | Maximum chunk length |
| `build_hierarchy` | `bool` | `false` | Extract topics/concepts during ingest |

### 5. Ingest pipeline wiring

When `build_hierarchy` is enabled:

1. Parse document headings → produce topic/concept skeleton via `graph::build_hierarchy`.
2. Chunk text using sentence-based strategy (if `sentence_chunking` is on).
3. Assign each chunk to its parent concept.
4. Write topics, concepts, and chunk FKs to storage in a single transaction.
5. Embeddings are generated per-chunk as before (no change to embedding pipeline).

When `build_hierarchy` is false, ingest behaves identically to v0.1.0.

## Scope boundaries

### In scope

- Migration 006 (new tables + nullable FK columns + indexes).
- Domain types in `graph` crate (`Topic`, `Concept`, `SemanticLink`).
- Heading-based hierarchy builder (`graph::build_hierarchy`).
- Sentence-based chunking function + config flag.
- Ingest pipeline integration gated by `build_hierarchy` config flag.
- Unit tests for hierarchy builder, sentence chunker, and migration 006.

### Out of scope

- **Retrieval changes** — chunk-first search with topic enrichment (Phase 11).
- **CLI topic commands** — `topics list`, `topics rename`, `topics move-concept`, etc. (Phase 12).
- **Semantic clustering** — cross-document link creation via embedding similarity (Phase 11+).
- **Embedding topic/concept vectors** — only chunk embeddings are generated; topic/concept embeddings are deferred.
- **semantic_links population** — table is created empty; population is Phase 11 work.

## Affected areas

| Crate | Change |
|---|---|
| `storage` | New migration file `006_hierarchy.sql`; query helpers for topics/concepts |
| `graph` | Domain types, hierarchy builder, heading parser (currently empty stub) |
| `ingest` | Sentence chunker function; pipeline branch gated by config |
| `config` | New config fields (`sentence_chunking`, `min/max_chunk_chars`, `build_hierarchy`) |
| `common` | Shared types if needed for Topic/Concept (TBD in design phase) |

## Acceptance criteria

1. **Migration 006** applies cleanly on a database with existing data (migrations 001–005); all existing rows have NULL `concept_id`/`topic_id`.
2. **Backward compatibility**: `cite ingest` and `cite context` behave identically when `build_hierarchy = false` (default).
3. **Hierarchy builder**: given a markdown document with `##` and `###` headings, `build_hierarchy` produces the correct topic/concept tree.
4. **Sentence chunker**: given a 2000-char text with 15 sentences, `chunk_by_sentence` produces chunks where each is 30–200 chars and no sentence is split across chunks.
5. **Ingest with hierarchy**: running ingest with `build_hierarchy = true` creates topic and concept rows in the database and sets `topic_id`/`concept_id` on the corresponding chunks.
6. **Non-markdown fallback**: ingesting a `.txt` file with `build_hierarchy = true` creates a single topic named "Untitled" containing all chunks.
7. **Unit tests pass**: all new and existing tests pass (`cargo test`).
8. **Review budget**: no single PR exceeds 400 changed lines.

## Non-goals

- Replacing cosine similarity or changing the embedding model.
- Real-time or streaming hierarchy updates.
- Multi-user or concurrent hierarchy editing.
- Topic/concept-level embedding and search (deferred to Phase 11+).

## Risks and mitigations

| Risk | Severity | Mitigation |
|---|---|---|
| **Chunk count explosion** — sentence chunking produces 3–10× more chunks, increasing embedding costs | HIGH | Keep `sentence_chunking` default `false`; measure chunk count and cost in a benchmark before flipping. Gate behind config flag so users opt in. |
| **UTF-8 offset tracking** — sentence boundary detection on multi-byte characters | MEDIUM | Use `char_indices()` for all offset math; add property-based tests with Unicode-heavy fixtures. |
| **Non-markdown files have no headings** — hierarchy is meaningless for `.txt`, `.pdf` without structure | MEDIUM | Fall back to single "Untitled" topic; document this limitation. Future phases can add semantic sectioning. |
| **Backward compat regression** — existing corpus queries break | LOW | Nullable FKs mean existing chunks are NULL and flat retrieval path is unchanged. Integration test: ingest with v0.1.0, migrate, query returns same results. |
| **semantic_links table unused** — empty table could confuse consumers | LOW | Document that the table is a forward placeholder; no queries reference it in Phase 10. |

## Dependencies and sequencing

```
Phase 9 (Installation Experience) ── completed
         │
         ▼
   Phase 10 (Hierarchical Graph Foundation) ◄── this proposal
         │
         ├── Migration 006         (no deps)
         ├── Graph domain types    (no deps)
         ├── Sentence chunker      (no deps)
         ├── Hierarchy builder     (depends on graph types)
         └── Ingest wiring         (depends on all above)
         │
         ▼
   Phase 11 (Hierarchical Retrieval)
         │
         ▼
   Phase 12 (Topic Management CLI)
```

Internal sequencing within Phase 10:

1. **Migration 006** — unblocks everything; can be reviewed in isolation.
2. **Graph domain types** — pure types, no I/O; small PR.
3. **Sentence chunker** — independent function + tests; parallel with graph types.
4. **Hierarchy builder** — depends on domain types; heading parser + builder logic.
5. **Ingest wiring** — integrates all pieces; gated by config flags.
6. **Integration tests** — end-to-end: ingest a markdown file with hierarchy enabled, verify DB state.

## Proposed slices (for tasks phase)

1. **Slice A — Migration 006**: new SQL file, migration runner test, verify existing data preserved.
   Allowlist: `crates/storage/src/migrations/006_hierarchy.sql`, storage migration tests.
2. **Slice B — Graph domain types**: `Topic`, `Concept`, `SemanticLink` structs, serialization, `Default`/`Builder` patterns.
   Allowlist: `crates/graph/src/lib.rs`, `crates/graph/src/types.rs` (new).
3. **Slice C — Sentence chunker**: `chunk_by_sentence` function, config fields, unit tests with ASCII + Unicode fixtures.
   Allowlist: `crates/ingest/src/chunk.rs` (or new file), `crates/config/src/lib.rs`.
4. **Slice D — Hierarchy builder**: heading parser, `build_hierarchy` function, unit tests with sample markdown.
   Allowlist: `crates/graph/src/hierarchy.rs` (new), `crates/graph/src/lib.rs`.
5. **Slice E — Ingest pipeline integration**: wire hierarchy + sentence chunker into ingest path, gated by config; integration test.
   Allowlist: `crates/ingest/src/pipeline.rs` (or relevant file), integration test files.
6. **Slice F — Verification + closeout**: run full test suite, verify backward compat, document known limitations.
   Allowlist: test output, docs/sdd notes.

Each slice targets ≤400 changed lines.
