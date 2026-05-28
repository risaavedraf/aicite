# Design — Phase 8 Rename `harness` → `cite`

Phase 8 performs a **CLI identity cutover only**: command name/help/docs move to `cite`, while runtime naming (`HARNESS_*`, `harness` data/db paths) is intentionally preserved for Phase 9.

## Quick path

1. Rename CLI binary + Clap app name to `cite`.
2. Update the canonical command-doc set to use `cite` examples.
3. Add a local migration checklist documenting script updates and explicit Phase 9 deferrals.
4. Run required verification commands and record outcomes.

## Scope and boundaries

### In scope (Phase 8)
- CLI/binary identity (`cargo run --bin cite`, help usage shows `cite`).
- Canonical docs command examples in:
  - `README.md`
  - `docs/demo.md`
  - `docs/installation.md`
  - `docs/agent-usage-guide.md`
  - `docs/rename-to-cite.md`
- Local single-user migration checklist + rollback notes.
- Verification evidence for spec-required commands.

### Explicitly deferred to Phase 9
- Runtime env var rename (`HARNESS_*` → `CITE_*`).
- Data directory/database path rename (`harness/`, `harness.db` naming migration).
- Installer/release artifact naming hardening.

## Architecture / decision log

| ID | Decision | Why |
|---|---|---|
| AD-1 | Hard cutover for command identity: primary invocation is `cite`. | Avoids repeated churn before later phases add more CLI surface. |
| AD-2 | No runtime alias layer in Phase 8 (`harness` alias/symlink not required). | Keeps scope small and reviewable; migration is checklist-based. |
| AD-3 | Runtime naming remains `HARNESS_*` and existing data/db paths in this phase. | Matches approved proposal/spec boundary; prevents risky storage/config migration now. |
| AD-4 | Canonical docs only are updated in Phase 8; broader historical/infra docs wait for later phases. | Minimizes blast radius and keeps slices under 300 changed lines. |

## Data flow impact

### CLI invocation flow after Phase 8
`user command (cite ...)` → Cargo bin target `cite` (`crates/cli/Cargo.toml`) → Clap command metadata (`crates/cli/src/main.rs`) emits help/usage with `cite` → existing command handlers unchanged.

### Runtime/config flow (unchanged in Phase 8)
`Config::load` + runtime env/data resolution remain as-is (`HARNESS_*`, current data-dir/db naming). This is a deliberate compatibility hold until Phase 9.

## File-level design

### Slice A — CLI identity rename (<=300 changed lines)
**Goal:** make local invocation/help identity `cite`.

**Allowlist**
- `crates/cli/Cargo.toml`
- `crates/cli/src/main.rs`

**Planned changes**
- Rename `[[bin]] name` from `harness` to `cite`.
- Update Clap `#[command(name = "harness", ...)]` to `cite`.
- Keep runtime path/env naming logic untouched (no `HARNESS_*` edits).

**Validation (immediate)**
- `cargo run --bin cite -- --help`
  - Expected: `Usage:` starts with `cite`.
  - Expected: `harness` is not primary app name in help header/usage.

---

### Slice B — Canonical docs command surface (<=300 changed lines)
**Goal:** command examples in canonical docs use `cite`.

**Allowlist**
- `README.md`
- `docs/demo.md`
- `docs/installation.md`
- `docs/agent-usage-guide.md`
- `docs/rename-to-cite.md`

**Planned changes**
- Replace command invocations (`harness ...`, `./target/release/harness ...`, release binary names in examples) with `cite`-based equivalents where they are command-facing examples.
- Preserve explicit Phase 8 note where runtime naming stays on `HARNESS_*`.

**Validation (immediate)**
- `rg -n "harness\s+(context|search|retrieve|ingest|list|get|trace|read|evaluate|refresh|retry)" README.md docs/demo.md docs/installation.md docs/agent-usage-guide.md docs/rename-to-cite.md`
  - Expected: no matches for canonical command examples.

---

### Slice C — Migration checklist + Phase 9 deferral note (<=300 changed lines)
**Goal:** give local users a safe, explicit transition path without runtime rename.

**Allowlist**
- `docs/sdd/phase-8-rename-cite/migration-checklist.md`
- `docs/installation.md` (link/reference only, optional)

**Planned changes**
- Add checklist covering:
  - update local scripts/aliases to `cite`
  - validate command help and representative command(s)
  - confirm runtime vars remain `HARNESS_*`
  - confirm existing data/db location compatibility
  - rollback steps
- Add explicit “Phase 9 handles `CITE_*` + data/db rename” statement.

**Validation (immediate)**
- `rg -n "CITE_|HARNESS_" docs/sdd/phase-8-rename-cite/migration-checklist.md docs/installation.md`
  - Expected: text clearly states current `HARNESS_*` usage and deferral of `CITE_*` to Phase 9.

---

### Slice D — Verification closeout (<=300 changed lines)
**Goal:** auditable pass/fail evidence for Phase 8 acceptance criteria.

**Allowlist**
- `openspec/changes/phase-8-rename-cite/design.md` (if minor updates from findings)
- `docs/sdd/phase-8-rename-cite/migration-checklist.md` (optional evidence section)

**Required verification suite**
1. `cargo run --bin cite -- --help`
   - Expected: primary name/usage is `cite`.
2. `cargo test`
   - Expected: tests pass.
3. `rg -n "harness\s+(context|search|retrieve|ingest|list|get|trace|read|evaluate|refresh|retry)" README.md docs/demo.md docs/installation.md docs/agent-usage-guide.md docs/rename-to-cite.md`
   - Expected: no canonical command hits.
4. `rg -n "HARNESS_" crates/config crates/storage`
   - Expected: at least one match (runtime naming still present).
5. `rg -n "CITE_" crates/config crates/storage`
   - Expected: no matches (runtime naming migration not started).
6. `rg -n "CITE_|HARNESS_" docs/sdd/phase-8-rename-cite/migration-checklist.md docs/installation.md`
   - Expected: deferral policy clearly documented.

## Contract mapping (spec traceability)

- **CLI identity MUST cut over to `cite`** → Slice A + validation #1.
- **Canonical docs MUST use `cite`** → Slice B + validation #3.
- **Runtime naming MUST be deferred to Phase 9** → Slice C + validations #4/#5/#6.
- **Verification MUST be auditable** → Slice D checklist with explicit expected outcomes.

## Failure handling and rollback

### Failure handling
- If CLI rename compiles but help still shows `harness`: stop after Slice A, fix Clap metadata before doc edits.
- If docs have mixed naming after Slice B: run grep gate and fix only allowlisted files before proceeding.
- If any attempt introduces `CITE_` runtime references in `crates/config` or `crates/storage`: revert that hunk immediately; treat as out-of-scope.

### Rollback
- Revert Slice A commit(s) to restore binary/help identity to `harness`.
- Revert Slice B/C docs commits to restore prior docs.
- Since runtime naming is untouched in Phase 8, no config/data migration rollback is required.

## Test strategy

- Smoke: `cargo run --bin cite -- --help`.
- Regression: `cargo test`.
- Static policy checks via `rg` commands listed above.
- Review gate: no slice exceeds 300 changed lines; each slice constrained to its allowlist.

## Rollout notes

- Apply slices sequentially (A → B → C → D).
- Do not begin Phase 10 hierarchy work until Phase 8 validations are all green.
- Carry forward explicit note into Phase 9 planning: runtime naming migration starts there, not earlier.
