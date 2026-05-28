# Apply Progress — phase-8-rename-cite

## Completed tasks

- [x] Slice A: renamed CLI binary target from `harness` to `cite`.
- [x] Slice A: updated Clap command name to `cite`.
- [x] Slice A: verified local help invocation via `cargo run --bin cite -- --help`.
- [x] Slice B: updated canonical command-facing docs to use `cite`:
  - `README.md`
  - `docs/demo.md`
  - `docs/installation.md`
  - `docs/agent-usage-guide.md`
  - `docs/rename-to-cite.md`
- [x] Slice C: created local migration checklist with explicit runtime deferral to Phase 9.
- [x] Slice D: executed full verification suite and recorded auditable evidence.

## Files changed

- `crates/cli/Cargo.toml`
- `crates/cli/src/main.rs`
- `README.md`
- `docs/demo.md`
- `docs/installation.md`
- `docs/agent-usage-guide.md`
- `docs/rename-to-cite.md`
- `docs/sdd/phase-8-rename-cite/migration-checklist.md` (new)
- `docs/sdd/phase-8-rename-cite/verification-evidence.md` (new)
- `openspec/changes/phase-8-rename-cite/apply-progress.md` (new)

## Test / verify commands run

- `cargo run --bin cite -- --help`
- `cargo test`
- `rg -n "harness\s+(context|search|retrieve|ingest|list|get|trace|read|evaluate|refresh|retry)" README.md docs/demo.md docs/installation.md docs/agent-usage-guide.md docs/rename-to-cite.md`
- `rg -n "HARNESS_" crates/config crates/storage`
- `rg -n "CITE_" crates/config crates/storage`
- `rg -n "CITE_|HARNESS_" docs/sdd/phase-8-rename-cite/migration-checklist.md docs/installation.md`

## Deviations from design

- None.

## Remaining tasks

- [ ] Run Phase 8 `sdd-verify` gate.
- [ ] Optional: broaden docs grep to `\bharness\b` for non-subcommand leftovers in command examples.

## Workload / PR boundary

- Delivery followed chained slices A → B → C → D.
- No chained PR requirement was enforced by forecast (`Low` risk), but slice boundaries were preserved.
