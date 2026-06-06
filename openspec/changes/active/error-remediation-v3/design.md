# Design — error-remediation-v3

**Change:** `error-remediation-v3`  
**Artifact:** `openspec/changes/active/error-remediation-v3/design.md`  
**Phase:** design  
**Status:** drafted  
**Date:** 2026-06-04

## 1. Session and Delivery Choices

This design continues from the already-written proposal and specification.

| Setting | Value |
| --- | --- |
| Execution mode | `auto` for planning; apply still gated by ask-always PR decisions |
| Artifact store | OpenSpec + Engram requested |
| PR strategy | `ask-always` |
| Review budget | 400 changed lines per PR |
| Strict TDD | `false` |
| Validation commands | `cargo test`; `cargo clippy -- -D warnings`; `cargo fmt --check` |

Engram note: this design executor has no callable Engram memory tools available in its toolset, so the design is persisted to OpenSpec only. Parent/session memory should save the decision under `sdd/error-remediation-v3/design` if memory tools are available there.

## 2. Design Goals

1. Include `coderabbit-findings.md` as a validation/minimal-remediation lane without treating findings as automatically true.
2. Keep CodeRabbit code/test fixes separate from documentation/archive corrections unless the final diff is demonstrably safer and still under the 400-line budget.
3. Split the broad C9 typed-ID migration into dependency-aware, revertible, PR-sized slices.
4. Preserve external compatibility for CLI arguments, JSON output, SQLite persisted strings, and fixtures unless a deliberate versioned API break is approved.
5. Prefer tests that expose the targeted behavior before or alongside implementation, even though strict TDD is disabled.
6. Keep every apply slice independently verifiable with the configured workspace commands.

## 3. High-Level Architecture

V3 is organized as two major lanes with separate review concerns.

```text
Lane A: CodeRabbit validation/minimal remediation
  A1. Verify findings against current code
  A2. Apply still-valid code/test fixes
  A3. Apply concise docs/archive corrections
  A4. Record fixed/skipped status per finding

Lane B: Deferred core remediation
  B1. C9 typed identifiers in dependency-aware slices
  B2. Snapshot activation rollback tests
  B3. Snapshot pointer updated_at additive migration
  B4. created_at DateTime<Utc> consistency
  B5. H19 ScoredChunk clone removal, preferably handled in A2
```

Lane A should normally land before Lane B because it is smaller, clarifies stale documentation, and includes the low-risk H19 clone-removal overlap. Lane B then proceeds from stable artifacts and cleaner review context.

## 4. CodeRabbit Validation and Minimal Remediation Lane

### 4.1 Validation Contract

Before editing any file for a CodeRabbit item, the apply phase must verify the claim against current code. Each item receives one of these statuses in `apply-progress.md` or an equivalent V3 tracking artifact:

- `verified-fixed`: current code matched the finding and the slice fixed it.
- `verified-skipped-stale`: current code no longer matched the finding.
- `verified-skipped-not-reproducible`: the finding could not be reproduced.
- `verified-deferred`: still valid, but deliberately deferred with a reason and owner slice.

No item should remain `unverified` after the CodeRabbit lane verify phase.

### 4.2 Code/Test Fix Slice

Recommended PR: **CR-1 — verified code/test fixes**.

Candidate file areas:

- `crates/cli/src/commands/health.rs`
- `crates/cli/src/commands/setup.rs`
- `crates/config/src/lib.rs`
- `crates/retrieval/src/lib.rs`
- `crates/storage/src/rate_limits.rs`

Design choices:

1. **CLI health `--json` behavior**
   - Verification must determine whether the command currently advertises local-only behavior while executing provider checks.
   - Preferred minimal behavior decision for implementation: preserve existing live health semantics if users already depend on provider status, but make help text truthful. If product intent is true local-only JSON, that requires a user decision before apply because it changes behavior.
   - Tests should assert the selected contract, not just string wording.

2. **CLI setup provider/model consistency**
   - Setup should derive a selected-provider model once and use it for both connection testing and persisted config.
   - The model derivation boundary should be close to provider selection so later code cannot accidentally reuse `config.embedding.model` from a previous provider.
   - Avoid broad provider abstraction changes in this slice.

3. **Config test determinism**
   - Tests that mutate `CITE_*` environment variables should save/restore prior values through existing mutex patterns.
   - Fallback-default tests should call an isolated loader path such as `Config::load_from(Some(nonexistent_path))` or a temp fixture.
   - This slice should not change runtime config loading semantics.

