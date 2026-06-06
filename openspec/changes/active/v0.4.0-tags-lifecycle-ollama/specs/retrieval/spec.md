# Retrieval Specification

## Purpose

Define retrieval-facing metadata filtering behavior for the tag transition while preserving existing topic and concept filters.

## Requirements

### Requirement: Tag Filters on Retrieval Commands

The system MUST support `--tag` filters on `search`, `retrieve`, and `context`. Multiple `--tag` filters MUST use AND semantics. Key:value filters MUST match exact local tags; key-only filters MAY match any local value for that key when the command explicitly supports key-only parsing.

#### Scenario: Context uses multiple tag filters

- GIVEN chunks exist with tags `type:spec`, `status:implemented`, and `status:planned`
- WHEN a user runs `cite context "provider" --tag type:spec --tag status:implemented`
- THEN returned context MUST be based on chunks that have both local tags
- AND chunks missing either tag MUST NOT be included because of the tag-filtered query

### Requirement: Pre-Ranking Tag Filtering

The system MUST apply supported tag filters before final vector ranking for retrieval commands so non-matching candidates are excluded from ranking results.

#### Scenario: Exclude non-matching candidates before ranking

- GIVEN chunks exist for the query term with local tags `status:implemented` and `status:planned`
- WHEN a user runs `cite retrieve "auth" --tag status:implemented`
- THEN chunks without local `status:implemented` MUST be excluded before final ranked results are returned

### Requirement: Status Filters Are Chunk-Local in Retrieval

The system MUST evaluate retrieval `--tag status:*` filters against chunk-local status tags only. Retrieval MUST NOT include a chunk because its parent document has a matching status tag, and MUST NOT include sibling chunks because one chunk has a matching status tag.

#### Scenario: Sibling chunk does not inherit changed status

- GIVEN a document contains chunk A with local tag `status:changed`
- AND chunk B has no local `status` tag
- WHEN a user runs `cite retrieve "update" --tag status:changed`
- THEN chunk A MAY be returned if it otherwise matches the query
- AND chunk B MUST NOT be returned solely because it shares a document with chunk A

### Requirement: Tag Filters on List Command

The system MUST support `--tag` filters on `list` for document-local tags. List filters MUST use the same AND semantics as retrieval filters, but MUST evaluate document-local tags only.

#### Scenario: List filters documents by tag

- GIVEN documents exist with local tags `type:prd` and `type:rfc`
- WHEN a user runs `cite list --tag type:prd`
- THEN returned documents MUST have local tag `type:prd`
- AND documents with only local tag `type:rfc` MUST NOT be returned

### Requirement: List Status Filters Are Document-Local

The system MUST evaluate `cite list --tag status:*` against document-local status tags only. The list command MUST NOT infer document status from chunk-local status tags.

#### Scenario: List does not infer status from chunks

- GIVEN a document has no local `status:changed` tag
- AND one chunk in the document has local `status:changed`
- WHEN a user runs `cite list --tag status:changed`
- THEN that document MUST NOT be returned solely because its chunk is changed

### Requirement: Legacy Topic and Concept Filters Remain Supported

The system MUST preserve current topic and concept filter behavior during the tag transition. Tag filters MUST be additive and MUST NOT remove existing command-line options or silently reinterpret topic/concept as tags.

#### Scenario: Existing topic filter remains valid

- GIVEN a query worked with an existing topic filter before tag filtering was introduced
- WHEN the same topic-filtered query is run after tag filtering is introduced
- THEN the command MUST continue to accept and apply the topic filter according to legacy behavior
