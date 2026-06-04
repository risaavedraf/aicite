# Storage Hierarchy Specification

## Purpose

Define the schema migration and storage requirements for the hierarchical document structure (Document → Topic → Concept → Chunk). This migration adds three new tables and nullable foreign key columns to the existing `chunks` table, enabling hierarchical organization while maintaining full backward compatibility.

## Requirements

### Requirement: Migration 006 MUST create the `topics` table (MUST)

Migration 006 MUST create a `topics` table with the following schema:
- `topic_id TEXT PRIMARY KEY`
- `document_id TEXT NOT NULL REFERENCES documents(document_id)`
- `name TEXT NOT NULL`
- `summary TEXT`
- `embedding BLOB`
- `chunk_count INTEGER DEFAULT 0`
- `created_at TEXT NOT NULL DEFAULT (datetime('now'))`

#### Scenario: Topics table is created with correct schema

- GIVEN a database with migrations 001–005 applied
- WHEN migration 006 is applied
- THEN the `topics` table exists
- AND the `topics` table has columns: `topic_id`, `document_id`, `name`, `summary`, `embedding`, `chunk_count`, `created_at`
- AND `topic_id` is the primary key
- AND `document_id` has a foreign key reference to `documents(document_id)`
- AND `created_at` defaults to the current timestamp

#### Scenario: Topics table can store valid data

- GIVEN a database with migration 006 applied
- WHEN a topic row is inserted with valid `topic_id`, `document_id`, and `name`
- THEN the row is stored successfully
- AND `chunk_count` defaults to 0
- AND `created_at` is populated with a valid timestamp

### Requirement: Migration 006 MUST create the `concepts` table (MUST)

Migration 006 MUST create a `concepts` table with the following schema:
- `concept_id TEXT PRIMARY KEY`
- `topic_id TEXT NOT NULL REFERENCES topics(topic_id)`
- `name TEXT NOT NULL`
- `summary TEXT`
- `embedding BLOB`
- `chunk_count INTEGER DEFAULT 0`
- `created_at TEXT NOT NULL DEFAULT (datetime('now'))`

#### Scenario: Concepts table is created with correct schema

- GIVEN a database with migrations 001–005 applied
- WHEN migration 006 is applied
- THEN the `concepts` table exists
- AND the `concepts` table has columns: `concept_id`, `topic_id`, `name`, `summary`, `embedding`, `chunk_count`, `created_at`
- AND `concept_id` is the primary key
- AND `topic_id` has a foreign key reference to `topics(topic_id)`
- AND `created_at` defaults to the current timestamp

#### Scenario: Concepts table can store valid data

- GIVEN a database with migration 006 applied
- WHEN a concept row is inserted with valid `concept_id`, `topic_id`, and `name`
- THEN the row is stored successfully
- AND `chunk_count` defaults to 0
- AND `created_at` is populated with a valid timestamp

### Requirement: Migration 006 MUST create the `semantic_links` table (MUST)

Migration 006 MUST create a `semantic_links` table with the following schema:
- `link_id TEXT PRIMARY KEY`
- `source_chunk_id TEXT NOT NULL REFERENCES chunks(chunk_id)`
- `target_chunk_id TEXT NOT NULL REFERENCES chunks(chunk_id)`
- `similarity_score REAL NOT NULL`
- `link_type TEXT NOT NULL DEFAULT 'semantic'`
- `created_at TEXT NOT NULL DEFAULT (datetime('now'))`

#### Scenario: Semantic links table is created with correct schema

- GIVEN a database with migrations 001–005 applied
- WHEN migration 006 is applied
- THEN the `semantic_links` table exists
- AND the `semantic_links` table has columns: `link_id`, `source_chunk_id`, `target_chunk_id`, `similarity_score`, `link_type`, `created_at`
- AND `link_id` is the primary key
- AND `source_chunk_id` has a foreign key reference to `chunks(chunk_id)`
- AND `target_chunk_id` has a foreign key reference to `chunks(chunk_id)`
- AND `link_type` defaults to `'semantic'`
- AND `created_at` defaults to the current timestamp

