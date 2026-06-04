# Tasks: Error Remediation V2

**Change ID:** `error-remediation-v2`
**Phase:** tasks
**Date:** 2026-06-02
**Implementation pacing:** approved for a 7-PR stacked-to-main chain, executed over multiple passes/waves to control cost and review load
**Inputs read:** `openspec/changes/error-remediation-v2/design.md`, `openspec/reports/error-tracking.md`, `openspec/reports/revision-repo/analisis-errores-completo.md`, `openspec/config.yaml`
**Input note:** `openspec/changes/error-remediation-v2/spec.md` was requested but is not present; tasks use the design artifact and canonical reports as source of truth.

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~1,550-2,540 total, split into ~160-390 per PR |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR-1 CLI DRY → PR-2a golden fixtures → PR-2b deterministic tests → PR-3 cast/type safety → PR-4 storage/engine cleanup → PR-5 dead code → PR-6 naming/docs/UX |
| Delivery strategy | approved 7-PR chain, implemented over multiple passes/waves to control cost |
| Chain strategy | stacked-to-main |

Decision needed before apply: Completed 2026-06-02 — user approved the 7-PR chain, requested stacked-to-main, and preferred extending implementation across multiple passes/waves rather than reducing scope.
Chained PRs recommended: Yes
Chain strategy: stacked-to-main
400-line budget risk: High

## Apply Gates

- **Gate A — chain approval:** Completed 2026-06-02. User approved the 7-PR chain, selected `stacked-to-main`, and asked to stretch implementation over multiple passes/waves if needed for cost control rather than reduce scope.
- **Gate B — newtype scope:** Completed 2026-06-02. C9/M33 full newtype migration is fully deferred to separate SDD `id-newtype-migration`; no common-only re-export/doc slice in this pass unless later re-approved.
- **Gate C — 400-line budget:** If any PR forecast or actual diff exceeds 390 changed lines during apply, stop and split before coding further.
- **Gate D — no implementation in tasks phase:** This file is planning only.

## Global Verification Gates

After each PR:

