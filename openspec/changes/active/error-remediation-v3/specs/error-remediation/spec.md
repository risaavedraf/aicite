# Error Remediation V3 Specification

## Purpose

This change defines the expected behavior after the third error-remediation pass. It completes deferred type-safety, snapshot correctness, timestamp consistency, retrieval cleanup, and CodeRabbit validation work while preserving CLI-first behavior, SQLite persistence compatibility, and the configured review budget.

## Requirements

### Requirement: CodeRabbit Findings MUST Be Verified Before Remediation

The system MUST treat each finding listed in `coderabbit-findings.md` as advisory until verified against current code. A verified finding MUST be fixed with the smallest behavior-preserving change that satisfies the finding, while a stale, already-fixed, or non-reproducible finding MUST be skipped with a concise reason recorded in the change artifacts.

#### Scenario: Valid finding is fixed

- GIVEN a CodeRabbit finding is verified against current code and still reproduces
- WHEN the V3 implementation handles that finding
- THEN the relevant code or documentation MUST be updated to remove the verified issue
- AND the change MUST remain scoped to the finding unless the spec/design explicitly expands it

#### Scenario: Stale finding is skipped

- GIVEN a CodeRabbit finding no longer matches current code or cannot be reproduced
- WHEN the finding is evaluated
- THEN implementation MUST NOT introduce unnecessary changes for that finding
- AND the apply or verify artifact MUST record the skip reason

#### Scenario: Validation status is auditable

- GIVEN all CodeRabbit findings have been evaluated
- WHEN the change is verified
- THEN each finding MUST be marked as verified-fixed or skipped with reason
- AND no finding MUST remain in an ambiguous unreviewed state

### Requirement: CLI Health JSON Behavior MUST Match Its Contract

The `health` command MUST make its `--json` behavior clear and truthful. If JSON health output is specified as local-only, it MUST avoid live provider checks such as embedding test calls; if live checks are retained, help text and documentation MUST state that JSON output can perform provider checks.

#### Scenario: JSON is local-only

- GIVEN the command contract says `health --json` is local-only
- WHEN a user runs `health --json`
- THEN the command MUST NOT call provider embedding checks
- AND the JSON response MUST contain only local or configuration-derived health fields

#### Scenario: JSON performs live checks

- GIVEN the command contract says `health --json` includes live provider checks
- WHEN a user runs `health --json`
- THEN provider connectivity checks MAY run
- AND help text MUST not describe the command as local-only

### Requirement: CLI Setup MUST Persist Provider-Consistent Embedding Models

The setup flow MUST use an embedding model that is valid for the selected provider when testing connectivity and saving configuration. It MUST NOT blindly reuse an existing model configured for a different provider when the user selects a new provider.

#### Scenario: Provider-specific model is tested

- GIVEN setup is run and the user selects an embedding provider
- WHEN the setup flow tests the provider connection
- THEN the model used for the test MUST correspond to the selected provider
- AND the test MUST NOT use a stale model from a different provider selection

#### Scenario: Saved model matches selected provider

- GIVEN setup has completed successfully for a selected provider
- WHEN the configuration is persisted
- THEN the saved embedding provider and embedding model MUST form a valid pair
- AND subsequent configuration loading MUST preserve that pair

### Requirement: Configuration Tests MUST Be Deterministic And Environment-Safe

Tests that mutate `CITE_*` environment variables MUST restore the previous environment state after execution. Tests that validate environment fallback behavior MUST load configuration through an isolated path so host-local default TOML files cannot affect expected defaults.

#### Scenario: Environment variable is restored

- GIVEN a config test changes a `CITE_*` environment variable
- WHEN the test completes
- THEN the original variable value MUST be restored if it existed
- AND the variable MUST be removed if it was absent before the test

#### Scenario: Fallback defaults are isolated

- GIVEN a config test verifies fallback behavior for invalid environment input
- WHEN the test loads configuration
- THEN it MUST use an isolated nonexistent path or temp config fixture
- AND assertions MUST NOT depend on any host default config file

### Requirement: Rate-Limit Pruning MUST Reject Non-Positive Ages