4. **H19 / ScoredChunk clone removal**
   - Add a reference-based conversion such as `impl From<&ChunkEmbeddingRecord> for ScoredChunk` or an equivalent helper.
   - `rank_by_similarity` should convert from a reference and no longer clone the embedding vector solely to construct output.
   - Ranking order, score values, and output fields must remain stable.

5. **Rate-limit prune validation**
   - `prune_stale_rate_limits` should reject `max_age_seconds <= 0` before computing cutoff or deleting rows.
   - Use the existing project error vocabulary for invalid arguments; do not introduce a new error hierarchy unless needed.
   - Add a test proving no rows are deleted when a non-positive value is supplied.

Validation for CR-1:

- Focused tests for changed behavior where practical.
- Full commands: `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`.

### 4.3 Documentation/Archive Correction Slice

Recommended PR: **CR-2 — verified OpenSpec/archive corrections**.

Candidate file areas:

- `openspec/changes/active/error-remediation-v2/apply-progress.md`
- `openspec/changes/active/error-remediation/second-pass-prompt.md`
- `openspec/reports/archive/revision-repo/**`

Design choices:

- Keep this PR documentation-only unless a verification step discovers a real runtime regression.
- Correct stale claims by saying what current code does now and, if relevant, what historical risk remains.
- Do not rewrite archived reports for style or completeness; target only the stale/inconsistent claims listed in `coderabbit-findings.md` after verification.
- If the documentation diff threatens the 400-line budget, split by report family: runtime guard docs, UTF-8 docs, provider API-key docs, prior-count docs.

Validation for CR-2:

- Markdown review plus targeted grep/reference checks against current code.
- Full Rust validation is optional for docs-only changes but should be run before final V3 verification or if any code changed in the same slice.

## 5. C9 Typed Identifier Migration Design

C9 is the broadest and highest-risk part of V3. It must not be implemented as a single sweeping crate-by-crate rewrite. The design instead uses dependency-aware slices that preserve string boundaries while migrating internal records.

### 5.1 Typed ID Invariants

`DocumentId`, `ChunkId`, and `TraceId` should provide:

- `Display` for stable string rendering.
- `FromStr` for CLI/config/raw input boundaries.
- `AsRef<str>` and/or `Deref<Target = str>` for low-friction call sites.
- `serde(transparent)` semantics so JSON stays string-compatible.
- Explicit constructors or validation if the project decides IDs have format constraints.

External persisted and serialized values remain the same strings unless the tasks phase records a deliberate compatibility break.

### 5.2 C9 Slice Plan

| Slice | Purpose | Main dependency | Target areas | Review strategy |
| --- | --- | --- | --- | --- |
| C9-1 common type capabilities | Ensure newtypes have the traits and tests needed by downstream crates | none | `crates/common/src/types.rs`, common tests | Small foundation PR; no downstream migration |
| C9-2 storage boundary pilot | Migrate a narrow set of storage records/row decoders to typed IDs | C9-1 | storage row structs and conversion helpers for one or two high-value paths | Establish raw-string ↔ typed-ID persistence pattern |
| C9-3 retrieval boundary | Migrate retrieval records and `ScoredChunk` ID fields where compatible | C9-2 and/or CR-1 | `crates/retrieval/src/lib.rs`, storage retrieval records | Keep ranking behavior stable; avoid vector API redesign |
| C9-4 graph/domain boundary | Migrate graph topic/concept/document/chunk identifiers where meaningful | C9-1/C9-2 | `crates/graph/src/**`, related storage rows | Coordinate with timestamp work only if diff remains small |
| C9-5 engine/CLI integration | Parse CLI string IDs into typed IDs at command boundaries and update engine callers | C9-2 through C9-4 | `crates/engine/src/**`, `crates/cli/src/commands/**` | Preserve CLI strings; update fixtures/tests incrementally |
| C9-6 cleanup and public API audit | Remove redundant string conversions and document remaining string boundaries | prior C9 slices | cross-crate cleanup, docs/tests | Only after migrated paths compile and verify |

Each C9 slice must include a before/after boundary note in apply progress identifying which IDs are migrated and which remain raw strings. This prevents accidental half-migrations that are difficult to review.

### 5.3 C9 Data Flow

