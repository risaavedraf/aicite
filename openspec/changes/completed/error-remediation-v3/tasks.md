# Tasks — error-remediation-v3

**Change:** `error-remediation-v3`  
**Artifact:** `openspec/changes/active/error-remediation-v3/tasks.md`  
**Phase:** tasks  
**Status:** drafted  
**Date:** 2026-06-04

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | Full change: ~1,400-2,500; per planned slice: ~60-380 |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | CR-1 → CR-2 → C9-1 → C9-2 → C9-3 → C9-4 → C9-5a/b → C9-6 → SNAP-1 → SNAP-2 → TIME-1 |
| Delivery strategy | ask-on-risk |
| Chain strategy | pending |

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: pending
400-line budget risk: High

## Session Choices And Gates

- Execution mode for planning: auto.
- Artifact store requested: OpenSpec + Engram. This tasks executor has no Engram tool available; persist important decisions in parent/session memory if available.
- PR strategy: ask-always.
- Review budget: 400 changed lines per PR.
- Strict TDD: `false`, but code slices should still use focused RED → GREEN → TRIANGULATE → REFACTOR sequencing where tests exist.
- Validation commands for code slices:
  - `cargo test`
  - `cargo clippy -- -D warnings`
  - `cargo fmt --check`
- Immediate first slice needing user approval before apply: **CR-1 — verified CodeRabbit code/test fixes**.

## CodeRabbit Verification Checklist

Before editing for any CodeRabbit item, verify the claim against current code and record one status in `openspec/changes/active/error-remediation-v3/apply-progress.md`:

- `verified-fixed`: finding matched current code and was fixed.
- `verified-skipped-stale`: current code already differs from the finding.
- `verified-skipped-not-reproducible`: claim could not be reproduced.
- `verified-deferred`: still valid but deferred to a named slice with reason.

Concrete checklist:

- [x] Verify/fix or skip `crates/cli/src/commands/health.rs` `--json` wording vs live provider check behavior. Reconciled during verify: apply-progress records `verified-fixed` in CR-1, with live provider checks preserved and wording clarified.
- [x] Verify/fix or skip `crates/cli/src/commands/setup.rs` provider/model pairing during test and save. Reconciled during verify: apply-progress records `verified-fixed` in CR-1 with `selected_provider_model()` tests.
- [x] Verify/fix or skip `crates/config/src/lib.rs` environment restoration and isolated fallback tests. Reconciled during verify: apply-progress records `verified-fixed` in CR-1 with env guard/isolation tests.
- [x] Verify/fix or skip `crates/retrieval/src/lib.rs` `ChunkEmbeddingRecord` vector clone in `rank_by_similarity`. Reconciled during verify: apply-progress records `verified-fixed` in CR-1 with reference conversion and ranking coverage.
- [x] Verify/fix or skip `crates/storage/src/rate_limits.rs` non-positive `prune_stale_rate_limits` age handling. Reconciled during verify: apply-progress records `verified-fixed` in CR-1 with invalid-age preservation test.
- [x] Verify/fix or skip prior artifact count corrections in `openspec/changes/active/error-remediation-v2/apply-progress.md` and `openspec/changes/active/error-remediation/second-pass-prompt.md`. Reconciled during verify: apply-progress records `verified-fixed` in CR-2.
- [x] Verify/fix or skip stale archive report claims under `openspec/reports/archive/revision-repo/**`. Reconciled during verify: apply-progress records `verified-fixed` in CR-2 for the listed runtime guard, UTF-8, and provider API-key archive claims.

## Slice CR-1 — Verified CodeRabbit Code/Test Fixes

**Forecast:** ~180-360 changed lines; 400-line risk: Medium.  
**Depends on:** user approval to start apply; proposal/spec/design/tasks.  
**Files/discovery targets:**

- `crates/cli/src/commands/health.rs`
- `crates/cli/src/commands/setup.rs`
- `crates/config/src/lib.rs`
- `crates/retrieval/src/lib.rs`
- `crates/storage/src/rate_limits.rs`
- `openspec/changes/active/error-remediation-v3/apply-progress.md`

Tasks:

