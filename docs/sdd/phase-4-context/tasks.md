# SDD Tasks — Phase 4: Context Packs + Citations

## Overview

| # | Task | Depends on | Est. lines |
|---|---|---|---|
| 1 | Storage schema + trace/citation persistence APIs | — | ~320 |
| 2 | Engine context assembly + result-kind logic | 1 | ~360 |
| 3 | CLI commands: context/read/trace + output contracts | 1,2 | ~320 |
| 4 | Contract tests + verification pass | 1-3 | ~220 |

Total estimated diff: ~1220 lines (before refactors).

## Chained slice proposal (review budget: 400 lines)

### Slice 1 — Storage + contracts foundation (~320 lines)

Scope:
- Migration updates for trace/citation persistence.
- Storage read/write APIs for trace/citation lookup.
- Common DTO groundwork for context/trace/read outputs.
- Unit tests for persistence lookups and readiness-scoped chunk resolution.

Review target: under 400 lines.

### Slice 2 — Engine context logic (~360 lines)

Scope:
- `engine::context` orchestration.
- `result_kind` threshold decision function.
- context metadata/disclaimer assembly.
- trace record creation and citation linkage.
- Engine-level tests for threshold table and readiness semantics.

Review target: under 400 lines.

### Slice 3 — CLI surface + end-to-end contract tests (~320 + tests)

Scope:
- Add `harness context`, `harness read`, `harness trace` commands.
- JSON/human output contract wiring.
- Selector validation and error-mapping checks.
- Integration tests for command contract behavior.

Review target: keep code slice under 400 lines; tests may be split in follow-up commit if needed.

## Task details

### 1) Storage foundation
- Extend migrations for trace/citation records.
- Add storage methods:
  - persist trace header + citations
  - get citation by trace scope
  - get chunk by document scope (ready snapshot only)
  - get trace envelope by trace ID
  - return excluded non-ready document IDs for context metadata
- Add tests for missing/ambiguous/stale resolution behavior.

### 2) Engine context logic
- Add context pack builder with instruction template and disclaimer.
- Implement threshold evaluator for `no_results` / `insufficient_context` / `context`.
- Implement deterministic facet heuristic (`required_facets` / `covered_facets`).
- Implement `insufficient_context` marking fields (`confidence_label`, `insufficient_context_reason`, caution text).
- Implement partial-corpus metadata reporting (counts + excluded document IDs).
- Persist trace/citation evidence at context creation time.

### 3) CLI command surface
- Add `context`, `read`, `trace` commands and args.
- Enforce mutually exclusive read selectors.
- Ensure stable JSON keys and contract-compliant human output.
- Enforce `responsible_owner` output rule by runtime mode (required in production/team, nullable in local/private).

### 4) Verify
- Run `cargo test`
- Run `cargo clippy -- -D warnings`
- Run `cargo fmt --check`
- Fix regressions and confirm error code/exit code alignment.
- Validate acceptance checks for excluded non-ready IDs and insufficient-context marking.

## Risks to track during apply

1. Trace↔citation persistence ambiguity if schema is under-specified.
2. Semantic drift between `document_not_ready` and `no_results` behavior.
3. Redaction leakage via provider/storage error details.
4. Review budget creep if slice boundaries are not enforced.

## Delivery decision

Proceed with chained PR slices by default (auto-forecast + 400-line budget). If any slice exceeds budget, split again before review.
