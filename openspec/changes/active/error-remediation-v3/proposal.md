# Proposal — error-remediation-v3

**Change:** `error-remediation-v3`  
**Artifact:** `openspec/changes/active/error-remediation-v3/proposal.md`  
**Phase:** proposal  
**Status:** proposed  
**Date:** 2026-06-04

## Intent

Complete the third error-remediation pass for `aiharness` by addressing deferred type-safety, snapshot correctness, timestamp consistency, and retrieval cleanup work that was intentionally left out of earlier remediation passes.

This proposal also brings `coderabbit-findings.md` into scope as an added validation/remediation lane. Those findings should be verified against current code before implementation; only still-valid findings should be fixed, and stale findings should be recorded as skipped with a short reason.

## Background

Earlier remediation passes handled the first two waves of repository quality debt:

- `error-remediation`: first pass for T1/T2 issues.
- `error-remediation-v2`: second pass for T3/T4 issues and unchecked casts.

`error-remediation-v3` currently resumes the items deferred because they were broader, riskier, or better handled after the lower-risk cleanup was complete. The active change is under:

- `openspec/changes/active/error-remediation-v3/`

Session choices for this continuation:

- **Execution mode:** auto planning
- **Artifact store:** OpenSpec + Engram requested
- **PR strategy:** ask-always
- **Review budget:** 400 changed lines per PR

Engram note: this proposal executor has no callable memory tools available, so this phase persists to OpenSpec only. Parent/session memory persistence should save this decision if Engram tools are available there.

## Scope

### Lane A — CodeRabbit validation and minimal remediation

CodeRabbit findings are included in V3 as a separate validation/remediation lane. The lane is intentionally narrow: verify each finding, fix only issues that still reproduce in current code, and avoid broad rewrites.

Candidate items:

1. **CLI health JSON behavior**
   - Verify whether `--json` claims local-only behavior while still performing provider checks through `embed("test")`.
   - Fix either by making JSON/local-only behavior truly local or by correcting the help text and behavior contract.

2. **CLI setup provider/model consistency**
   - Verify whether setup reuses `config.embedding.model` for provider selections that require provider-specific defaults.
   - Ensure connection testing and saved config use the selected provider's model consistently.

3. **Config test determinism**
   - Restore mutated `CITE_*` environment variables after tests.
   - Use isolated config loading instead of host-default `Config::load()` where fallback behavior is under test.

4. **Retrieval hot-path clone avoidance**
   - Overlaps existing V3 H19.
   - Add a reference-based `ScoredChunk` conversion if still needed and remove unnecessary `ChunkEmbeddingRecord.vector` cloning in ranking.

5. **Storage rate-limit pruning validation**
   - Reject non-positive `max_age_seconds` before computing cutoff/deleting rows.

6. **OpenSpec and archive documentation corrections**
   - Correct stale or internally inconsistent claims in prior remediation artifacts and archived review reports.
   - Keep documentation edits concise and factual.

### Lane B — Deferred V3 core remediation

1. **C9 newtype migration**
   - Migrate `DocumentId`, `ChunkId`, and `TraceId` from unused definitions toward actual public and cross-crate usage.
   - Preserve serialization, CLI behavior, storage persistence, fixtures, and public API clarity.
   - Split into reviewable increments; this is the highest-risk lane and likely cannot fit in one PR.

2. **H7 snapshot activation rollback confidence**
   - Add tests proving partial snapshot activation failures rollback as expected through SQLite transaction behavior.
   - Avoid changing transaction semantics unless tests expose a real correctness gap.

3. **`created_at` type consistency**
   - Replace public `String` timestamps with `DateTime<Utc>` where appropriate for `Topic`, `Concept`, and `SemanticLinkRow`.
   - Preserve SQLite/CLI input-output behavior and make parsing/formatting boundaries explicit.

4. **Snapshot pointer `updated_at`**
   - Add an additive migration and write-path updates so snapshot pointers track update time.
   - Keep backward compatibility for existing databases.

5. **H19 ScoredChunk dedup**
   - Prefer the minimal clone-removal fix in Lane A.
   - Avoid re-architecting `ScoredChunk` unless later spec/design proves the API change is worth the churn.

## Out of Scope

- Rewriting the retrieval ranking architecture beyond removing unnecessary clones.
- Replacing SQLite persistence or changing the durable process model.
- Adding daemon/server behavior.
- Broad public API redesign outside typed IDs and timestamp consistency.
- Fixing CodeRabbit findings that are stale, already addressed, or not reproducible after verification.
- Large markdown rewrites of archived reports beyond targeted factual corrections.

## Affected Areas

