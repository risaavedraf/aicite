# Error Remediation V2 Specification

## Purpose

This change defines the expected behavior after the second remediation pass for the remaining medium and low severity quality issues plus the unchecked integer casts discovered after the first pass. The remediation MUST be grouped by theme, MUST preserve working behavior, and MUST keep each review slice under the configured review budget.

## Requirements

### Requirement: Theme-Based Remediation Scope

The system MUST remediate the second-pass inventory by theme rather than by crate. The prioritized theme order MUST be: DRY refactoring, test infrastructure, type consistency and cast safety, dead code cleanup, naming and documentation, then newtype migration planning or an explicitly bounded incremental newtype slice.

#### Scenario: Themes are ordered for execution

- GIVEN the second-pass inventory contains DRY, test, type, dead code, naming, and newtype issues
- WHEN the remediation plan is reviewed
- THEN issues MUST be grouped under thematic work areas instead of per-crate buckets
- AND the themes MUST preserve the requested priority order unless a dependency is explicitly documented

#### Scenario: Cross-crate issues remain cohesive

- GIVEN one theme affects multiple crates
- WHEN that theme is specified for implementation
- THEN the acceptance criteria MUST describe the complete behavioral expectation for the theme
- AND it MUST NOT split the same concern into unrelated crate-only requirements

### Requirement: Reviewable Delivery Slices

Each implementation slice for this change MUST remain below 400 changed lines and MUST use ask-always delivery decisions before apply work begins. If a theme is forecast to exceed the review budget, the theme MUST be split into independently reviewable slices with explicit dependencies.

#### Scenario: Oversized theme is split

- GIVEN a theme is forecast above 400 changed lines
- WHEN tasks are prepared for apply
- THEN the theme MUST be divided into smaller review slices
- AND each slice MUST identify its prerequisite slices, if any

#### Scenario: Delivery decision is required before apply

- GIVEN the configured chained PR strategy is ask-always
- WHEN the spec/design/tasks phases forecast chained or oversized work
- THEN apply MUST NOT begin until the user approves the delivery strategy

### Requirement: DRY Error Handling and Validation

The CLI MUST present command errors and mutually exclusive retrieval flag validation through consistent behavior. Repeated error display branches MUST result in the same exit code, JSON shape, and human-readable stderr message for the same underlying error.

#### Scenario: JSON and text error outputs remain equivalent

- GIVEN two CLI commands encounter the same underlying error
- WHEN one command is run with JSON output and another without JSON output
- THEN the JSON response MUST include the same error identity and exit code semantics
- AND the text response MUST describe the same error to the user

#### Scenario: Retrieval flags are validated consistently

- GIVEN search, retrieve, and context commands expose mutually exclusive retrieval scope flags
- WHEN a user supplies an invalid combination
- THEN each command MUST reject the combination with equivalent error semantics
- AND no command MUST silently choose one conflicting scope over another

### Requirement: Shared Test Infrastructure and Deterministic Fixtures

Golden fixtures, provider test doubles, and network-sensitive tests MUST be deterministic and shared where behavior is shared. Tests MUST NOT depend on external network availability unless explicitly marked as ignored or integration-only.

#### Scenario: Golden expectations are canonical

- GIVEN CLI evaluation tests and engine golden tests validate the same fixture behavior
- WHEN a fixture expectation changes
- THEN there MUST be one canonical expected representation or one documented source from which equivalent expectations are derived
- AND contradictory expectations for the same fixture MUST NOT remain

#### Scenario: Provider tests avoid accidental network dependency

- GIVEN provider tests exercise invalid keys or endpoints
- WHEN the default test suite runs with `cargo test`
- THEN those tests MUST NOT require live external network access
- AND network-dependent checks MUST be ignored, mocked, or explicitly marked as integration tests

#### Scenario: Regression coverage covers edge cases

- GIVEN retrieval ranking, cosine similarity, config merging, and text offset behavior have known edge cases
- WHEN the test suite is extended
- THEN it MUST include representative boundary cases for empty inputs, invalid candidates, merge precedence, and non-ASCII text where applicable

### Requirement: Type Consistency and Cast Safety

The system MUST avoid silent lossy conversion between storage integers and domain integer types. Remaining unchecked numeric casts from database rows to `u32` MUST report an error on negative or overflowing values instead of truncating.

