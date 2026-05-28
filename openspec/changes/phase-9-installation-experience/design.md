# Design — Phase 9 Installation Experience

Phase 9 is limited to installation and runtime migration clarity before hierarchy work.

## Quick path

1. Canonicalize run/install pathways in core docs.
2. Define explicit runtime naming and compatibility policy.
3. Add auditable verification evidence (including rollback checks).

## Scope boundaries

### In scope
- Canonical command pathways for:
  - `cargo run --bin cite -- ...`
  - `./target/release/cite ...`
  - release-downloaded executable usage (`cite` / `cite.exe`)
- Runtime naming/data-path policy clarification and migration checklist.
- Verification evidence for acceptance criteria.

### Out of scope
- Hierarchy schema/retrieval (Phases 10-11).
- Agent UX formatting changes (Phase 12).
- New retrieval semantics.

## Slice plan (A/B/C)

### Slice A — Install/run pathway canonicalization

**Goal:** remove ambiguity in local and release usage paths.

**Allowlist**
- `README.md`
- `docs/installation.md`
- `docs/demo.md`
- `docs/agent-usage-guide.md`

**Changes**
- Normalize command examples by pathway (dev / local built / release binary).
- Ensure pathway labels are consistent across docs.

**Verification**
- `cargo run --bin cite -- --help`
- `cargo build --release`
- `./target/release/cite health --json`
- Grep checks for contradictory command patterns in canonical docs.

---

### Slice B — Runtime naming/data migration policy

**Goal:** resolve Phase 8 deferral into explicit policy and compatibility rules.

**Allowlist**
- `docs/installation.md`
- `docs/rename-to-cite.md`
- `.env.example`
- `README.md` (runtime/env section only)

**Changes**
- State canonical runtime namespace and current data/db naming.
- Remove contradictory placeholder statements (e.g., self-mapping migration lines).
- Document compatibility mode vs explicit migration path and boundary.

**Verification**
- Grep policy sections to confirm one canonical statement and no contradictory mappings.
- Validate examples in docs and `.env.example` align with runtime code expectations.

---

### Slice C — Verification evidence + rollback checklist

**Goal:** produce auditable closeout evidence.

**Allowlist**
- `docs/sdd/phase-9-installation-experience/verification-evidence.md`
- `docs/sdd/phase-9-installation-experience/migration-checklist.md`
- `openspec/changes/phase-9-installation-experience/apply-progress.md` (optional)

**Changes**
- Add migration checklist: pre-checks, steps, validation commands, rollback commands.
- Record command outputs + pass/fail outcomes.

**Verification**
- Run/check all acceptance commands and store outputs with outcome labels.

## Spec-to-slice contract mapping

- **Canonical local run/install pathways** → Slice A
- **Release artifact naming/usage consistency** → Slice A (+ targeted checks)
- **Runtime naming migration policy** → Slice B
- **Migration validation + rollback evidence** → Slice C
- **Auditable verification evidence** → Slice C

## Failure handling

- If docs show mixed command pathways after Slice A: stop and reconcile before Slice B.
- If policy remains contradictory after Slice B: do not proceed to verification until one canonical policy statement is accepted.
- If any verification check fails in Slice C: mark Phase 9 verify as failed and record remediation task.

## Rollback

- Revert Slice A doc commits to restore previous command guidance.
- Revert Slice B policy edits if migration guidance is unsafe/incorrect.
- Keep Slice C evidence files even on failure (audit trail), adding failure notes.

## Workload forecast and PR strategy

| Field | Value |
|---|---|
| Estimated changed lines | 220-320 total |
| Review budget risk (300) | Medium (near threshold) |
| Chained PR recommended | Yes, by slice |
| Strategy | Ask before each PR (A, B, C) |

Recommended delivery split:
1. PR-A: Slice A
2. PR-B: Slice B
3. PR-C: Slice C (evidence/closeout)
