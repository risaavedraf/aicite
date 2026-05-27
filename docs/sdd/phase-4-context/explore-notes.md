# Explore Notes — Phase 4: Context Packs + Citations

## Current state

Phase 3 left the system with:
- Vector-first retrieval (`search` / `retrieve`) over ready documents.
- Ranked chunk outputs with metadata and scores.
- No formal context-pack assembly contract implemented in code.
- No complete `context`, `read`, and `trace` command surface.
- Existing `traces` table is not yet sufficient for full citation/read/trace contract fidelity.

## What Phase 4 must deliver

From roadmap and PRD contracts:
1. Context-pack assembly (`context_pack_id`, `result_kind`, `trace_id`, citations, metadata)
2. Canonical `result_kind` behavior (`context`, `no_results`, `insufficient_context`)
3. Citation model for agent-consumable evidence
4. `harness context` command (JSON + human output)
5. `harness read` command (scoped citation/chunk lookup)
6. `harness trace` command (traceability envelope)
7. Agent instruction template + verification disclaimer
8. Partial-corpus metadata and ready-snapshot behavior

## Key design choices

- **Persistence strategy (MVP)**: extend storage to support deterministic trace↔citation lookup for `read --citation-id --trace-id` and `trace` output without reconstructing from transient runtime state.
- **Result-kind decision**: centralize evidence-threshold logic in engine so `context` behavior is consistent and testable.
- **Selector semantics** (`read`): enforce mutually exclusive selector modes and explicit scope (`trace_id` or `document_id`).
- **Safety/redaction**: enforce safe-field output and avoid leaking provider payloads, raw paths, and full unsafe internals in error/log details.
- **Reviewability**: implement in 3 chained slices to respect the 400-line review budget.

## Known risks

- Contract mismatch risk between current retrieval empty-results behavior and `document_not_ready` / `result_kind` semantics.
- Incomplete trace/citation persistence can break deterministic `read` and `trace` outputs.
- Redaction/logging drift can leak sensitive fields through error details or provider failures.
- Scope creep into durability/rate-limits (Phase 5) if boundaries are not kept explicit.

## Non-goals

- Durable retrieval/context rate limiting (Phase 5)
- Durable lock/backlog refresh semantics beyond current baseline (Phase 5)
- Golden dataset evaluation pipeline (Phase 6)
- Built-in answer-generation adapter (post-MVP)