#### Scenario: Overflowing storage value is rejected

- GIVEN a database row contains an integer value outside the `u32` range for a field represented as `u32`
- WHEN the row is decoded into a domain record
- THEN decoding MUST fail with a storage error
- AND the decoded record MUST NOT contain a truncated value

#### Scenario: Valid storage value is preserved

- GIVEN a database row contains an integer value within the `u32` range
- WHEN the row is decoded into a domain record
- THEN the decoded field MUST equal the original value

### Requirement: Temporal Types Are Consistent

Timestamp fields representing creation time SHOULD use a consistent time representation across graph, storage, and common domain records. If a field remains a string for persistence or compatibility reasons, that exception MUST be documented in the relevant model or mapping behavior.

#### Scenario: Domain creation time is comparable

- GIVEN graph or storage records expose `created_at`
- WHEN tests compare two records or sort records by creation time
- THEN the timestamp representation SHOULD support deterministic comparison without ad hoc parsing in every caller

#### Scenario: String timestamp exception is documented

- GIVEN a `created_at` field remains a string
- WHEN the model is reviewed
- THEN the reason for string representation MUST be explicit
- AND conversion boundaries MUST be test-covered where applicable

### Requirement: Dead Code Is Removed or Justified

Public or internal items that are not used by production code or tests MUST either be removed or given a documented purpose and coverage. Placeholder structs, unused domain types, unused conversion helpers, and unused dependencies MUST NOT remain solely as unexplained dead code.

#### Scenario: Placeholder type has no behavior

- GIVEN a unit struct or placeholder type has no state, methods, or current callers
- WHEN dead code cleanup runs
- THEN it MUST be removed unless a documented compatibility reason requires it to remain

#### Scenario: Allowed dead code is justified

- GIVEN an item retains an allow-dead-code attribute
- WHEN the code is reviewed
- THEN the item MUST have a clear rationale for remaining
- AND the rationale MUST identify the expected future or compatibility use

### Requirement: Naming and Documentation Consistency

Names and public documentation MUST accurately describe behavior, units, and operational scope. Misleading names, imprecise deprecation warnings, and undocumented public behavior MUST be corrected or explicitly documented.

#### Scenario: Behavior matches user-facing name

- GIVEN a command, flag, function, or field name suggests a specific behavior
- WHEN the behavior is exercised
- THEN the observed behavior MUST match the name
- OR documentation MUST clearly explain the narrower behavior

#### Scenario: Public APIs are understandable

- GIVEN a public enum, struct, trait, or command behavior is exposed to users or downstream crates
- WHEN generated documentation or help text is reviewed
- THEN it SHOULD explain the purpose and relevant constraints without relying on source-code archaeology

### Requirement: Newtype Migration Is Separately Scoped

The `DocumentId`, `ChunkId`, and `TraceId` newtype migration MUST NOT be bundled as an unbounded cross-repository rewrite in this pass. The change MUST either define a small, reviewable incremental adoption slice or defer the full migration to a separate SDD change.

#### Scenario: Incremental newtype adoption is bounded

- GIVEN a newtype adoption slice is included in this change
- WHEN tasks are prepared
- THEN the slice MUST identify the exact boundary where raw strings are converted to typed IDs
- AND the slice MUST stay below the review budget

#### Scenario: Full migration is deferred

- GIVEN full newtype migration would affect approximately 50 files
- WHEN this change is scoped for implementation
- THEN the full migration MUST be documented as a separate SDD effort
- AND this pass MUST NOT partially migrate unrelated call sites without a clear boundary

### Requirement: Verification Gate

The remediation MUST pass the configured quality gate after each approved implementation slice: `cargo test`, `cargo clippy -- -D warnings`, and `cargo fmt --check`. The error tracking report MUST be updated to show which second-pass items were fixed, deferred, or found to be false alarms.

#### Scenario: Slice verification passes

- GIVEN an implementation slice has completed
- WHEN verification runs
- THEN `cargo test` MUST pass
- AND `cargo clippy -- -D warnings` MUST pass
- AND `cargo fmt --check` MUST pass

#### Scenario: Tracking remains authoritative

- GIVEN a second-pass item is fixed, deferred, or invalidated
- WHEN verify completes
- THEN `openspec/reports/error-tracking.md` MUST reflect the updated status
- AND any false alarm or deferral MUST include a reason