#### Scenario: Semantic links table can store valid data

- GIVEN a database with migration 006 applied
- WHEN a semantic link row is inserted with valid `link_id`, `source_chunk_id`, `target_chunk_id`, and `similarity_score`
- THEN the row is stored successfully
- AND `link_type` defaults to `'semantic'`
- AND `created_at` is populated with a valid timestamp

### Requirement: Migration 006 MUST add nullable `concept_id` and `topic_id` columns to `chunks` (MUST)

Migration 006 MUST add two nullable columns to the existing `chunks` table:
- `concept_id TEXT REFERENCES concepts(concept_id)` (nullable)
- `topic_id TEXT REFERENCES topics(topic_id)` (nullable)

#### Scenario: Nullable columns are added to chunks table

- GIVEN a database with migrations 001–005 applied and existing chunk rows
- WHEN migration 006 is applied
- THEN the `chunks` table has columns: `concept_id`, `topic_id`
- AND both columns are nullable (allow NULL values)
- AND `concept_id` has a foreign key reference to `concepts(concept_id)`
- AND `topic_id` has a foreign key reference to `topics(topic_id)`

#### Scenario: Existing chunks have NULL concept_id and topic_id after migration

- GIVEN a database with migrations 001–005 applied and existing chunk rows
- WHEN migration 006 is applied
- THEN all existing chunk rows have `concept_id = NULL`
- AND all existing chunk rows have `topic_id = NULL`
- AND existing chunk data (text, offset, page, etc.) is unchanged

#### Scenario: New chunks can have concept_id and topic_id set

- GIVEN a database with migration 006 applied
- WHEN a new chunk is inserted with valid `concept_id` and `topic_id`
- THEN the row is stored successfully
- AND the foreign key references are valid

### Requirement: Migration 006 MUST create indexes for hierarchy queries (MUST)

Migration 006 MUST create the following indexes:
- `idx_chunks_concept ON chunks(concept_id)`
- `idx_chunks_topic ON chunks(topic_id)`
- `idx_topics_document ON topics(document_id)`
- `idx_concepts_topic ON concepts(topic_id)`
- `idx_semantic_links_source ON semantic_links(source_chunk_id)`
- `idx_semantic_links_target ON semantic_links(target_chunk_id)`

#### Scenario: All required indexes are created

- GIVEN a database with migrations 001–005 applied
- WHEN migration 006 is applied
- THEN the following indexes exist:
  - `idx_chunks_concept`
  - `idx_chunks_topic`
  - `idx_topics_document`
  - `idx_concepts_topic`
  - `idx_semantic_links_source`
  - `idx_semantic_links_target`

### Requirement: Migration 006 MUST be idempotent (MUST)

Migration 006 MUST use `CREATE TABLE IF NOT EXISTS` and `CREATE INDEX IF NOT EXISTS` to ensure idempotency. The migration runner MUST skip migration 006 if it has already been applied (version check).

#### Scenario: Migration 006 can be applied multiple times without error

- GIVEN a database with migration 006 already applied
- WHEN the migration runner executes again
- THEN migration 006 is skipped (version check)
- AND no errors occur
- AND existing data is unchanged

#### Scenario: Migration 006 handles partial application gracefully

- GIVEN a database where migration 006 partially applied (e.g., `topics` table created but `concepts` not yet)
- WHEN migration 006 is applied again
- THEN `CREATE TABLE IF NOT EXISTS` statements succeed for already-created tables
- AND remaining tables are created
- AND no duplicate table errors occur

### Requirement: Migration 006 MUST NOT break existing queries (MUST)

Migration 006 MUST NOT alter the behavior of existing queries on `documents`, `chunks`, and `embeddings` tables. Nullable FK columns mean existing queries that don't reference `concept_id` or `topic_id` continue to work unchanged.