Rate-limit pruning MUST validate that `max_age_seconds` is positive before computing deletion cutoffs. Non-positive values MUST return an error and MUST NOT delete active or recent rate-limit windows.

#### Scenario: Positive max age prunes stale rows

- GIVEN stored rate-limit rows older than a positive `max_age_seconds` cutoff
- WHEN pruning runs with that positive max age
- THEN stale rows MAY be deleted
- AND non-stale rows MUST remain

#### Scenario: Non-positive max age is rejected

- GIVEN active rate-limit rows exist
- WHEN pruning is requested with `max_age_seconds` equal to zero or less than zero
- THEN the function MUST return an invalid-argument style error
- AND no rate-limit rows MUST be deleted by that request

### Requirement: ScoredChunk Construction MUST Avoid Embedding Vector Clones

Retrieval ranking MUST NOT clone a `ChunkEmbeddingRecord` embedding vector merely to construct a `ScoredChunk`. The ranking path MUST support reference-based conversion or equivalent behavior that copies only required scalar/display fields and preserves ranking results.

#### Scenario: Ranked chunk is built without vector duplication

- GIVEN a retrieval candidate contains an embedding vector and scalar chunk metadata
- WHEN ranking converts the candidate into a `ScoredChunk`
- THEN the conversion MUST NOT clone the candidate vector
- AND the resulting `ScoredChunk` MUST retain the expected chunk id, document id, display name, section id, chunk index, text, page, offsets, score, and optional topic/concept fields

#### Scenario: Ranking output is unchanged

- GIVEN the same query vector and candidate records before and after clone-removal remediation
- WHEN ranking is executed
- THEN ranked chunk order and scores MUST remain equivalent except for intentional floating-point tolerance

### Requirement: Typed Identifiers MUST Replace Stringly-Typed IDs At Meaningful Boundaries

`DocumentId`, `ChunkId`, and `TraceId` MUST be adopted at meaningful crate and public API boundaries selected by the final design. The migration MUST preserve string serialization, CLI argument compatibility, SQLite persistence, fixtures, and clear conversion boundaries between raw strings and typed identifiers.

#### Scenario: Storage row decoding returns typed IDs in migrated paths

- GIVEN a migrated storage query decodes rows containing document, chunk, or trace identifiers
- WHEN the rows are converted to domain records
- THEN migrated identifier fields MUST use the corresponding newtype
- AND invalid or malformed identifier values MUST fail at the documented conversion boundary if validation exists for that newtype

#### Scenario: CLI string input remains compatible

- GIVEN a user supplies an identifier through a CLI argument or config value
- WHEN a migrated command uses that identifier internally
- THEN the command MUST accept the same string representation as before
- AND the internal migrated path MUST use the appropriate typed identifier after parsing

#### Scenario: Serialization remains stable

- GIVEN a migrated record is serialized to JSON or persisted through SQLite
- WHEN the record contains `DocumentId`, `ChunkId`, or `TraceId`
- THEN the external serialized or persisted value MUST remain the same string representation unless a deliberate compatibility break is documented

#### Scenario: Review slices are bounded

- GIVEN full typed-ID migration is forecast to exceed the 400-line review budget
- WHEN implementation tasks are prepared
- THEN the migration MUST be split into independently reviewable slices with explicit dependencies
- AND no slice MUST partially migrate unrelated call sites without a documented boundary

### Requirement: Snapshot Activation MUST Be Rollback-Safe On Partial Failure

Snapshot activation MUST be covered by tests proving that partial activation failures do not leave committed partial state. If the current SQLite transaction behavior already provides rollback, the implementation MAY add only regression tests; if tests expose leakage, the implementation MUST repair transaction handling.

#### Scenario: Partial activation failure rolls back pointer changes

- GIVEN snapshot activation starts inside a SQLite transaction
- AND an injected or simulated failure occurs after partial activation work
- WHEN activation returns an error
- THEN the snapshot pointer state MUST remain as it was before activation
- AND no partially activated snapshot MUST be visible as current

#### Scenario: Successful activation commits atomically

