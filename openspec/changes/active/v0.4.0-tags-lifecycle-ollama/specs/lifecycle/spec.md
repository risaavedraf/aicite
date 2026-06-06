# Lifecycle Specification

## Purpose

Track source freshness during ingest and re-ingest so Cite can skip unchanged files and mark changed chunks for review without confusing lifecycle metadata with pipeline status.

## Requirements

### Requirement: Document Lifecycle Metadata

The system MUST store source lifecycle metadata for ingested physical documents, including `source_hash`, `ingested_at`, and `file_modified_at`. `source_hash` MUST represent source file contents at ingest time; `ingested_at` MUST represent when Cite processed the source; and `file_modified_at` MUST represent the source file modification time observed during ingest.

#### Scenario: Store lifecycle fields on ingest

- GIVEN a valid physical source file
- WHEN the file is ingested
- THEN the resulting document MUST store a non-empty `source_hash`
- AND the document MUST store `ingested_at`
- AND the document MUST store `file_modified_at` when the filesystem provides it

### Requirement: Source Identity Lookup

The system MUST identify prior physical documents for re-ingest by source path or an equivalent stable source identity. Re-ingesting a changed source MUST update or replace the existing source representation instead of silently creating duplicate active documents for the same path.

#### Scenario: Re-ingest same path does not duplicate active source

- GIVEN a document exists for source path `docs/a.md`
- WHEN `docs/a.md` is ingested again with changed content
- THEN Cite MUST associate the operation with the existing source identity
- AND the active document list MUST NOT contain duplicate active documents for `docs/a.md`

### Requirement: Unchanged Re-Ingest Skip

The system MUST compare the current source hash with the stored document `source_hash` during re-ingest of the same source path. If the hash is unchanged, the system MUST skip reprocessing chunks and embeddings for that file.

#### Scenario: Skip unchanged source

- GIVEN a document was previously ingested from a source path
- AND the current source file hash matches the stored `source_hash`
- WHEN the same source path is ingested again
- THEN Cite MUST skip re-chunking and re-embedding that document
- AND existing chunks and embeddings MUST remain unchanged

### Requirement: Changed Re-Ingest Processing

The system MUST process a source file when its current source hash differs from the stored `source_hash` for the same source path. The system MUST update document lifecycle metadata after successful processing.

#### Scenario: Process changed source

- GIVEN a document was previously ingested from a source path
- AND the current source file hash differs from the stored `source_hash`
- WHEN the same source path is ingested again
- THEN Cite MUST process the changed source content
- AND the document `source_hash` MUST be updated to the new hash after successful processing
- AND `ingested_at` MUST reflect the latest successful processing time

### Requirement: Chunk-Local Changed Status

The system MUST mark changed content using chunk-local tag `status:changed`. The system MUST NOT automatically assign `status:changed` to the parent document when one or more chunks changed.

#### Scenario: Mark changed chunks only

- GIVEN a re-ingested document has changed content in one chunk
- WHEN changed chunks are identified
- THEN the changed chunk MUST receive local tag `status:changed`
- AND the parent document MUST NOT receive `status:changed` automatically

#### Scenario: Do not mark unchanged chunks as changed

- GIVEN a re-ingested document has some chunks whose content is known to be unchanged
- WHEN changed-content marking runs
- THEN chunks known to be unchanged MUST NOT receive local tag `status:changed`

### Requirement: Changed Status Recalculation

The system MUST recompute chunk-local `status:changed` tags for a source during successful changed re-ingest. Stale `status:changed` tags from an earlier ingest MUST NOT remain on chunks that are no longer changed under the latest source comparison.

#### Scenario: Clear stale changed status

- GIVEN a chunk previously had local tag `status:changed`
- AND the next successful re-ingest determines the corresponding content is no longer changed
- WHEN lifecycle tags are updated
- THEN that stale local `status:changed` tag MUST be removed or not recreated

### Requirement: Changed Chunk Detection Safety

The system SHOULD mark only chunks known to be new or changed as `status:changed`. If exact changed-chunk detection is not possible for a re-ingest case, the system MUST NOT imply document-level status aggregation and MUST document or report the detection limitation.

#### Scenario: Detection limitation does not create document status

- GIVEN a changed source file cannot be matched exactly to prior chunk boundaries
- WHEN Cite applies changed-content marking
- THEN Cite MUST NOT add document-local `status:changed` as a substitute for chunk detection
- AND any marked `status:changed` tags MUST remain local to chunks

### Requirement: Lifecycle Metadata Separation from Pipeline Status

The system MUST keep source lifecycle metadata and `status:*` tags distinct from document ingestion pipeline status. Pipeline states such as processing, ready, or failed MUST NOT be treated as tag inheritance or user-facing feature status.

#### Scenario: Pipeline ready is not feature status

- GIVEN a document pipeline status is ready
- WHEN tags for that document are read
- THEN the system MUST NOT synthesize `status:ready` from the pipeline status
