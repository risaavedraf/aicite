# SDD Init — error-remediation-v3

**Change:** `openspec/changes/active/error-remediation-v3/`  
**Generated:** 2026-06-04  
**Phase:** init

## Status

`completed`

## Executive Summary

Initialized SDD context for resuming `error-remediation-v3`. `openspec/config.yaml` already exists and was read; no configuration edits were required or made. The active change is still pre-proposal: only `README.md`, `third-pass-prompt.md`, and `coderabbit-findings.md` are present; proposal/spec/design/tasks/apply/verify artifacts are not present yet.

The repository is a Rust workspace with 9 crates (`cli`, `engine`, `storage`, `config`, `graph`, `retrieval`, `ingest`, `providers`, `common`) using SQLite/vector-search architecture. Current branch is `refactor/error-remediation-v2-waves-1-2`.

## Current SDD Configuration

From `openspec/config.yaml`:

- **Project name:** `aicite`
- **Description:** CLI-first semantic document engine for AI agents — Rust + SQLite + vector search
- **Language:** Rust
- **Repo root:** `.`
- **Execution mode:** `interactive`
- **Artifact store:** `openspec`
- **Artifact directory:** `openspec/changes`
- **Chained PR strategy:** `ask_always`
- **Review budget:** `400` changed lines
- **Phases:** init → explore → proposal → spec → design → tasks → apply → verify → archive

### Session Overrides Applied for This Resume

- **execution_mode:** `auto`
- **artifact_store:** `both` (`OpenSpec + Engram` requested)
- **chained_pr_strategy:** `ask_always`
- **review_budget_lines:** `400`

Note: this init executor has no callable Engram memory tools available, so persistence performed here is OpenSpec-only. The requested Engram side should be handled by the parent/session if memory tools are available there.

## Current Testing Configuration

From `openspec/config.yaml`:

- **strict_tdd:** `false`
- **Test command:** `cargo test`
- **Lint command:** `cargo clippy -- -D warnings`
- **Format command:** `cargo fmt --check`

## Active Change Context

`README.md` marks the change as **NOT STARTED** and lists deferred scope:

1. **C9 Newtype migration** — high risk, ~50 files, priority 1
2. **H7 Snapshot rollback** — medium risk, 2–3 files, priority 2
3. **created_at type consistency** — medium risk, 5–10 files, priority 3
4. **H19 ScoredChunk dedup** — low/medium risk, 1–2 files, priority 4
5. **Snapshot pointer updated_at migration** — low risk, 2–3 files, priority 5

`third-pass-prompt.md` recommends proposal → spec → design → tasks before apply, grouping by theme and keeping PRs under the 400-line review budget.

`coderabbit-findings.md` is present and should be considered for scope expansion. It adds a validation/minimal-fix lane covering CLI health/setup behavior, config test determinism, retrieval clone avoidance, storage rate-limit validation, and stale OpenSpec/archive documentation corrections.

## Artifacts

- Read: `openspec/config.yaml`
- Read: `openspec/changes/active/error-remediation-v3/README.md`
- Read: `openspec/changes/active/error-remediation-v3/third-pass-prompt.md`
- Read: `openspec/changes/active/error-remediation-v3/coderabbit-findings.md`
- Read: `Cargo.toml`
- Confirmed present: `.atl/skill-registry.md`
- Written: `openspec/changes/active/error-remediation-v3/init.md`

## Next Recommended

Proceed to **proposal** for `error-remediation-v3`, explicitly deciding whether CodeRabbit findings are in scope. Recommended proposal split:

1. **CodeRabbit validation/minimal-fix lane** — small, independently reviewable, likely before broad C9 work.
2. **Deferred V3 core lane** — C9 newtypes plus snapshot/type-consistency work, split into PR-sized phases under 400 changed lines.

## Risks

- C9 newtype migration is broad and likely exceeds review budget unless split carefully.
- `openspec/config.yaml` project name is `aicite` while the working directory/project context is `aiharness`; this may be historical naming but should be noted for future memory/artifact workflows.
- `coderabbit-findings.md` is currently untracked in Git status.
- Repository has an untracked `tmp/` directory; not investigated during init.
- Config says `strict_tdd: false`, so later phases should define explicit test discipline per task rather than assuming strict TDD.

## Skill Resolution

`paths-injected`

The inherited session included the Gentle AI skill path and `.atl/skill-registry.md` exists. No additional skill discovery or subagent launch was performed by this init executor.