1. **RED/VERIFY:** Inspect `health.rs` `execute`, `check_provider`, and `embed("test")` path; decide with user whether apply should preserve live checks with corrected wording or implement true local/no-network JSON behavior.
2. **GREEN:** If approved behavior is wording-only, update `health.rs` help/comment/tests to state JSON may perform live provider checks; if approved behavior is local-only, add a local-only branch that avoids provider embedding calls.
3. **RED:** Add/adjust focused setup test or code inspection checklist proving selected provider and model are paired in `setup.rs` before provider connection testing and config persistence.
4. **GREEN:** Derive the selected-provider model once in `setup.rs`; use it for `test_provider_connection` and saved embedding config instead of stale `config.embedding.model`.
5. **RED:** In `config/src/lib.rs`, identify tests mutating `CITE_*` variables and fallback tests using `Config::load()` against host defaults.
6. **GREEN:** Add save/restore of original env values and change fallback tests to `Config::load_from(Some(nonexistent_path))` or an isolated temp config fixture.
7. **RED:** Add retrieval regression coverage or explicit code-inspection note showing `rank_by_similarity` currently constructs `ScoredChunk` through a candidate clone.
8. **GREEN:** Add `impl From<&ChunkEmbeddingRecord> for ScoredChunk` or equivalent helper in `retrieval/src/lib.rs`; update ranking to avoid cloning embedding vectors while preserving output fields and score/order behavior.
9. **RED:** Add storage rate-limit test proving `max_age_seconds <= 0` does not delete active rows.
10. **GREEN:** Add early invalid-argument style error in `prune_stale_rate_limits` before cutoff calculation for non-positive ages.
11. **TRIANGULATE:** Run focused crate tests for touched areas if available, then `cargo test`.
12. **REFACTOR:** Remove helper duplication and keep changes minimal; do not broaden provider abstractions or retrieval architecture.
13. **VERIFY:** Run `cargo test`, `cargo clippy -- -D warnings`, and `cargo fmt --check`.
14. **ACCEPTANCE EVIDENCE:** Record CodeRabbit statuses and command results in `openspec/changes/active/error-remediation-v3/apply-progress.md`.

Rollback boundary: revert CR-1 code/test files plus its `apply-progress.md` entry; no schema migrations expected.

## Slice CR-2 — Verified CodeRabbit Documentation/Archive Corrections

**Forecast:** ~160-380 changed lines; 400-line risk: Medium, split if archive edits grow.  
**Depends on:** user approval; CR-1 optional but recommended first.  
**Files/discovery targets:**

- `openspec/changes/active/error-remediation-v2/apply-progress.md`
- `openspec/changes/active/error-remediation/second-pass-prompt.md`
- `openspec/reports/archive/revision-repo/cli/errores.md`
- `openspec/reports/archive/revision-repo/compliance/review.md`
- `openspec/reports/archive/revision-repo/engine/errores.md`
- `openspec/reports/archive/revision-repo/engine/review.md`
- `openspec/reports/archive/revision-repo/graph/errores.md`
- `openspec/reports/archive/revision-repo/graph/review.md`
- `openspec/reports/archive/revision-repo/ingest/errores.md`
- `openspec/reports/archive/revision-repo/ingest/review.md`
- `openspec/reports/archive/revision-repo/providers/errores.md`
- Current-code reference targets: `crates/cli/src/commands/ingest.rs`, `crates/engine/src/ingest.rs`, `crates/engine/src/runtime_guard.rs`, `crates/graph/src/heading_parser.rs`, `crates/ingest/src/validator.rs`, `crates/ingest/src/extractor.rs`, provider factory files discovered by searching `create_provider` and `resolve_api_key`.

Tasks:

1. **VERIFY:** Cross-check each documentation finding against current code before editing.
2. Correct V2 PR/wave/commit count inconsistency in `error-remediation-v2/apply-progress.md` only if verified.
3. Correct T3/T4 total inconsistency in `error-remediation/second-pass-prompt.md` only if verified.
4. Correct runtime-guard archive claims to distinguish CLI guard enforcement from any remaining engine-internal API boundary risk.
5. Correct UTF-8 archive claims for graph/ingest only where current code uses `chars().count()` or safe char truncation.
6. Correct provider API-key validation archive claim only after verifying current `create_provider` and `resolve_api_key` behavior.
7. If docs changes approach 350 changed lines, stop and split by report family before continuing.
8. **VERIFY:** Run markdown/reference grep checks; run full Rust validation only if code changed in this slice or before final V3 verification.
9. **ACCEPTANCE EVIDENCE:** Record each docs finding as `verified-fixed` or skipped with reason in `apply-progress.md`.