- GIVEN snapshot activation completes without errors
- WHEN the transaction commits
- THEN the new snapshot pointer MUST be visible as current
- AND the previous current pointer MUST no longer be current according to the documented pointer semantics

### Requirement: Snapshot Pointer Rows MUST Track Updated Time

The `snapshot_pointer` persistence model MUST maintain an `updated_at` timestamp through an additive migration and write paths. Existing databases MUST remain loadable after migration, and snapshot pointer updates MUST refresh `updated_at` in a stable format.

#### Scenario: Existing database gains updated_at

- GIVEN a database created before `snapshot_pointer.updated_at` existed
- WHEN migrations run
- THEN the `snapshot_pointer` table MUST contain an `updated_at` column
- AND existing rows MUST receive a valid timestamp value or documented default

#### Scenario: Pointer update refreshes timestamp

- GIVEN a snapshot pointer row exists
- WHEN snapshot activation or pointer update changes the current snapshot
- THEN `updated_at` MUST be set or refreshed
- AND the stored value MUST be parseable by the timestamp boundary used by storage

### Requirement: Creation Timestamps MUST Use DateTime<Utc> In Domain Models Selected For Migration

Public creation timestamp fields selected by this change, including `graph::types::Topic`, `graph::types::Concept`, and `storage::SemanticLinkRow`, MUST use `DateTime<Utc>` instead of raw `String` values. SQLite and CLI boundaries MUST continue to parse and format timestamps explicitly and consistently.

#### Scenario: Storage timestamp parses into DateTime

- GIVEN a storage row contains a valid creation timestamp string
- WHEN it is decoded into a migrated domain record
- THEN the `created_at` field MUST be a `DateTime<Utc>` representing the stored timestamp
- AND callers MUST NOT need ad hoc string parsing to compare or sort the value

#### Scenario: Invalid storage timestamp is rejected

- GIVEN a storage row contains an invalid creation timestamp
- WHEN it is decoded into a migrated domain record
- THEN decoding MUST return an error at the storage boundary
- AND the invalid timestamp MUST NOT be exposed as a domain `DateTime<Utc>` value

#### Scenario: CLI output remains stable

- GIVEN a migrated record with a `DateTime<Utc>` creation timestamp is displayed or emitted as JSON
- WHEN CLI output is generated
- THEN the timestamp MUST be formatted consistently with the previous external contract or a documented new format
- AND tests MUST cover the selected output format

### Requirement: Prior OpenSpec And Archive Reports MUST Be Factually Corrected Concisely

Prior remediation artifacts and archived review reports included in V3 scope MUST be corrected only where verification shows stale or inconsistent factual claims. Corrections MUST be concise, must not rewrite unrelated report content, and MUST distinguish historical findings from current code behavior.

#### Scenario: Prior count inconsistency is corrected

- GIVEN a prior OpenSpec artifact has inconsistent status, wave, commit, or severity totals
- WHEN the inconsistency is verified
- THEN the artifact MUST be updated so summary lines and detailed counts agree
- AND the correction MUST NOT alter unrelated historical context

#### Scenario: Stale archived runtime or UTF-8 claim is corrected

- GIVEN an archived report claims a runtime guard, UTF-8 offset, truncation, character count, or provider API-key bug still exists
- WHEN current code shows the claim is stale or partially stale
- THEN the report MUST be updated to describe current behavior accurately
- AND any remaining risk MUST be stated precisely rather than preserving an obsolete blanket claim

### Requirement: V3 Verification Gate MUST Pass For Each Approved Slice

Each approved implementation slice MUST pass the configured verification commands and update tracking artifacts before it is considered complete. The final change MUST preserve the CLI-first, SQLite-backed durable process model.

#### Scenario: Slice verification passes

- GIVEN an implementation slice has completed
- WHEN verification runs
- THEN `cargo test` MUST pass
- AND `cargo clippy -- -D warnings` MUST pass
- AND `cargo fmt --check` MUST pass

#### Scenario: Tracking remains authoritative

- GIVEN a V3 item is fixed, deferred, or skipped after verification
- WHEN the slice is completed
- THEN the relevant OpenSpec progress or tracking artifact MUST record the item status
- AND skipped or deferred items MUST include a brief reason
