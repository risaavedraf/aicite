# Tasks — Phase 8 Rename `cite` → `cite`

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 180-280 total (4 slices, each <=300) |
| 400-line budget risk | Low |
| Chained PRs recommended | No |
| Suggested split | PR1: Slice A, PR2: Slice B, PR3: Slice C+D |
| Delivery strategy | ask-on-risk |
| Chain strategy | size-exception |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: Low

## Evidence location (mandatory)

- Record all verification command outputs in:
  - `docs/sdd/phase-8-rename-cite/verification-evidence.md`

## Task 1 — Slice A: CLI identity rename

**Goal**: Primary command identity becomes `cite`.

**Files (allowlist)**
- `crates/cli/Cargo.toml`
- `crates/cli/src/main.rs`

**Steps**
1. Rename CLI binary target name from `cite` to `cite`.
2. Update Clap command/app name to `cite`.
3. Confirm no runtime naming edits (`CITE_*`) are introduced.

**Verify**
- `cargo run --bin cite -- --help` (Usage starts with `cite`)
- Append outputs + pass/fail to `docs/sdd/phase-8-rename-cite/verification-evidence.md`

**Definition of done**
- Help output shows `cite` as primary command name.
- Slice diff stays <=300 changed lines.

---

## Task 2 — Slice B: Canonical docs command surface

**Goal**: Canonical command examples use `cite`.

**Files (allowlist)**
- `README.md`
- `docs/demo.md`
- `docs/installation.md`
- `docs/agent-usage-guide.md`
- `docs/rename-to-cite.md`

**Steps**
1. Replace command-facing examples using `cite` with `cite`.
2. Keep Phase 9 deferral text coherent where runtime naming is mentioned.

**Verify**
- `rg -n "cite\s+(context|search|retrieve|ingest|list|get|trace|read|evaluate|refresh|retry)" README.md docs/demo.md docs/installation.md docs/agent-usage-guide.md docs/rename-to-cite.md`
  - Expected: no command-example hits.
- Append outputs + pass/fail to `docs/sdd/phase-8-rename-cite/verification-evidence.md`

**Definition of done**
- Canonical docs listed above use `cite` for command examples.
- Slice diff stays <=300 changed lines.

---

## Task 3 — Slice C: Local migration checklist + deferrals

**Goal**: Document safe local migration with explicit Phase 9 runtime deferral.

**Files (allowlist)**
- `docs/sdd/phase-8-rename-cite/migration-checklist.md`
- `docs/installation.md` (link/reference only)

**Steps**
1. Create checklist for local single-user migration:
   - update scripts/aliases to `cite`
   - run help smoke command
   - keep runtime env naming on `CITE_*`
   - confirm current data/db paths remain unchanged
   - rollback steps
2. State explicitly: `CITE_*` and data/db path rename start in Phase 9.

**Verify**
- `rg -n "CITE_|CITE_" docs/sdd/phase-8-rename-cite/migration-checklist.md docs/installation.md`
  - Expected: deferral policy is explicit.
- Append outputs + pass/fail to `docs/sdd/phase-8-rename-cite/verification-evidence.md`

**Definition of done**
- Checklist exists and is clear for local execution.
- Phase 9 boundary is explicit and non-contradictory.
- Slice diff stays <=300 changed lines.

---

## Task 4 — Slice D: Verification closeout

**Goal**: Produce auditable acceptance evidence for Phase 8.

**Files (allowlist)**
- `docs/sdd/phase-8-rename-cite/verification-evidence.md`
- `openspec/changes/phase-8-rename-cite/design.md` (only if tiny note update is required)

**Steps**
1. Run full required suite:
   - `cargo run --bin cite -- --help`
   - `cargo test`
   - docs grep from Task 2
   - `rg -n "CITE_" crates/config crates/storage` (expect >=1 match)
   - `rg -n "CITE_" crates/config crates/storage` (expect 0 matches)
   - `rg -n "CITE_|CITE_" docs/sdd/phase-8-rename-cite/migration-checklist.md docs/installation.md`
2. Record command, exit result, and short outcome in evidence file.
3. Mark Phase 8 acceptance gate pass/fail.

**Definition of done**
- Evidence file contains all required command outputs and pass/fail verdicts.
- Scope boundary (runtime naming deferred) is demonstrated by checks.
- Slice diff stays <=300 changed lines.