```text
CLI/config/raw input string
  -> FromStr / constructor boundary
  -> DocumentId / ChunkId / TraceId in migrated domain/storage records
  -> SQLite bind as string via Display/AsRef<str>
  -> SQLite row text
  -> storage decode through typed constructor
  -> domain/retrieval/engine callers use typed IDs
  -> JSON/display serialize as the same string representation
```

Compatibility rule: storage and JSON representations remain string-shaped. Type safety improves inside Rust APIs and row/domain records.

### 5.4 C9 Migration Risks

- Public structs may be consumed by tests or downstream crates expecting `String`.
- Serde derives can change JSON if not transparent.
- SQLite row decoding may need error mapping from parse failures into project errors.
- Introducing `Deref<Target = str>` improves ergonomics but can hide conversion boundaries; tasks should still prefer explicit constructors at IO edges.

Rollback: because each slice leaves external string representation stable, any failed C9 slice should be revertible without database migration rollback. Avoid schema changes in C9 slices.

## 6. Snapshot Correctness Design

Snapshot work has two related but distinct concerns: rollback confidence and `updated_at` persistence.

### 6.1 H7 Rollback Tests

Recommended PR: **SNAP-1 — snapshot rollback confidence**.

Design:

- Add a regression test that simulates a failure during activation after partial pointer work has begun.
- If current SQLite transaction semantics already rollback correctly, do not change production code.
- If leakage is observed, repair transaction handling in the smallest possible area around `activate_snapshot`.

Test expectations:

- Pre-existing current pointer remains current after failed activation.
- Failed snapshot is not visible as current.
- Successful activation still commits atomically.

### 6.2 Snapshot Pointer `updated_at`

Recommended PR: **SNAP-2 — snapshot pointer updated_at migration**, or combine with SNAP-1 only if the total diff remains comfortably under budget and review remains coherent.

Design:

- Add an additive migration for `snapshot_pointer.updated_at`.
- Existing rows receive a valid timestamp or documented default.
- Pointer update/activation paths refresh `updated_at`.
- Timestamp format should match the existing storage convention, preferably RFC3339/UTC if already used elsewhere.

Migration considerations:

- Additive schema change only; no destructive data rewrite.
- Existing databases must load after migration.
- Tests should cover migrating an old schema and updating the timestamp on pointer changes.

Rollback:

- Code rollback after additive migration should tolerate the extra column if queries do not use `SELECT *` assumptions.
- If a migration file is introduced, document that database downgrade is not automatic; application compatibility relies on ignoring additive columns.

## 7. `created_at` DateTime<Utc> Consistency Design

Recommended PR: **TIME-1 — domain timestamp type consistency**.

Target fields from the spec:

- `graph::types::Topic.created_at`
- `graph::types::Concept.created_at`
- `storage::SemanticLinkRow.created_at`

Design:

- Parse storage timestamp strings into `DateTime<Utc>` at row decoding boundaries.
- Format `DateTime<Utc>` back to stable strings at SQLite write and CLI display boundaries.
- Keep JSON output stable where possible by relying on chrono serde defaults only if they match current output; otherwise use explicit formatting or serde helpers.
- Do not combine this with broad C9 slices unless the changed-line budget remains small and the same records are already being touched.

Tests:

- Valid storage timestamp decodes into `DateTime<Utc>`.
- Invalid storage timestamp returns an error at the boundary.
- CLI or JSON output remains stable for representative topic/concept/link rows.

Rollback:

- No schema migration is required if persisted values remain strings.
- Reverting the PR returns domain fields to `String` without data migration.

## 8. Delivery Decision Table

Because the configured PR strategy is ask-always, this table is a forecast, not approval to apply. Before implementation starts, the parent/session should ask the user to approve the first immediate PR slice.

