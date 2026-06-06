# Tags Specification

## Purpose

Provide a key:value metadata model for documents and chunks so agents can classify, inspect, and filter Cite content without relying on the legacy topic/concept hierarchy.

## Requirements

### Requirement: Tag Entity Model

The system MUST store tags as flat key:value pairs on `document` and `chunk` entities. The system MUST allow multiple values for the same key on the same entity, but MUST NOT store duplicate key:value pairs for the same entity and entity type.

#### Scenario: Store multiple tag values

- GIVEN a chunk entity exists
- WHEN tags `tag:auth` and `tag:security` are set on the chunk
- THEN both tags MUST be returned for that chunk
- AND setting `tag:auth` again MUST NOT create a duplicate tag

### Requirement: Tag Persistence

The system MUST persist tags durably in SQLite with enough entity scope to distinguish document tags from chunk tags. The tag store MUST support efficient lookup by entity, key, and key:value pair.

#### Scenario: Retrieve persisted tags after restart

- GIVEN a document has local tag `type:rfc`
- WHEN the database is closed and reopened
- THEN reading tags for the document MUST return `type:rfc`

### Requirement: Reserved Tag Keys

The system MUST reserve the keys `workspace`, `type`, `session`, and `source_kind` for engine-managed metadata. User-facing tag commands MUST reject attempts to set or remove reserved keys unless the operation is explicitly engine-owned.

#### Scenario: Reject user write to reserved key

- GIVEN a document exists
- WHEN a user runs `cite tag set <document_id> workspace:aiharness`
- THEN the command MUST fail validation
- AND existing tags MUST remain unchanged

#### Scenario: Allow engine auto-tag write

- GIVEN a document is ingested from a known workspace
- WHEN ingest assigns `workspace:<name>` and `source_kind:document`
- THEN the engine MUST store those reserved tags
- AND user-facing reserved-key validation MUST NOT block the engine-owned write

### Requirement: Descriptive Tag Storage

The system MUST store inherited or descriptive tags locally on both documents and chunks when those tags are intended to support filtering at both scopes. The system MUST NOT require runtime inheritance inference for normal descriptive document and chunk filters.

#### Scenario: Store document and chunk descriptive tags

- GIVEN a document path maps to `type:rfc` and `workspace:aiharness`
- WHEN the document is ingested and chunked
- THEN the document MUST have local `type:rfc` and `workspace:aiharness` tags
- AND each chunk from that document MUST have local `type:rfc` and `workspace:aiharness` tags

### Requirement: Non-Inheritable Status Tags

The system MUST treat `status` as a local-only, non-inheritable tag key. The system MUST NOT propagate `status` tags between documents and chunks in either direction.

#### Scenario: Chunk status does not update document status

- GIVEN a document has no local `status` tag
- AND one chunk in that document has local tag `status:changed`
- WHEN document tags are read
- THEN the document MUST NOT include `status:changed`

#### Scenario: Document status does not update chunk status

- GIVEN a document has local tag `status:planned`
- AND a chunk in that document has no local `status` tag
- WHEN chunk tags are read
- THEN the chunk MUST NOT include `status:planned`

### Requirement: Changed Status Is Chunk-Only

The system MUST NOT allow document entities to receive local tag `status:changed` in v0.4.0. `status:changed` MUST represent changed chunk content only.

#### Scenario: Reject document changed status

- GIVEN a document entity exists
- WHEN a user or engine path attempts to store local tag `status:changed` on that document
- THEN the system MUST reject or ignore that document-level tag
- AND no document-local `status:changed` MUST be persisted

### Requirement: Tag CLI Operations

The system MUST provide `cite tag set`, `cite tag get`, and `cite tag rm` operations for document and chunk entities. `set` and `rm` MUST require explicit `key:value` input. `get` MUST return local tags for the target entity.

#### Scenario: Set and read tags

- GIVEN a chunk entity exists
- WHEN a user runs `cite tag set <chunk_id> status:implemented tag:auth`
- AND then runs `cite tag get <chunk_id>`
- THEN the output MUST include `status:implemented` and `tag:auth`

#### Scenario: Remove one tag without affecting others

- GIVEN a chunk has tags `status:implemented` and `tag:auth`
- WHEN a user runs `cite tag rm <chunk_id> status:implemented`
- THEN `status:implemented` MUST be removed from that chunk
- AND `tag:auth` MUST remain on that chunk

### Requirement: Tag Input Validation

The system MUST reject empty keys, empty values in `key:value` form, malformed tag strings, and key-only inputs for tag mutation commands. Key-only tag syntax MAY be supported for filters only.

#### Scenario: Reject malformed tag mutation

- GIVEN a document exists
- WHEN a user runs `cite tag set <document_id> status:`
- THEN the command MUST fail validation
- AND no tag MUST be stored for that input

#### Scenario: Reject key-only mutation

- GIVEN a chunk exists
- WHEN a user runs `cite tag rm <chunk_id> status`
- THEN the command MUST fail validation
- AND local status tags MUST remain unchanged

### Requirement: Path-Based Auto-Tags

The system MUST assign engine-managed path-based tags during ingest for recognized OpenSpec folders. Paths under `openspec/prd/`, `openspec/specs/`, `openspec/architecture/`, `openspec/guides/`, and `openspec/rfc/` MUST map respectively to `type:prd`, `type:spec`, `type:architecture`, `type:guide`, and `type:rfc`.

#### Scenario: Ingest RFC path

- GIVEN a source file path `openspec/rfc/active/example.md`
- WHEN the file is ingested
- THEN the document MUST have local tag `type:rfc`
- AND chunks produced from the document MUST have local tag `type:rfc`