Rollback boundary: documentation-only revert; no runtime behavior changes.

## Slice C9-1 — Common Typed-ID Foundation

**Forecast:** ~80-180 changed lines; 400-line risk: Low.  
**Depends on:** user approval; can run after CR-1/CR-2 or independently.  
**Files/discovery targets:**

- `crates/common/src/types.rs`
- `crates/common/src/lib.rs` if exports need adjustment
- Existing common tests or new tests colocated under `crates/common/src/`

Tasks:

1. **RED:** Add tests for `DocumentId`, `ChunkId`, and `TraceId` `Display`, `FromStr`, `AsRef<str>`/`Deref<Target = str>`, clone/equality, and serde string round trips.
2. **GREEN:** Implement missing trait support and transparent serde behavior in `types.rs` without migrating downstream crates.
3. **TRIANGULATE:** Add invalid-input tests only if the selected constructors enforce format constraints; otherwise document that IDs are currently string-transparent.
4. **REFACTOR:** Keep constructor names and trait impls consistent across all three ID types.
5. **VERIFY:** Run common crate tests if available, then full validation commands.
6. **ACCEPTANCE EVIDENCE:** Record supported typed-ID traits and unchanged external string representation in `apply-progress.md`.

Rollback boundary: revert common type/test changes only.

## Slice C9-2 — Storage Boundary Typed-ID Pilot

**Forecast:** ~180-360 changed lines; 400-line risk: Medium.  
**Depends on:** C9-1.  
**Files/discovery targets:**

- `crates/storage/src/**` row structs and row decoding helpers discovered by searching `document_id`, `chunk_id`, and `trace_id`
- `crates/common/src/types.rs`
- Storage tests/fixtures for selected pilot path

Tasks:

1. **RED:** Choose one or two high-value storage records/queries as the pilot and add tests that decode persisted string IDs into typed IDs while binding back as strings.
2. **GREEN:** Migrate only the selected storage pilot fields to `DocumentId`, `ChunkId`, or `TraceId`.
3. **TRIANGULATE:** Add invalid storage ID parse/error mapping tests only if C9-1 introduced validation.
4. **REFACTOR:** Centralize raw-string-to-ID conversion helper if repeated in the pilot; do not migrate unrelated records.
5. **VERIFY:** Run storage-focused tests and full validation commands.
6. **ACCEPTANCE EVIDENCE:** Record exact fields migrated and exact fields intentionally left as `String`.

Rollback boundary: revert pilot storage field/test changes; no schema migration.

## Slice C9-3 — Retrieval Boundary Typed IDs

**Forecast:** ~140-320 changed lines; 400-line risk: Medium.  
**Depends on:** C9-2 and preferably CR-1 H19 clone fix.  
**Files/discovery targets:**

- `crates/retrieval/src/lib.rs`
- Storage retrieval records discovered through `ChunkEmbeddingRecord`
- Retrieval tests/fixtures

Tasks:

1. **RED:** Add retrieval tests proving JSON/display strings remain stable when migrated IDs are serialized or output.
2. **GREEN:** Migrate compatible retrieval `document_id`/`chunk_id` fields and conversions to typed IDs.
3. **TRIANGULATE:** Confirm ranking output, `ScoredChunk` conversion, and optional topic/concept fields remain unchanged apart from Rust types.
4. **REFACTOR:** Remove redundant `.to_string()` calls only inside migrated retrieval boundaries.
5. **VERIFY:** Run retrieval tests and full validation commands.
6. **ACCEPTANCE EVIDENCE:** Record retrieval migrated fields and remaining string boundaries.

Rollback boundary: revert retrieval/storage retrieval type changes; no schema migration.

## Slice C9-4 — Graph/Domain Typed IDs

**Forecast:** ~180-380 changed lines; 400-line risk: Medium.  
**Depends on:** C9-1 and C9-2.  
**Files/discovery targets:**

