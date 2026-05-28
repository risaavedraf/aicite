# CLI Identity Specification

## Purpose

Define the Phase 8 CLI identity cutover from `harness` to `cite`, align canonical command-facing documentation, and lock scope boundaries before Phase 9 runtime naming migration.

## Requirements

### Requirement: CLI command identity MUST cut over to `cite`

The system MUST present `cite` as the primary CLI command identity for Phase 8 usage and help output.

#### Scenario: Local CLI help uses `cite`

- GIVEN a local development workflow for this repository
- WHEN the user runs `cargo run --bin cite -- --help`
- THEN the help `Usage:` line starts with `cite`
- AND `harness` is not presented as the primary command identity

### Requirement: Canonical command documentation MUST use `cite`

Phase 8 canonical documentation MUST show command examples with `cite` in the following files:
- `README.md`
- `docs/demo.md`
- `docs/installation.md`
- `docs/agent-usage-guide.md`
- `docs/rename-to-cite.md`

#### Scenario: Canonical docs show renamed command surface

- GIVEN the Phase 8 canonical documentation set
- WHEN a reviewer checks command examples in the listed files
- THEN command invocations use `cite` as the CLI name

### Requirement: Runtime naming migration MUST be deferred to Phase 9

Phase 8 MUST NOT rename runtime configuration and data-path naming. `HARNESS_*` runtime naming and existing local data/database paths SHALL remain in place during Phase 8. `CITE_*` runtime/env naming and data-path/database renaming SHALL be handled in Phase 9.

#### Scenario: Runtime naming remains unchanged in Phase 8

- GIVEN a Phase 8 implementation branch
- WHEN a reviewer runs `rg -n "HARNESS_" crates/config crates/storage`
- THEN at least one `HARNESS_` runtime naming reference is present
- AND WHEN the reviewer runs `rg -n "CITE_" crates/config crates/storage`
- THEN no runtime/env naming references are found in those crates
- AND migration docs state this runtime rename is deferred to Phase 9

### Requirement: Phase 8 verification MUST include the defined commands and outcomes

Phase 8 verification MUST include execution of the following commands and expected outcomes:
- `cargo run --bin cite -- --help` → help output shows `cite` as primary command name
- `cargo test` → test suite passes
- `rg -n "harness\s+(context|search|retrieve|ingest|list|get|trace|read|evaluate|refresh|retry)" README.md docs/demo.md docs/installation.md docs/agent-usage-guide.md docs/rename-to-cite.md` → no command-example hits remain in canonical docs
- `rg -n "CITE_|HARNESS_" docs/sdd/phase-8-rename-cite/migration-checklist.md docs/installation.md` → output reflects explicit deferral policy (HARNESS runtime naming now, CITE runtime naming in Phase 9)

#### Scenario: Verification checklist is auditable

- GIVEN Phase 8 closeout review
- WHEN the reviewer inspects verification evidence
- THEN evidence includes each required verification command
- AND evidence includes whether each expected outcome passed or failed