#### Scenario: Existing retrieval queries work unchanged after migration

- GIVEN a database with migrations 001–005 applied and existing data
- WHEN migration 006 is applied
- AND an existing retrieval query is executed (e.g., SELECT on chunks with embeddings)
- THEN the query returns the same results as before migration 006
- AND NULL `concept_id`/`topic_id` values do not affect query results

#### Scenario: Existing insert operations work unchanged after migration

- GIVEN a database with migration 006 applied
- WHEN a new document is ingested using the v0.1.0 ingest path (no hierarchy)
- THEN chunks are inserted successfully with `concept_id = NULL` and `topic_id = NULL`
- AND the ingest completes without errors

### Requirement: Migration 006 MUST handle foreign key constraints correctly (MUST)

Migration 006 MUST enforce referential integrity for foreign keys while allowing NULL values in nullable columns.

#### Scenario: Foreign key constraint prevents invalid references

- GIVEN a database with migration 006 applied
- WHEN a chunk is inserted with `concept_id` referencing a non-existent concept
- THEN the insert fails with a foreign key constraint violation

#### Scenario: Foreign key constraint allows NULL values

- GIVEN a database with migration 006 applied
- WHEN a chunk is inserted with `concept_id = NULL` and `topic_id = NULL`
- THEN the insert succeeds
- AND no foreign key constraint violation occurs

### Requirement: Migration runner MUST include migration 006 (MUST)

The migration runner in `crates/storage/src/migrations/mod.rs` MUST include migration 006 with the constant `HIERARCHY_SCHEMA` loaded from `006_hierarchy.sql`.

#### Scenario: Migration runner applies migration 006 when version < 6

- GIVEN a database with version 5 (migrations 001–005 applied)
- WHEN the migration runner executes
- THEN migration 006 is applied
- AND the version is updated to 6
- AND the new tables and columns exist

#### Scenario: Migration runner skips migration 006 when version >= 6

- GIVEN a database with version 6 (migration 006 already applied)
- WHEN the migration runner executes
- THEN migration 006 is skipped
- AND the version remains 6

### Requirement: Storage crate MUST provide query helpers for topics and concepts (SHOULD)

The storage crate SHOULD provide helper functions for common hierarchy queries:
- `insert_topic(conn, topic) -> Result<()>`
- `insert_concept(conn, concept) -> Result<()>`
- `update_chunk_hierarchy(conn, chunk_id, topic_id, concept_id) -> Result<()>`
- `get_topics_by_document(conn, document_id) -> Result<Vec<Topic>>`
- `get_concepts_by_topic(conn, topic_id) -> Result<Vec<Concept>>`

#### Scenario: Query helpers can insert and retrieve hierarchy data

- GIVEN a database with migration 006 applied
- WHEN a topic is inserted using `insert_topic`
- AND concepts are inserted using `insert_concept`
- AND chunks are updated using `update_chunk_hierarchy`
- THEN `get_topics_by_document` returns the correct topics
- AND `get_concepts_by_topic` returns the correct concepts
- AND chunk rows have the correct `topic_id` and `concept_id` values

### Requirement: Migration 006 MUST handle edge cases (SHOULD)

Migration 006 SHOULD handle edge cases gracefully:
- Empty database (no existing documents/chunks)
- Database with existing data but no embeddings
- Database with orphaned chunks (no matching document)

#### Scenario: Migration works on empty database

- GIVEN a database with migrations 001–005 applied but no data rows
- WHEN migration 006 is applied
- THEN all tables and indexes are created
- AND no errors occur
- AND all tables are empty

#### Scenario: Migration preserves orphaned chunks

- GIVEN a database with orphaned chunks (document deleted but chunks remain)
- WHEN migration 006 is applied
- THEN orphaned chunks have `concept_id = NULL` and `topic_id = NULL`
- AND no foreign key violations occur (orphaned chunks are not deleted)