| Proposed PR | Contents | Depends on | Est. risk | Est. review size | Recommendation | User decision needed before apply |
| --- | --- | --- | --- | --- | --- | --- |
| CR-1 | Verified CodeRabbit code/test fixes: health/setup/config tests/retrieval/rate-limit | proposal/spec/design/tasks | Low-Med | <400 if minimal | Do first | Yes |
| CR-2 | Verified CodeRabbit docs/archive corrections | CR-1 optional | Low | <400; split if broad | Separate from CR-1 unless tiny | Yes |
| C9-1 | Common typed-ID trait/construction foundation | tasks | Medium | <400 | Start C9 here | Yes |
| C9-2 | Storage boundary pilot for typed IDs | C9-1 | High | <400 if narrow | Prove row pattern | Yes |
| C9-3 | Retrieval boundary typed IDs | C9-2 / CR-1 | High | <400 | After storage pilot | Yes |
| C9-4 | Graph/domain typed IDs | C9-1/C9-2 | High | <400 only if focused | Keep separate from timestamps unless small | Yes |
| C9-5 | Engine/CLI typed-ID integration | prior C9 slices | High | May need multiple PRs | Split by command/API path | Yes |
| C9-6 | Cleanup/public API audit | prior C9 slices | Medium | <400 | Final typed-ID polish | Yes |
| SNAP-1 | Snapshot rollback regression tests and minimal fix if needed | CR lane optional | Medium | <400 | Can run before/after C9 | Yes |
| SNAP-2 | `snapshot_pointer.updated_at` additive migration | SNAP-1 preferred | Medium | <400 | Keep migration review focused | Yes |
| TIME-1 | `created_at` `DateTime<Utc>` consistency | C9 slices optional | Medium | <400 if limited | Separate from broad C9 | Yes |

## 9. Rollout Plan

1. Write tasks from this design with exact validation steps and per-slice acceptance criteria.
2. Ask the user to approve the first PR slice because PR strategy is ask-always.
3. Apply CR-1 first unless the user chooses to defer CodeRabbit code fixes.
4. Verify CR-1 with full commands.
5. Apply CR-2 docs corrections separately, splitting if docs become too broad.
6. Start C9 with common type capabilities, then one migrated boundary at a time.
7. Run snapshot and timestamp slices as independent PRs when their dependencies are clear.
8. Update V3 progress/tracking after every slice with fixed/skipped/deferred status.
9. Final verify runs all configured commands and confirms no CodeRabbit finding remains unclassified.

## 10. Testing Strategy

Even though `strict_tdd=false`, each implementation slice should include focused regression tests for behavior that can regress.

| Area | Focused validation |
| --- | --- |
| CLI health | Contract test or direct unit test proving selected `--json` behavior |
| CLI setup | Provider/model pairing test for selected provider and saved config |
| Config tests | Env restoration and isolated fallback loading |
| Retrieval | Ranking output stable; no candidate vector clone in construction path by code inspection/test-supported helper |
| Rate limits | Non-positive max age returns error and preserves rows |
| C9 | Serde string stability, FromStr/Display round trips, storage decode/bind paths |
| Snapshot rollback | Failed activation leaves previous pointer intact; success commits |
| Snapshot migration | Old schema gains `updated_at`; update refreshes timestamp |
| DateTime | Valid parse, invalid rejection, stable CLI/JSON formatting |
| Docs/archive | Reference checks against current code symbols before editing |

Full verification gate per approved code slice:

```bash
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

## 11. Risks and Mitigations

| Risk | Impact | Mitigation |
| --- | --- | --- |
| C9 migration exceeds review budget | Large hard-to-review PRs | Enforce C9 slices; stop if a slice approaches 400 lines |
| Typed IDs alter external JSON or SQLite strings | Compatibility break | Use transparent serde and string bind/display boundaries |
| CLI health product ambiguity | Unexpected network/no-network behavior | Require user decision if behavior changes; otherwise correct wording only |
| Docs/archive corrections balloon | Review fatigue | Separate CR-2; split by report family if needed |
| Snapshot migration affects existing DBs | Runtime migration failure | Additive migration with old-schema test |
| DateTime parsing rejects legacy values | Existing data may fail to load | Verify existing timestamp formats before apply; document parse boundary |
| CodeRabbit finding is stale | Wasted churn | Mandatory verification and skip recording |
| Cross-slice conflicts | Rework between C9/timestamp/snapshot | Order dependencies explicitly and avoid combining unrelated migrations |

## 12. Open Decisions Before Apply

These decisions do not block writing tasks, but they do block the relevant implementation slice:

1. Should `health --json` become true local/no-network behavior, or should existing live checks remain with corrected wording?
2. Should CR-1 be the first implementation PR, or should C9 start first despite higher risk?
3. Should CR-2 documentation corrections be separate by default? This design recommends yes.
4. Should C9 target full public API migration in V3, or stop at meaningful storage/retrieval/domain boundaries if budget pressure grows?
5. Should a version bump be included once C9 changes public APIs, likely `v0.3.0`?

## 13. Skill Resolution

`none`

No explicit `SKILL.md` path was injected into this delegated design task. The design was completed using the assigned SDD design-executor instructions and the requested OpenSpec inputs only.
