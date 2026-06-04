# Phase 9 — Installation Experience (Explore Notes)

## Context

Phase 8 is closed and archived (`phase-8-rename-cite`). CLI identity is now `cite`.
The next risk is installation/distribution consistency across local dev and release usage.

## Session preflight (confirmed)

- Execution mode: interactive
- Artifact store: OpenSpec + Engram
- PR strategy: ask always
- Review budget: 300 changed lines

## Problem to solve

Today there are still installation-path ambiguities after the rename:

1. Local usage variants are not standardized enough (`cargo run`, built binary, install paths).
2. Release artifact identity must stay fully aligned with `cite`.
3. Runtime naming migration (`CITE_*`/legacy paths) was explicitly deferred by Phase 8 and needs a controlled plan.

## Goal for Phase 9

Define and implement a reproducible installation experience for `cite` with explicit migration guidance and verification commands.

## Candidate slices

1. **Slice A — Install path matrix**
   - Canonicalize supported ways to run/install (`cargo run`, release binary, optional local install path).
   - Document OS-specific examples where needed.

2. **Slice B — Runtime naming migration policy**
   - Decide Phase 9 boundary for `CITE_*` -> `CITE_*` and data path handling.
   - Define backward-compat expectations (read-old/write-new or explicit manual migration).

3. **Slice C — Release/install consistency checks**
   - Validate release workflow naming and docs references.
   - Add/refresh verification evidence commands.

## Risks

- Breaking existing local setup unexpectedly while migrating runtime names.
- Mixed docs if install commands and env vars diverge.
- Over-scoping into Phase 10+ architecture work.

## Mitigations

- Keep Phase 9 focused on installation/migration only.
- Use explicit allowlists per slice and review budget guard.
- Require auditable verify commands for each migration decision.