- `crates/graph/src/**`
- Related storage graph/link row structs discovered by searching `document_id`, `chunk_id`, `topic`, and `concept`
- Graph tests/fixtures

Tasks:

1. **RED:** Add graph/domain tests showing ID fields round-trip as strings externally but use typed IDs internally for the selected graph records.
2. **GREEN:** Migrate focused graph/domain ID fields; avoid mixing with `created_at` DateTime work unless diff remains clearly under budget.
3. **TRIANGULATE:** Verify heading/topic boundary behavior and link records are unaffected.
4. **REFACTOR:** Keep conversions at graph/storage boundaries explicit.
5. **VERIFY:** Run graph tests and full validation commands.
6. **ACCEPTANCE EVIDENCE:** Record graph/domain IDs migrated and any deferred graph paths.

Rollback boundary: revert graph/domain type changes; no schema migration.

## Slice C9-5a/b — Engine And CLI Typed-ID Integration

**Forecast:** C9-5 total likely >400; split into ~180-350 changed-line sub-slices.  
**400-line risk:** High if attempted as one PR.  
**Depends on:** C9-2 through C9-4 as relevant.  
**Files/discovery targets:**

- `crates/engine/src/**` discovered by searching `document_id`, `chunk_id`, `trace_id`
- `crates/cli/src/commands/**` discovered by searching ID argument usage
- CLI/engine integration tests and fixtures

Tasks:

1. **RED:** Pick one engine/CLI path per sub-slice and add tests proving existing string CLI arguments still work.
2. **GREEN:** Parse CLI strings into typed IDs at command boundaries and pass typed IDs through the selected engine path.
3. **TRIANGULATE:** Confirm JSON output and fixture snapshots remain string-compatible.
4. **REFACTOR:** Remove redundant conversion churn in the selected path only.
5. **VERIFY:** Run focused CLI/engine tests and full validation commands per sub-slice.
6. **ACCEPTANCE EVIDENCE:** Record each migrated path; split again if changed-line forecast approaches 350.

Rollback boundary: revert one engine/CLI sub-slice at a time.

## Slice C9-6 — Typed-ID Cleanup And Public API Audit

**Forecast:** ~120-280 changed lines; 400-line risk: Low-Medium.  
**Depends on:** prior C9 slices.  
**Files/discovery targets:**

- Workspace-wide search targets: `document_id: String`, `chunk_id: String`, `trace_id: String`, `.to_string()` around typed IDs
- `openspec/changes/active/error-remediation-v3/apply-progress.md`
- Public docs or README files only if they mention ID types

Tasks:

1. Audit remaining raw string ID fields and classify them as migrated, intentionally string-boundary, or deferred.
2. Remove redundant conversions introduced during migration where safe.
3. Add/adjust tests for final public API invariants and serde compatibility.
4. Run full validation commands.
5. Record final C9 boundary map and deferred leftovers with reasons.

Completion state:

- [x] C9-6 typed-ID cleanup/public API audit completed; `apply-progress.md` records the final boundary map, local redundant conversion cleanup, command evidence, and remaining deferred ID boundaries.

Rollback boundary: cleanup-only revert after previous slices remain valid.

## Slice SNAP-1 — Snapshot Activation Rollback Confidence

**Forecast:** ~120-300 changed lines; 400-line risk: Low-Medium.  
**Depends on:** user approval; CR/C9 optional.  
**Files/discovery targets:**

- `crates/engine/src/refresh.rs`
- `crates/storage/src/snapshots.rs`
- Snapshot tests/fixtures discovered by searching `activate_snapshot` and `snapshot_pointer`

Tasks:

1. **RED:** Add a regression test that injects or simulates failure after partial snapshot activation work has begun.
2. **GREEN:** If current transaction behavior passes the test, make no production change; if it fails, minimally repair transaction handling around activation.
3. **TRIANGULATE:** Add successful activation test if current coverage does not prove commit behavior.
4. **REFACTOR:** Keep failure injection test-only if possible.
5. **VERIFY:** Run snapshot-focused tests and full validation commands.
6. **ACCEPTANCE EVIDENCE:** Record whether production code changed or test-only confirmation was sufficient.

Completion state:

