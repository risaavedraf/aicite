# Phase 9 Proposal — Installation Experience for CITE

## Why now

Phase 8 completed CLI rename to `cite` and deferred runtime naming migration. Before hierarchy work (Phases 10-12), installation and migration paths must be explicit, reproducible, and release-safe.

## In scope

- Define canonical install/run pathways for `cite`:
  - local dev (`cargo run --bin cite -- ...`)
  - built local binary (`target/release/cite`)
  - GitHub release binary naming/usage (`cite-*` artifacts)
- Align installation docs and command examples to the canonical pathways.
- Finalize Phase 9 policy for runtime naming migration deferred by Phase 8:
  - environment variables (`CITE_*`)
  - local config/data path expectations
  - migration/rollback checklist for current local users
- Add auditable verification evidence for installation flows and migration expectations.

## Out of scope

- Hierarchical graph schema/ingest/retrieval behavior (Phases 10-11).
- Agent UX/result formatting enhancements (Phase 12).
- New retrieval semantics unrelated to installation/migration.
- Broad enterprise multi-tenant installer support.

## Affected areas

- Documentation: `README.md`, `docs/installation.md`, `docs/demo.md`, `docs/agent-usage-guide.md`.
- Runtime/config crates and storage path handling (only if approved in spec scope).
- Release workflow consistency checks.
- Phase artifacts: `docs/sdd/phase-9-installation-experience/*`, `openspec/changes/phase-9-installation-experience/*`.

## Risks and mitigations

- **Risk:** Runtime-name migration can break existing local setups.
  **Mitigation:** Explicit migration policy + rollback steps + verify commands.

- **Risk:** Scope creep into architecture work.
  **Mitigation:** Hard boundary to installation/migration concerns only.

- **Risk:** Review overload.
  **Mitigation:** Slice plan with file allowlists and budget 300 lines per PR segment.

## Rollback plan

- Revert Phase 9 commits to restore previous install docs/paths.
- Keep migration path documented so users can return to prior env/config names if needed.
- If partial migration lands, require a repair patch before continuing to Phase 10.

## Acceptance criteria

1. Canonical `cite` installation/run paths are documented and executable locally.
2. Release artifact naming and usage docs are consistent with produced binaries.
3. Runtime naming migration policy is explicit (what changes now vs what remains compatible).
4. Migration checklist includes validate/rollback commands.
5. Verify report can demonstrate installation flow and migration behavior with command evidence.

## Proposed slices (for tasks phase)

1. **Slice A — Install path canonicalization** (docs + command checks)
2. **Slice B — Runtime naming/data migration** (policy + optional implementation)
3. **Slice C — Verification + closeout evidence** (auditable command logs)
