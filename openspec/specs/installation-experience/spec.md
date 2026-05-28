# Installation Experience Specification (Phase 9)

## Purpose

Define a reproducible installation and runtime migration experience for `cite` before Phase 10+ hierarchy work.

## Requirements

### Requirement: Canonical local run/install pathways MUST be explicit and reproducible

Phase 9 documentation MUST define canonical commands for:
- local dev execution (`cargo run --bin cite -- ...`)
- local built binary (`./target/release/cite ...`)
- release-downloaded binary usage (`cite` / `cite.exe` after install)

Canonical pathways MUST be consistent across:
- `README.md`
- `docs/installation.md`
- `docs/demo.md`
- `docs/agent-usage-guide.md`

#### Scenario: Reviewer can follow one unambiguous run matrix

- GIVEN a clean local environment
- WHEN a reviewer follows the documented pathways
- THEN each pathway executes with the documented command style
- AND docs do not present conflicting invocation patterns for the same pathway

### Requirement: Release artifact naming and usage MUST stay consistent with published binaries

Phase 9 docs and release references MUST use one artifact naming convention aligned with actual release outputs.

#### Scenario: Download instructions match release assets

- GIVEN a tagged release
- WHEN a reviewer compares docs install URLs/examples with release assets
- THEN each documented artifact name exists in the release
- AND post-download usage examples match the produced executable names

### Requirement: Runtime naming migration policy MUST be explicit and internally consistent

Phase 9 MUST resolve Phase 8 deferral ambiguity by documenting:
- canonical runtime env-var namespace for current releases
- canonical data directory and database naming for current releases
- backward-compatibility policy for legacy runtime names/paths (supported aliases vs manual migration)

Docs MUST avoid contradictory placeholders or self-mapping statements (for example `X -> X` when a migration is intended).

#### Scenario: Runtime policy is understandable and actionable

- GIVEN a current user setup and a legacy user setup
- WHEN they read the migration section
- THEN both can identify which runtime names/paths are canonical now
- AND both can identify how to validate, migrate, or keep compatibility safely

### Requirement: Migration checklist MUST include validation and rollback commands

Phase 9 MUST provide a checklist that includes:
- pre-migration checks
- migration steps (if required)
- validation commands and expected outcomes
- rollback steps restoring previous working state

#### Scenario: User can migrate and recover locally

- GIVEN a user performing runtime/install migration
- WHEN they execute the checklist
- THEN they can verify success with explicit commands
- AND they can restore prior state using documented rollback commands

### Requirement: Verification evidence MUST be auditable

Phase 9 verify evidence MUST record command outputs and pass/fail outcomes for:
- local run pathway checks
- release artifact consistency checks
- runtime naming policy checks
- migration/rollback validation checks

#### Scenario: Acceptance criteria can be audited from evidence

- GIVEN Phase 9 closeout review
- WHEN a reviewer inspects verification evidence artifacts
- THEN each required check is present with command + outcome
- AND acceptance status is derivable without hidden assumptions
