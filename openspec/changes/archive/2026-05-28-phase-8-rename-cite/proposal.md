# Phase 8 Proposal — Rename Harness to CITE

## Why now
Rename the CLI identity before Phase 10+ hierarchy work so new commands and docs are added once under the final name (`cite`). This avoids repeated churn and keeps v0.2.0 slices traceable.

## In scope
- Rename CLI/binary surface from `harness` to `cite`.
- Update Clap command/app name and help output to `cite`.
- Update command examples to `cite` in this canonical doc set:
  - `README.md`
  - `docs/demo.md`
  - `docs/installation.md`
  - `docs/agent-usage-guide.md`
  - `docs/rename-to-cite.md`
- Define and validate a local single-user migration checklist for current `cargo run` usage.
- Confirm Phase 8 sequencing: rename first, hierarchy work later.
- Keep runtime config/data naming untouched in this phase (`HARNESS_*` and existing data paths stay as-is).

## Out of scope
- Hierarchical graph schema/retrieval changes (Phases 10–11).
- Installation/release artifact hardening and installer flow design (Phase 9).
- Release workflow artifact renaming and distribution channel changes (Phase 9).
- Broad multi-user backward-compatibility guarantees.

## Affected areas
- Rust CLI entrypoints and binary naming (`Cargo.toml`, CLI crate Clap metadata).
- Command naming surface in the canonical doc set listed above.
- Validation scripts/commands used in local smoke checks.

## Migration policy (Phase 8)
- **Hard cutover applies only to CLI command identity:** primary invocation becomes `cite`.
- Runtime config/data naming is intentionally deferred: `HARNESS_*` and existing local data/db paths remain unchanged in Phase 8.
- `CITE_*` and data path/db renaming are deferred to Phase 9 (installation/migration phase).
- Migration is handled by explicit local checklist + rollback path, not runtime aliasing.

## Risks and mitigations
- **Risk:** Local scripts break after rename.  
  **Mitigation:** Add explicit migration checklist and smoke commands using `cargo run --bin cite -- --help`.
- **Risk:** Partial rename leaves mixed naming in docs/runtime config expectations.  
  **Mitigation:** Restrict scope to canonical file list, explicitly document deferred runtime naming migration, and run grep verification for `harness` leftovers in the canonical command docs.
- **Risk:** Rename expands beyond review budget.  
  **Mitigation:** Deliver in small slices; each slice must declare a changed-file allowlist and stay under 300 changed lines.

## Rollback plan
- Revert Phase 8 rename commits to restore `harness` binary/doc surface.
- Runtime config/data names remain unchanged in Phase 8, so rollback focuses on CLI/doc rename only.
- If breakage appears mid-phase, pause and ship a minimal repair patch before continuing.

## Acceptance criteria
- CLI is invocable as `cite` in local workflow (`cargo run --bin cite -- ...`).
- Help/usage output presents `cite` (not `harness`) as primary command name.
- Canonical docs (`README.md`, `docs/demo.md`, `docs/installation.md`, `docs/agent-usage-guide.md`, `docs/rename-to-cite.md`) show `cite` command examples.
- Migration policy is explicit: CLI hard cutover now, runtime naming migration deferred to Phase 9; checklist is validated on the local setup.
- Phase explicitly preserves ordering: rename complete before hierarchical graph implementation starts.

## Proposed slices (small, traceable)
1. **Slice A — CLI identity rename:** binary + Clap naming, compile and help check.  
   Allowlist: `Cargo.toml`, `crates/cli/**`.
2. **Slice B — Docs command surface:** canonical doc set examples updated to `cite`.  
   Allowlist: `README.md`, `docs/demo.md`, `docs/installation.md`, `docs/agent-usage-guide.md`, `docs/rename-to-cite.md`.
3. **Slice C — Local migration notes:** checklist for local script updates and explicit note that env/data renames are deferred to Phase 9.  
   Allowlist: `docs/sdd/phase-8-rename-cite/migration-checklist.md` (+ optional links from `docs/installation.md`).
4. **Slice D — Verification pass:** grep/smoke validation and closeout notes for Phase 8 acceptance.  
   Required checks:
   - `cargo run --bin cite -- --help`
   - `cargo test`
   - `rg -n "harness\s+(context|search|retrieve|ingest|list|get|trace|read|evaluate|refresh|retry)" README.md docs/demo.md docs/installation.md docs/agent-usage-guide.md docs/rename-to-cite.md`
   - `rg -n "CITE_|HARNESS_" docs/sdd/phase-8-rename-cite/migration-checklist.md docs/installation.md`
   Each slice stays under 300 changed lines.
