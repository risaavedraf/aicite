# Tasks — Phase 9 Installation Experience

## Review Workload Forecast

| Field | Value |
|---|---|
| Estimated changed lines | 220-320 total |
| 300-line budget risk | Medium (near/over threshold depending on doc edits) |
| Chained PRs recommended | Yes |
| Suggested split | PR-A: Slice A, PR-B: Slice B, PR-C: Slice C |
| Delivery strategy | ask-always |
| Chain strategy | by-slice |

Decision needed before apply: **Yes**
Chained PRs recommended: **Yes**
Chain strategy: **by-slice**
300-line budget risk: **Medium**

## Evidence location (mandatory)

Record verification outputs in:
- `docs/sdd/phase-9-installation-experience/verification-evidence.md`

## Task 1 — Slice A: Canonical install/run pathways

**Goal**: Make local dev, local built binary, and release-binary usage unambiguous and consistent.

**Files (allowlist)**
- `README.md`
- `docs/installation.md`
- `docs/demo.md`
- `docs/agent-usage-guide.md`

**Steps**
1. Normalize pathway sections by execution mode:
   - Dev: `cargo run --bin cite -- ...`
   - Local build: `cargo build --release` + `./target/release/cite ...`
   - Release binary: download + run `cite` / `cite.exe`
2. Remove conflicting wording/examples for the same pathway.
3. Keep examples executable and concise.

**Verify**
- `cargo run --bin cite -- --help`
- `cargo build --release`
- `./target/release/cite health --json`
- `rg -n "cargo run --bin cite|target/release/cite|cite.exe|release" README.md docs/installation.md docs/demo.md docs/agent-usage-guide.md`

**Definition of done**
- Canonical pathway matrix is coherent across all listed docs.
- Commands match real executable names.
- Slice diff remains reviewable and within target budget for its PR.

---

## Task 2 — Slice B: Runtime naming/data migration policy

**Goal**: Resolve Phase 8 deferral into explicit, non-contradictory policy.

**Files (allowlist)**
- `docs/installation.md`
- `docs/rename-to-cite.md`
- `.env.example`
- `README.md` (runtime/env section only)

**Steps**
1. Define canonical runtime namespace used now.
2. State current canonical data dir/db naming used now.
3. Document compatibility behavior (legacy acceptance vs manual migration path).
4. Remove contradictory placeholder lines (e.g., self-mapping migration statements).

**Verify**
- `rg -n "CITE_|CITE_|cite.db|cite.db|deferred to Phase 9|migration" README.md docs/installation.md docs/rename-to-cite.md .env.example`
- `rg -n "std::env::var\(\"CITE_" crates/config crates/cli`
- `rg -n "join\(\"cite\"\)|cite.db" crates/cli crates/storage`

**Definition of done**
- Docs clearly state what is canonical now and what compatibility exists.
- Runtime policy in docs matches actual code behavior.
- No contradictory migration placeholders remain in allowlisted docs.

---

## Task 3 — Slice C: Migration checklist + verification closeout

**Goal**: Provide auditable migration/rollback guidance and acceptance evidence.

**Files (allowlist)**
- `docs/sdd/phase-9-installation-experience/migration-checklist.md`
- `docs/sdd/phase-9-installation-experience/verification-evidence.md`
- `openspec/changes/phase-9-installation-experience/apply-progress.md` (optional)

**Steps**
1. Write migration checklist with:
   - pre-checks
   - migration steps
   - validation commands and expected outcomes
   - rollback steps
2. Execute verification suite and capture outputs + pass/fail:
   - local dev path check
   - local release build path check
   - release artifact naming/doc consistency checks
   - runtime naming policy checks
   - rollback sanity checks
3. Mark acceptance gate status.

**Verify**
- Ensure evidence file contains each command, exit status, and short verdict.
- Ensure checklist and evidence are internally consistent with Slice A/B decisions.

**Definition of done**
- Migration and rollback are executable by a local user.
- Evidence is sufficient for independent review/audit.
- Phase 9 acceptance criteria can be evaluated from artifacts alone.