```bash
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

Additional final verify checks:

```bash
grep -R "as u32" crates/storage/src
```

Classify every remaining T3/T4/C9/cast item in `openspec/reports/error-tracking.md` during verify as fixed, deferred, duplicate, false alarm, or out of scope.

## Dependency Map

- PR-1 is independent and should go first because CLI DRY reduces later command changes.
- PR-2a and PR-2b are independent after PR-1, but PR-2a may reduce fixture churn for PR-2b.
- PR-3 is independent and includes the 11 unchecked casts still present in `documents.rs`, `traces.rs`, and `rate_limits.rs`.
- PR-4 should follow PR-3 if it reuses checked conversion helpers.
- PR-5 can run after PR-1/PR-3 checks to avoid deleting APIs that become useful.
- PR-6 should be last because naming/docs/UX tasks depend on final behavior.

---

## PR-1: CLI DRY and Retrieval Validation (~220-340 changed lines)

**Themes:** DRY first; CLI error output and duplicated flag validation
**Items:** M1, M2, M3, M4, L5, selected L2
**Primary files:** `crates/cli/src/commands/mod.rs`, `search.rs`, `retrieve.rs`, `context.rs`, `setup.rs`, `read.rs`, representative command tests/discovery targets under `crates/cli/src/commands/*.rs`

### 1.1 RED: inventory CLI duplication and lock current behavior
- **Action:** Inspect `crates/cli/src/commands/{mod.rs,search.rs,retrieve.rs,context.rs,setup.rs,read.rs}` and record exact current JSON/text error shapes for one representative command.
- **Verify:** Add or update focused tests for invalid retrieval flag combinations (`--flat + --topic`, `--flat + --concept`, `--topic + --concept`) without changing production code first.

### 1.2 GREEN: centralize CLI error rendering
- **Action:** Convert `handle_command_error` in `crates/cli/src/commands/mod.rs` into the authoritative helper (for example `exit_for_error(e, json) -> i32`) and use it for repeated error branches in `search.rs`, `retrieve.rs`, `context.rs`, `list.rs`, `get.rs`, `read.rs`, `refresh.rs`, `retry.rs`, and `trace.rs` where mechanically safe.
- **Verify:** Representative JSON/text error tests pass; no command output changes unless test-approved.
- **Rollback:** Revert helper call-site changes; existing direct `print_json`/`eprintln!` branches remain valid.

### 1.3 GREEN: extract retrieval-scope validation
- **Action:** Add a shared helper in `crates/cli/src/commands/mod.rs` or a focused module such as `crates/cli/src/commands/retrieval_args.rs` for mutually exclusive `flat/topic/concept` validation.
- **Action:** Replace duplicated validation in `crates/cli/src/commands/{search.rs,retrieve.rs,context.rs}`.
- **Verify:** `cargo test -p cli` and invalid flag tests pass.

### 1.4 GREEN: fix setup TTY error silence locally
- **Action:** Replace the specific `unwrap_or_default()` in `crates/cli/src/commands/setup.rs` that masks TTY input errors with explicit error handling using the shared renderer if applicable.
- **Verify:** Add one test/discovery note for non-TTY/read failure behavior if test harness supports it; otherwise document manual verification target.

### 1.5 TRIANGULATE/REFACTOR: keep helpers small and reviewable
- **Action:** Ensure helpers preserve SRP: rendering helper only renders/errors; validation helper only validates scope.
- **Verify:** `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`; changed lines <400.

---

## PR-2a: Canonical Golden Fixtures and Evaluation Provider (~250-390 changed lines)

**Themes:** test infrastructure; fixture consistency
**Items:** M13, M14, M24, L4
**Primary files:** `crates/engine/src/golden_provider.rs`, `crates/engine/tests/golden/provider.rs`, `crates/engine/tests/golden/*`, `crates/cli/src/commands/evaluate.rs`

### 2a.1 RED: identify canonical golden fixture drift
- **Action:** Compare `crates/engine/tests/golden/*`, `crates/engine/src/golden_provider.rs`, `crates/engine/tests/golden/provider.rs`, and fixture construction in `crates/cli/src/commands/evaluate.rs`.
- **Verify:** Add failing/characterization tests that prove CLI and engine use inconsistent fixture IDs/expectations or duplicated provider behavior.

### 2a.2 GREEN: make one golden provider authoritative
- **Action:** Delete or replace duplicated test `GoldenProvider` in `crates/engine/tests/golden/provider.rs` with `engine::golden_provider::GoldenProvider` or a test-only constructor from `crates/engine/src/golden_provider.rs`.
- **Verify:** Golden provider tests pass using one implementation.
- **Rollback:** Restore test-local provider if public production provider API would expand too much.

### 2a.3 GREEN: extract canonical fixture definitions
- **Action:** Move repeated golden fixture/corpus setup out of `crates/cli/src/commands/evaluate.rs` into an engine-owned fixture/evaluation module or existing engine API.
- **Action:** Keep `cite evaluate` output shape stable; only source fixture data from canonical definitions.
- **Verify:** Engine golden tests and CLI evaluate tests pass with a single expected fixture source.

### 2a.4 TRIANGULATE: evaluate provider semantics
- **Action:** Add tests around `crates/providers/src/eval.rs` for prompt-injection/compliance keywords that caused false positives, then either tune the small deterministic embedding logic or document intentional limitations.
- **Verify:** `cargo test -p providers -p engine -p cli`.

### 2a.5 REFACTOR/SPLIT CHECK
- **Action:** If fixture extraction plus provider dedup exceeds 390 changed lines, split before implementation into PR-2a1 (provider dedup) and PR-2a2 (fixture canonicalization).
- **Verify:** Full quality gate; changed lines <400.

---

## PR-2b: Deterministic Test Infrastructure and Edge Cases (~200-360 changed lines)

**Themes:** test infrastructure; deterministic default suite
**Items:** M12, M21, M25, M26, M34, M36, M37, L19
**Primary files:** `crates/providers/src/{gemini.rs,openai.rs}`, `crates/retrieval/src/lib.rs`, `crates/config/src/lib.rs`, `crates/ingest/src/validator.rs`, `crates/graph/src/heading_parser.rs`, ignored doctest discovery targets across `crates/storage` and `crates/retrieval`

### 2b.1 RED/GREEN: remove network from default provider tests
- **Action:** For `test_embed_invalid_key_returns_error` in `crates/providers/src/gemini.rs` and `test_embed_invalid_endpoint_returns_error` in `crates/providers/src/openai.rs`, either mark network-dependent tests `#[ignore]` with reason or replace with deterministic mock HTTP tests.
- **Verify:** `cargo test -p providers` passes without internet.

### 2b.2 RED/GREEN: retrieval edge-case tests
- **Action:** Add tests in `crates/retrieval/src/lib.rs` for `cosine_similarity`: opposite, orthogonal, one-dimensional, zero-vector/invalid length behavior.
- **Action:** Add tests for `rank_by_similarity`: empty candidates, `k > candidates`, all-invalid vectors, and deterministic tie behavior if applicable.
- **Verify:** `cargo test -p retrieval`.

### 2b.3 RED/GREEN: config merge/env/TOML coverage
- **Action:** Add tests in `crates/config/src/lib.rs` for merge precedence, invalid env values, TOML sections, and `Default`/`PartialEq` comparison if already available.
- **Action:** If invalid env values are silently ignored by design, create a failing test that captures desired behavior, then implement only if within PR budget; otherwise defer to PR-6.
- **Verify:** `cargo test -p config`.

### 2b.4 TRIANGULATE: reconcile UTF-8 tests already fixed in first pass
- **Action:** Check `crates/ingest/src/validator.rs` and `crates/graph/src/heading_parser.rs` for first-pass UTF-8 tests covering M21/L19.
- **Verify:** If covered, mark as duplicate/already-fixed in verify tracking; if gaps remain, add focused tests only.

### 2b.5 REFACTOR: ignored doc tests policy
- **Action:** Inventory ignored doc tests in `crates/storage` and `crates/retrieval`; for each, convert, keep ignored with reason, or move to integration test.
- **Verify:** `cargo test --doc` if supported plus full quality gate.

---

## PR-3: Cast Safety and Type Consistency (~260-390 changed lines)

**Themes:** type consistency; small safety theme for casts
**Items:** 11 unchecked casts found in first-pass verify, M7, M8, M11, M18, M22, M29, L10, L11, L37, L38
**Primary files:** `crates/storage/src/{documents.rs,traces.rs,rate_limits.rs}`, `crates/common/src/{lib.rs,exit.rs,types.rs,error.rs}`, `crates/config/src/lib.rs`, `crates/graph/src/types.rs`, `crates/storage/src/{topics.rs,concepts.rs}`, `crates/providers/src/{gemini.rs,openai.rs}`

### 3.1 RED: cast overflow/negative tests
- **Action:** Add storage tests proving negative or overflowing SQLite integer values in `documents.rs`, `traces.rs`, and `rate_limits.rs` fail explicitly instead of truncating.
- **Verify:** Tests fail before cast conversion where practical.

### 3.2 GREEN: add checked conversion helpers
- **Action:** Add private helpers in storage (for example `i64_to_u32_field`, `usize_to_u32_field`, and optional variants) in a concrete existing module such as `crates/storage/src/util.rs` if accessible, or a new focused private module.
- **Verify:** Helper unit tests cover valid, negative, and overflow cases.

### 3.3 GREEN: replace remaining unchecked storage casts
- **Action:** Replace `as u32` in `crates/storage/src/documents.rs` lines discovered around `chunk_count`, `retry_count`, `max_retry_count`, and `Ok(n as u32)`.
- **Action:** Replace `as u32` in `crates/storage/src/rate_limits.rs` for `retry_after`.
- **Action:** Replace `as u32` in `crates/storage/src/traces.rs` for optional count/page/offset fields and `excluded_non_ready_document_ids.len() as u32`.
- **Verify:** `grep -R "as u32" crates/storage/src` returns only reviewed safe/out-of-scope casts or no matches.

### 3.4 GREEN: common ergonomics and comparability
- **Action:** Add missing re-exports in `crates/common/src/lib.rs` for `ErrorInfo`, `OffsetRange`, and other documented common public types if present.
- **Action:** Add `ExitCode::as_i32()` in `crates/common/src/exit.rs` and use it opportunistically only where it reduces duplicated casts.
- **Action:** Add `PartialEq` derives/tests for `Document` and `ErrorInfo` in `crates/common/src/types.rs`/`error.rs` if trait bounds allow.
- **Verify:** `cargo test -p common`.

### 3.5 GREEN: boundary validation for provider model/endpoint
- **Action:** Add empty model validation to `crates/providers/src/gemini.rs` and `crates/providers/src/openai.rs` constructors.
- **Action:** Add empty endpoint validation to `crates/providers/src/openai.rs`; keep broader HTTPS/model newtypes deferred.
- **Verify:** Provider constructor tests pass without network.

### 3.6 TRIANGULATE: timestamp type consistency
- **Action:** Evaluate `created_at` types in `crates/graph/src/types.rs`, `crates/storage/src/topics.rs`, and `crates/storage/src/concepts.rs`.
- **Action:** If conversion to `DateTime<Utc>` is local, implement with parsing/serialization tests. If it cascades beyond budget, document string persistence boundary in code/tests and defer migration.
- **Verify:** `cargo test -p graph -p storage`.

### 3.7 REFACTOR/SPLIT CHECK
- **Action:** If cast fixes plus common/type work exceed 390 lines, split PR-3 into PR-3a casts/provider validation and PR-3b common/timestamp consistency.
- **Verify:** Full quality gate; changed lines <400.

---

## PR-4: Storage and Engine Correctness Cleanup (~240-380 changed lines)

**Themes:** storage/engine correctness after type safety
**Items:** M27, M28, M30, M31, H7, H19 decision
**Primary files:** `crates/storage/src/{rate_limits.rs,embeddings.rs,snapshots.rs,migrations/005_snapshots.sql}`, `crates/engine/src/refresh.rs`, `crates/retrieval/src/lib.rs`

### 4.1 RED/GREEN: rate-limit TTL/pruning
- **Action:** Add tests in `crates/storage/src/rate_limits.rs` showing stale windows are pruned or ignored.
- **Action:** Implement pruning in `check_and_increment_rate_limit_at` or a dedicated storage method; no background daemon.
- **Verify:** `cargo test -p storage`.

### 4.2 RED/GREEN: shared embedding row mapper
- **Action:** Add characterization tests for `list_chunk_embeddings_hierarchical` and `list_ready_chunk_embeddings` in `crates/storage/src/embeddings.rs`.
- **Action:** Extract duplicated row mapping into one private helper while preserving returned records.
- **Verify:** Storage tests pass and diff remains readable.

### 4.3 RED/GREEN: corrupt vector blob handling
- **Action:** Add a test inserting a corrupt embedding blob and asserting explicit storage error or intentionally tracked rejection.
- **Action:** Replace silent skip behavior around `decode_vector_blob` in `crates/storage/src/embeddings.rs` with the chosen policy.
- **Verify:** `cargo test -p storage`.

### 4.4 TRIANGULATE: snapshot updated_at and activation rollback
- **Action:** For M31, add `updated_at` to snapshot pointer only if migration and tests fit under budget; otherwise defer with reason.
- **Action:** For H7 in `crates/engine/src/refresh.rs`, add rollback/mark-failed tests and implement only if local; otherwise defer to a snapshot-architecture PR.
- **Verify:** `cargo test -p engine -p storage`.

### 4.5 DECISION: ScoredChunk duplication
- **Action:** Inspect `crates/retrieval/src/lib.rs` public API impact for H19.
- **Action:** If a `From<ChunkEmbeddingRecord>` implementation removes duplication without API break, implement and test. If wrapping changes API, defer to separate architecture/API-change SDD.
- **Verify:** `cargo test -p retrieval`.

---

## PR-5: Dead Code, Placeholders, and Low-Risk Cleanup (~160-320 changed lines)

**Themes:** dead code cleanup
**Items:** M15, M16, M19, M20, L1, L3, L15-L18, L20-L34 low-risk subset
**Primary files:** `crates/engine/src/lib.rs`, `crates/engine/Cargo.toml`, `crates/graph/src/{lib.rs,types.rs}`, `crates/cli/src/output.rs`, `crates/ingest/src/chunker.rs`, `crates/cli/src/commands/*.rs`, `crates/storage/src/*`

### 5.1 RED: prove placeholders are unused or compatibility-required
- **Action:** Search direct uses/re-exports of `Engine`, `Graph`, `graph::SemanticLink`, and `into_compact_*` helpers.
- **Verify:** Document each as delete, keep-with-doc, or wire-to-use before production changes.

### 5.2 GREEN: remove or justify empty structs
- **Action:** Remove `pub struct Engine;` in `crates/engine/src/lib.rs` and `pub struct Graph;` in `crates/graph/src/lib.rs` if no public compatibility tests fail.
- **Action:** If keeping either, add doc comment and minimal test proving intended purpose.
- **Verify:** Workspace compile/tests pass.

### 5.3 GREEN: semantic link/domain cleanup
- **Action:** Remove unused `graph::SemanticLink` from `crates/graph/src/types.rs` and re-exports if it is not used by storage or public callers.
- **Action:** Do not remove `crates/storage/src/semantic_links.rs` rows unless proven unused and covered by tests.
- **Verify:** `cargo test -p graph -p storage`.

### 5.4 GREEN: compact output and unreachable code cleanup
- **Action:** In `crates/cli/src/output.rs`, either wire `into_compact_context/search/retrieve` into actual compact output paths or delete dead functions/attributes.
- **Action:** Remove unreachable match branch in `crates/ingest/src/chunker.rs` with a targeted test.
- **Verify:** `cargo test -p cli -p ingest`.

### 5.5 GREEN: dependency/import cleanup
- **Action:** Remove unused `tracing` from `crates/engine/Cargo.toml` only if still unused.
- **Action:** Check `serde_json` direct dependency in providers and other low-risk dependency claims before deleting.
- **Verify:** `cargo build`, `cargo clippy -- -D warnings`.

### 5.6 TRIANGULATE: low-risk L items only
- **Action:** Address low-risk local items such as read-only startup recovery or mock-provider detection only if they have concrete tests and stay under budget.
- **Verify:** Full quality gate; defer any command architecture rewrite.

---

## PR-6: Naming, Documentation, Setup/Health UX, and Focused Modularization (~220-360 changed lines)

**Themes:** naming/docs/minor cleanup; files >150 lines only when directly tied to remediation
**Items:** M5, M6, M9, M10, L9, L12-L14, selected L2/L7 and docs
**Primary files:** `crates/cli/src/commands/{setup.rs,health.rs,evaluate.rs}`, `crates/config/src/lib.rs`, `crates/common/src/*`, `openspec/reports/error-tracking.md` during verify

### 6.1 RED/GREEN: setup saves tested model/endpoint
- **Action:** Add config round-trip tests proving `save_config` persists the provider model/endpoint tested by setup.
- **Action:** Fix `crates/cli/src/commands/setup.rs` so tested and saved config match.
- **Verify:** `cargo test -p cli -p config`.

### 6.2 RED/GREEN: health local/network behavior
- **Action:** Add tests or explicit command-mode checks around `crates/cli/src/commands/health.rs` for local-state-only vs provider/network checks.
- **Action:** Either remove surprise network calls from local health or expose/document a flag/path for network checks.
- **Verify:** CLI tests and manual command target documented.

### 6.3 RED/GREEN: honest config errors
- **Action:** Add tests for `load_from` TOML parse failures and invalid env values in `crates/config/src/lib.rs`.
- **Action:** Either return real errors or rename/document permissive behavior; do not silently ignore invalid user configuration unless tests call that policy intentional.
- **Verify:** `cargo test -p config`.

### 6.4 GREEN: deprecation warning and path naming docs
- **Action:** Clarify `CITE_API_KEY` deprecation warning text in `crates/config/src/lib.rs` so it says fallback is accepted but deprecated.
- **Action:** Decide whether default config path `cite` vs `aiharness` is intentional; if unchanged, document reason in tracking/docs.
- **Verify:** Config tests around env fallback pass.

### 6.5 REFACTOR: focused module extraction for large files only when useful
- **Action:** If touched sections in `setup.rs`, `health.rs`, or `evaluate.rs` exceed 150-line cognitive load, extract only cohesive helpers tied to this PR (setup persistence helpers, health check modes, or previously planned fixture setup).
- **Verify:** No broad folder restructure; changed lines <400.

### 6.6 DOCS: public API documentation and non-exhaustive decision
- **Action:** Add doc comments to externally visible enums/structs that lack intent comments and are touched by this pass.
- **Action:** Add `#[non_exhaustive]` only where downstream crate stability is an actual goal; otherwise defer as not applicable to workspace-only APIs.
- **Verify:** `cargo doc --no-deps` if practical; otherwise `cargo test` + clippy.

---

## Phase N: Newtype Migration Scope (Deferred)

**Items:** C9, M33
**Primary files for future SDD:** `crates/common/src/types.rs`, cross-crate call sites across storage/engine/ingest/retrieval/CLI/tests

### N.1 Tracking-only task
- **Action:** During verify, update `openspec/reports/error-tracking.md` to mark full `DocumentId`/`ChunkId`/`TraceId` migration as deferred to a separate SDD, recommended change ID `id-newtype-migration`.
- **Reason:** Full migration is ~50 files and exceeds this pass/review budget.

### N.2 Optional common-only task (requires Gate B approval)
- **Action:** If approved before apply, re-export existing newtypes from `crates/common/src/lib.rs` and add doc comments describing intended boundaries.
- **Verify:** `cargo test -p common`.
- **Non-goal:** Do not migrate storage, engine, retrieval, CLI, traces, or fixtures in this pass.

---

## Deferred / Out of Scope

| Item | Scope decision |
|---|---|
| Full C9/M33 newtype migration | Separate SDD (`id-newtype-migration`) due ~50-file footprint. |
| Broad repository restructure or new crates | Out of scope; only focused module extraction tied to a remediation task. |
| Public API redesign for `ScoredChunk` if wrapping breaks callers | Defer to architecture/API-change SDD. |
| Snapshot migration/rollback if PR-4 exceeds budget | Split or defer; do not bundle oversized storage migration. |
| Low-priority cosmetic L items without tests | Defer unless local, testable, and under budget. |
| Version bump | Decide in verify/release planning after actual implementation scope is known. |

## Tracking Update Plan

During apply/verify, update `openspec/reports/error-tracking.md` but do not mark items complete before implementation. Planning status only may state: `error-remediation-v2 tasks created; apply pending user approval`.