- `crates/common/src/types.rs` — typed identifiers.
- `crates/storage/src/` — row decoding, snapshot tables, timestamp persistence, rate-limit pruning.
- `crates/engine/src/` — snapshot activation and typed-ID/timestamp callers.
- `crates/cli/src/commands/` — setup, health, argument parsing, display/output boundaries.
- `crates/retrieval/src/lib.rs` — `ScoredChunk` conversion and ranking clone behavior.
- `crates/graph/src/` — topic/concept timestamp types and ID usage.
- `crates/config/src/lib.rs` — deterministic environment/config tests.
- `openspec/changes/active/error-remediation-v2/` and `openspec/changes/active/error-remediation/` — prior artifact count corrections if verified.
- `openspec/reports/archive/revision-repo/**` — targeted stale-report corrections if verified.

## Proposed Delivery Shape

Because the full scope exceeds the 400-line review budget, implementation should be split after spec/design/tasks. The likely PR grouping is:

1. **PR 1 — CodeRabbit validated fixes**
   - CLI health/setup, config test determinism, storage prune validation, H19 minimal clone removal.
   - Include only findings verified against current code.

2. **PR 2 — CodeRabbit documentation corrections**
   - Prior artifact count corrections and archived report factual updates.
   - Could merge into PR 1 only if changed-line budget remains comfortable.

3. **PR 3+ — Newtype migration slices**
   - Split C9 by stable API boundary rather than by arbitrary crate if possible.
   - Candidate order: common type capabilities, storage boundaries, retrieval/graph/engine consumers, CLI/tests/fixtures.

4. **PR N — Snapshot correctness**
   - H7 rollback tests and snapshot pointer `updated_at` migration may be grouped because they touch snapshot storage paths.

5. **PR N+1 — Timestamp type consistency**
   - `DateTime<Utc>` migration for graph/storage rows, with CLI/storage formatting boundaries.

The exact PR split must be finalized in design/tasks and confirmed with the user because the selected strategy is ask-always.

## Risks

- **C9 migration breadth:** Typed IDs touch public APIs and many crate boundaries; naive migration can exceed review budget and create large fixture churn.
- **Compatibility risk:** Newtypes and `DateTime<Utc>` can affect serde, CLI output, SQLite row mapping, and downstream callers.
- **Migration risk:** Snapshot pointer `updated_at` requires additive database migration and old-database compatibility.
- **Behavior ambiguity:** CLI health `--json` may require a product decision: preserve current live provider checks with clearer wording, or make JSON/local-only output avoid network calls.
- **Documentation drift:** Archived report corrections are low runtime risk but can inflate review size if edited broadly.
- **Verification dependency:** CodeRabbit findings are advisory until verified against the current branch.

## Rollback Plan

- Keep each PR independently reviewable and revertible.
- Prefer additive schema migrations; if a migration is introduced, include downgrade/compatibility notes in design where applicable.
- For C9, migrate through explicit conversion boundaries so any problematic crate slice can be reverted without forcing a full repository rollback.
- For CLI behavior changes, preserve existing behavior unless the spec explicitly chooses a new user-visible contract.
- For documentation-only corrections, rollback is a normal file revert.

## Success Criteria

The change is successful when:

1. Each CodeRabbit finding is marked verified-fixed or skipped with a reason.
2. `DocumentId`, `ChunkId`, and `TraceId` are used at meaningful crate/API boundaries according to the final spec, with no unsafe stringly-typed regressions in migrated paths.
3. Snapshot activation rollback behavior is covered by tests that fail on partial-activation leakage.
4. Snapshot pointer rows maintain `updated_at` through migration and update paths.
5. Public timestamp fields selected by the spec use `DateTime<Utc>` with stable SQLite and CLI formatting boundaries.
6. Retrieval ranking no longer clones embedding vectors merely to construct `ScoredChunk`.
7. Prior OpenSpec/archive documentation corrections are factual, concise, and consistent with current code.
8. `cargo test`, `cargo clippy -- -D warnings`, and `cargo fmt --check` pass for the final implementation.
9. Each PR stays under the 400 changed-line review budget unless the user explicitly approves an exception.

## Proposal Question Round

No live product-question round was available to this proposal executor, but these are the assumptions and questions that should be user-reviewed before spec/design finalization:

1. **CLI health behavior:** Should JSON health output be truly local/no-network, or should it keep live provider checks and only correct the wording?
2. **Typed-ID rollout:** Is the goal full public API migration in V3, or a staged migration that starts at storage/retrieval boundaries and leaves some string-facing CLI surfaces unchanged?
3. **Archive docs:** Should stale archived reports be corrected as part of V3 implementation PRs, or kept in a separate documentation-only PR to protect code-review focus?
4. **Snapshot migration:** Is adding `updated_at` to `snapshot_pointer` required in the first V3 slice, or can it wait until after rollback tests establish current transaction behavior?
5. **Versioning:** If C9 changes public APIs, should V3 include a version bump plan, likely to `v0.3.0`?

## Skill Resolution

`paths-injected`

The prior init artifact reports the Gentle AI skill path was injected and the local skill registry exists. No additional skill discovery or subagent launch was performed by this proposal executor.
