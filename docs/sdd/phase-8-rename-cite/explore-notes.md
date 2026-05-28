# Phase 8 — Rename Harness → CITE (Explore Notes)

## Context

v0.1.0 is complete and tagged. The next milestone (v0.2.0) should be executed as multiple traceable phases, not a single monolithic version task.

User preference for this session:
- Single current user, local usage via `cargo run`
- Prioritize rename first to avoid renaming new commands later
- Favor traceability over speed (small phases/slices)

## Session preflight (confirmed)

- Execution mode: interactive
- Artifact store: OpenSpec + Engram
- PR strategy: ask always
- Review budget: 300 changed lines

## Why rename first

1. Prevent double work across upcoming CLI additions (`topics`, `links`, hierarchy flags)
2. Align docs, command UX, and release identity before major architecture work
3. Keep hierarchy implementation focused on domain behavior, not naming churn

## Proposed v0.2 phase map (milestone-driven)

- Phase 8: Rename to CITE (binary, clap name, docs baseline, local migration)
- Phase 9: Installation flows (local + release install docs and scripts)
- Phase 10: Hierarchical graph foundation (schema + ingest hierarchy)
- Phase 11: Hierarchical retrieval + CLI surface (context/search/retrieve integration)
- Phase 12: Agent UX improvements + evaluation + release readiness

v0.2.0 tag is cut only after Phase 12 acceptance is green.

## Initial scope for Phase 8

In:
- CLI binary rename (`harness` -> `cite`)
- Clap command name and help examples
- Core docs command examples (`README.md`, `docs/demo.md`, `docs/installation.md`, `docs/agent-usage-guide.md`)
- Local config naming migration policy for single-user setup

Out (deferred):
- Full backward-compat matrix for external users
- Cross-platform installer hardening (Phase 9)
- Hierarchical graph behavior changes (Phase 10+)

## Risks

- Breaking local scripts/aliases silently
- Inconsistent env var names across crates/docs
- Release workflow artifact names lagging rename

## Mitigations

- Add explicit migration checklist for local environment
- Keep one-time compatibility read path for old names where cheap
- Validate command surface with `cargo run --bin cite -- --help` and smoke tests
