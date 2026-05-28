# Apply Progress — phase-9-installation-experience

## Completed tasks

- [x] Task 1 / Slice A — Canonical install/run pathways
  - Added an explicit run/install matrix and normalized pathway examples in canonical docs.
  - Ensured pathways clearly separate dev run, local built binary, and installed release binary usage.

- [x] Task 2 / Slice B — Runtime naming/data migration policy
  - Documented canonical runtime namespace as `CITE_*`.
  - Documented canonical local naming as config/data under `cite` with DB `cite.db`.
  - Clarified compatibility: no runtime auto-alias for legacy `HARNESS_*` or legacy `harness` path names.
  - Removed contradictory self-mapping placeholders from runtime naming docs.

- [x] Task 3 / Slice C — Migration checklist + verification closeout
  - Created migration checklist with pre-checks, migration steps, validation commands, and rollback commands.
  - Created auditable verification evidence with command outputs, exit status, and pass/fail outcomes.
  - Re-ran core command checks and recorded outcomes for closeout.

## Files changed

- `README.md`
- `docs/installation.md`
- `docs/demo.md`
- `docs/agent-usage-guide.md`
- `docs/rename-to-cite.md`
- `.env.example`
- `docs/sdd/phase-9-installation-experience/migration-checklist.md`
- `docs/sdd/phase-9-installation-experience/verification-evidence.md`
- `openspec/changes/phase-9-installation-experience/apply-progress.md`

## Verification commands run

### Slice A
- `cargo run --bin cite -- --help` ✅
- `cargo build --release` ✅
- `./target/release/cite health --json` ✅
- `rg -n "cargo run --bin cite|target/release/cite|cite.exe|release" README.md docs/installation.md docs/demo.md docs/agent-usage-guide.md` ✅

### Slice B
- `rg -n "CITE_\*|HARNESS_\*|cite\.db|deferred to Phase 9|auto-aliased|auto-mapped|migration" README.md docs/installation.md docs/rename-to-cite.md .env.example` ✅
- `rg -n "CITE_\*\s*→\s*CITE_\*|cite\.db\s*→\s*cite\.db|harness\s*→\s*harness" README.md docs/installation.md docs/rename-to-cite.md .env.example` ✅ (expected no matches / exit 1)
- `rg -n "std::env::var\(\"CITE_" crates/config crates/cli` ✅
- `rg -n "join\(\"cite\"\)|cite\.db" crates/cli crates/storage` ✅

### Slice C closeout
- `cargo run --bin cite -- --help` ✅
- `cargo build --release` ✅
- `./target/release/cite health --json` ✅
- `rg -n "cargo run --bin cite|target/release/cite|cite.exe|Path A|Path B|Path C" README.md docs/installation.md docs/demo.md docs/agent-usage-guide.md` ✅
- `rg -n "Runtime naming policy|CITE_\*|HARNESS_\*|cite\.db|not auto|GEMINI_API_KEY|OPENAI_API_KEY" README.md docs/installation.md docs/rename-to-cite.md .env.example` ✅
- `rg -n "CITE_\*\s*[-=]?>\s*CITE_\*|cite\.db\s*[-=]?>\s*cite\.db|harness\s*[-=]?>\s*harness" README.md docs/installation.md docs/rename-to-cite.md .env.example` ✅ (expected no matches / exit 1)
- `rg -n "std::env::var\(\"CITE_" crates/config crates/cli` ✅
- `rg -n "join\(\"cite\"\)|cite\.db" crates/cli crates/storage` ✅
- `rg -n "releases/download|cite-(linux|macos|windows)|cite\.exe|bin\.install \"cite\"" docs/installation.md README.md docs/demo.md docs/agent-usage-guide.md` ✅

## TDD Cycle Evidence

Not applicable (strict TDD is disabled in `openspec/config.yaml`).

## Deviations from design

- None for Slice A/B/C scope.

## Remaining tasks

- [x] Task 1 / Slice A — complete
- [x] Task 2 / Slice B — complete
- [x] Task 3 / Slice C — complete

## Workload / PR boundary

- Delivery boundary implemented: **Slice A + Slice B + Slice C complete**.
- Recommended PR boundaries:
  - `PR-A`: Slice A docs only
  - `PR-B`: Slice B policy docs only
  - `PR-C`: Slice C migration checklist + verification evidence