- [x] SNAP-1 snapshot activation rollback confidence completed; `apply-progress.md` records the rollback regression test, test-only failure injection, no production transaction change, command evidence, and remaining SNAP-2/TIME-1 scope.

Rollback boundary: revert snapshot tests and any minimal transaction fix.

## Slice SNAP-2 — Snapshot Pointer `updated_at` Migration

**Forecast:** ~160-340 changed lines; 400-line risk: Medium.  
**Depends on:** SNAP-1 preferred.  
**Files/discovery targets:**

- `crates/storage/src/snapshots.rs`
- Migration files or migration registry discovered by searching `CREATE TABLE snapshot_pointer` and existing migrations
- Snapshot pointer tests

Tasks:

1. **RED:** Add old-schema migration test proving a database without `snapshot_pointer.updated_at` gains a valid timestamp.
2. **GREEN:** Add additive migration and update pointer write/activation paths to set or refresh `updated_at`.
3. **TRIANGULATE:** Add test proving repeated pointer update refreshes timestamp in the expected storage format.
4. **REFACTOR:** Avoid `SELECT *` assumptions and keep compatibility with existing rows.
5. **VERIFY:** Run storage snapshot tests and full validation commands.
6. **ACCEPTANCE EVIDENCE:** Record migration behavior and downgrade/rollback note.

Completion state:

- [x] SNAP-2 snapshot pointer `updated_at` migration completed; `apply-progress.md` records the additive migration, old-schema backfill behavior, activation refresh behavior, command evidence, and rollback/downgrade note.

Rollback boundary: code rollback tolerates additive column; database downgrade is not automatic.

## Slice TIME-1 — `created_at` DateTime<Utc> Consistency

**Forecast:** ~220-380 changed lines; 400-line risk: Medium.  
**Depends on:** user approval; avoid combining with broad C9 slices.  
**Files/discovery targets:**

- `crates/graph/src/**` for `Topic` and `Concept`
- `crates/storage/src/**` for `SemanticLinkRow`
- CLI/output paths discovered by searching `created_at`
- Timestamp tests/fixtures

Tasks:

1. **RED:** Add tests for valid timestamp parse, invalid timestamp rejection, and stable CLI/JSON formatting for selected records.
2. **GREEN:** Change selected `created_at` fields to `chrono::DateTime<Utc>` and add explicit storage parse/format boundaries.
3. **TRIANGULATE:** Verify representative existing SQLite timestamp strings parse successfully.
4. **REFACTOR:** Centralize timestamp parsing/formatting helpers if repeated; do not migrate unrelated timestamp fields.
5. **VERIFY:** Run graph/storage/CLI focused tests and full validation commands.
6. **ACCEPTANCE EVIDENCE:** Record migrated fields and external timestamp format guarantees.

Completion state:

- [x] TIME-1 `created_at` DateTime<Utc> consistency completed for selected graph/storage records; `apply-progress.md` records migrated fields, external timestamp format guarantees, command evidence, CLI discovery, and deferred timestamp leftovers.

Rollback boundary: revert domain type changes; no schema migration if persisted strings remain unchanged.

## Final V3 Verification And Tracking

**Forecast:** ~40-120 documentation/tracking lines; 400-line risk: Low.  
**Depends on:** all approved slices.

Tasks:

1. Ensure `openspec/changes/active/error-remediation-v3/apply-progress.md` classifies every CodeRabbit finding and each V3 deferred/core item.
2. Write `openspec/changes/active/error-remediation-v3/verify-report.md` with final command outputs and any skipped/deferred rationale.
3. Update `openspec/reports/error-tracking.md` if present and if V3 item statuses changed.
4. Run final validation commands:
   - `cargo test`
   - `cargo clippy -- -D warnings`
   - `cargo fmt --check`
5. If C9 changed public APIs, prepare version-bump decision notes for user approval before release/merge.

## Recommended Next Step

Ask the user to approve **CR-1 — verified CodeRabbit code/test fixes** as the immediate first apply slice and to choose the `health --json` behavior contract if implementation will do more than wording correction.

## Skill Resolution

`none`

No explicit `SKILL.md` path was injected into this delegated tasks phase. Work used the assigned SDD tasks-executor instructions and the requested OpenSpec inputs only.
