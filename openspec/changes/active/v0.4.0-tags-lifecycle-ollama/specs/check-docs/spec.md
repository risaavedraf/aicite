# Check Docs Specification

## Purpose

Allow documentation verification to understand markdown tag annotations for planned commands so future-facing examples are not reported as outdated implementation evidence.

## Requirements

### Requirement: Markdown Tag Comment Parsing

The system MUST parse markdown tag comments of the form `<!-- tag:key=value -->` when evaluating command examples in `check-docs`. At minimum, the system MUST recognize `<!-- tag:status=planned -->` as applying to the associated planned command example.

#### Scenario: Planned command annotation

- GIVEN a markdown document contains `<!-- tag:status=planned -->` immediately associated with a Cite command example
- WHEN `cite check-docs` verifies the document
- THEN the command example MUST be treated as planned
- AND it MUST NOT be reported as an outdated implemented command solely because the command is unavailable

### Requirement: Default Verification for Untagged Commands

The system MUST preserve existing `check-docs` verification behavior for command examples that do not have a recognized planned status tag.

#### Scenario: Untagged command remains verified

- GIVEN a markdown document contains a Cite command example without a recognized status tag annotation
- WHEN `cite check-docs` verifies the document
- THEN the command MUST be verified using the existing command verification behavior

### Requirement: Implemented Status Does Not Skip Verification

The system MUST NOT skip command verification for examples annotated as implemented. `status:implemented` MUST mean the example is expected to reflect available behavior.

#### Scenario: Implemented command is verified

- GIVEN a markdown command example is associated with `<!-- tag:status=implemented -->`
- WHEN `cite check-docs` verifies the document
- THEN the command example MUST be verified
- AND failures MUST be reported according to the existing check-docs reporting model

### Requirement: Unknown Markdown Tags Are Non-Destructive

The system SHOULD ignore unknown markdown tag annotations for verification decisions unless a later specification defines behavior for them. Unknown tags MUST NOT cause command examples to be skipped as planned.

#### Scenario: Unknown tag does not skip

- GIVEN a markdown command example is associated with `<!-- tag:priority=low -->`
- WHEN `cite check-docs` verifies the document
- THEN the command example MUST NOT be skipped because of that unknown tag
