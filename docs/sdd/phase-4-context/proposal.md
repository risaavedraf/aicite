# SDD Proposal — Phase 4: Context Packs + Citations

## Change name

`phase-4-context` — Agent-consumable context packs with citations and traceable evidence

## Problem

After Phase 3, retrieval can rank chunks but the MVP still lacks its core agent-facing artifact: a stable context pack with explicit result-kind semantics, inspectable citations, and deterministic trace/read behavior.

## Proposed change

Implement Phase 4 context contracts end-to-end: `harness context`, `harness read`, and `harness trace`, including result-kind decisions, citation/trace persistence, metadata/disclaimer requirements, and contract-aligned error handling.

## In scope

- Context-pack assembly with:
  - `context_pack_id`, `result_kind`, `query_id`, `trace_id`, `instructions`, `citations[]`, `metadata`
- Result-kind decision table implementation:
  - `no_results`, `insufficient_context`, `context`
- Citation contract fields for context/read/trace usage
- `harness context` command (JSON + human output)
- `harness read` command with scoped selectors:
  - `--citation-id` + `--trace-id`
  - `--chunk-id` + `--document-id`
- `harness trace` command returning traceability envelope
- Verification disclaimer and agent instruction template
- Partial-corpus metadata in context outputs
- Tests for selector validation, result-kind thresholds, and trace/citation lookup

## Out of scope

- Durable retrieval/context rate limiting (Phase 5)
- Durable lock conflict UX/backlog expansion (Phase 5)
- Golden dataset and quality benchmark automation (Phase 6)
- Packaging and demo release workflow (Phase 7)
- Built-in answer generation over context packs

## Acceptance criteria

1. `harness context "<query>" --json` returns `{ context_pack_id, result_kind, query_id, trace_id, instructions, citations, metadata }` with disclaimer.
2. Result-kind logic follows threshold table:
   - below floor => `no_results` and `citations: []`
   - weak/partial evidence => `insufficient_context`
   - sufficient evidence => `context` with supporting citations.
3. `harness read` enforces mutually exclusive selectors and required scoping; invalid combos return `invalid_parameter`.
4. `harness read --citation-id <id> --trace-id <id>` resolves deterministic citation evidence or returns `citation_not_found`.
5. `harness read --chunk-id <id> --document-id <id>` returns ready-snapshot chunk only; stale/non-ready paths return `chunk_not_found` or `document_not_ready`.
6. `harness trace <trace_id> --json` returns required trace metadata including citation IDs and ranking metadata.
7. Context/read/trace outputs respect safe-field and redaction constraints.
8. Partial-corpus behavior is explicit in metadata when some docs are non-ready, including excluded non-ready counts and IDs.
9. `harness trace` includes `responsible_owner` required in production/team mode and nullable/omittable in local/private mode.
10. `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` pass.

## Estimated size

~900–1300 lines including migrations, engine/CLI wiring, DTOs, and tests.

Given a 400-line review budget, delivery should be split into 3 chained slices.
