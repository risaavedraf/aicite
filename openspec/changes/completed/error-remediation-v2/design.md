# Design: Error Remediation V2

**Change ID:** `error-remediation-v2`
**Phase:** design
**Date:** 2026-06-02
**Inputs:** `specs/error-remediation/spec.md`, `openspec/reports/error-tracking.md`, `openspec/reports/revision-repo/analisis-errores-completo.md`, first-pass `openspec/changes/error-remediation/tasks.md`
**Preflight:** interactive, artifact store both requested, chained PR strategy ask-always, review budget 400 changed lines

## Executive Summary

This design plans the second remediation pass as a sequence of theme-based, reviewable slices for the remaining T3/T4 quality inventory plus the unchecked casts discovered during first-pass verification. It intentionally avoids a crate-by-crate rewrite and keeps full `DocumentId`/`ChunkId`/`TraceId` migration out of this pass because that migration is approximately 50 files and would violate review focus.

The proposed delivery is an ask-always chain of 7 implementation PRs plus one separate newtype SDD decision. No implementation may start until the user approves this delivery strategy.

## Source Inventory Notes

The source reports describe the second-pass scope as **78 remaining T3+T4 errors** plus **11 unchecked casts**. The detailed inventory table in `analisis-errores-completo.md` lists 37 medium and 38 low items (75), with additional deferred/high-adjacent items and C9 newtype work referenced separately. Implementation tasks must reconcile the count in `openspec/reports/error-tracking.md` during verify and mark each item as fixed, deferred, duplicate, or false alarm.

First-pass artifacts show that T1/T2 core fixes are complete, but several T3/T4 examples are still visible in code, including repeated CLI error output branches, duplicated retrieval flag validation, provider tests that can hit the network, `as u32` casts in storage row decoding paths, dead placeholder structs, and large evaluation fixture setup in `crates/cli/src/commands/evaluate.rs`.

## Design Principles Applied

From the loaded code-quality skill and references:

- **DRY:** centralize repeated CLI error output and retrieval flag validation without changing user-facing semantics.
- **SRP / small functions:** move shared fixture/provider/test helpers to focused modules; do not turn large files into broad rewrites.
- **Rust error handling:** use `Result`, `TryFrom`, `?`, and explicit storage errors instead of lossy casts or silent defaults.
- **Type clarity:** prefer `DateTime<Utc>` at domain boundaries; document persistence string exceptions when SQLite storage requires strings.
- **YAGNI:** do not perform full newtype migration in this pass; plan it as a separate SDD unless a narrowly bounded boundary is approved.
- **Review discipline:** each PR stays below 400 changed lines and has its own quality gate.

## Delivery Strategy and Ask-Always Gate

Because `chained_pr_strategy` is `ask_always`, this design recommends a chained delivery but requires approval before `sdd-apply`.

```text
Decision needed before apply: Yes
Recommended chain: PR-1 -> PR-2a -> PR-2b -> PR-3 -> PR-4 -> PR-5 -> PR-6
Review budget: <400 changed lines per PR
Newtype migration: separate SDD, not bundled
```

## PR / Phase Forecast

| PR | Theme | Primary Items | Est. Changed Lines | Risk | Dependencies |
|---:|---|---|---:|---|---|
| 1 | CLI DRY and validation | M1, M2, M3, M4, L5, selected L2 coverage | 220-340 | Medium | None |
| 2a | Golden fixtures and evaluation test doubles | M13, M14, M24, L4 | 250-390 | Medium | PR-1 optional |
| 2b | Deterministic edge-case tests | M12, M25, M26, M34, M36, M37, M21/L19 reconciliation | 200-360 | Low-Medium | None |
| 3 | Cast safety and type consistency | 11 casts, M7, M8, M11, M18, M22, M29, L10/L11/L37/L38 | 260-390 | Medium | None |
| 4 | Storage/engine correctness cleanup | M27, M28, M30, M31, H7/H19 decision if still deferred | 240-380 | Medium-High | PR-3 recommended |
| 5 | Dead code and dependency cleanup | M15, M16, M19, M20, L1, L3, L15-L18, L20-L34 where low-risk | 160-320 | Low-Medium | None |
| 6 | Naming, docs, and focused modularization | M5, M6, M9, M10, L9, L12-L14, public docs | 220-360 | Medium | PR-1/3 helpful |
| N | Newtype migration scope | C9/M33 | design-only or separate SDD | High if bundled | Separate approval |

If any PR forecast crosses 400 changed lines during tasks/apply, split it before coding and ask the user again.

## Phase Designs

### PR-1: CLI DRY and Retrieval Validation

**Goal:** make repeated CLI error/validation behavior consistent without changing command results.

**Design:**
- Promote the existing private `handle_command_error` pattern in `crates/cli/src/commands/mod.rs` into the authoritative CLI error renderer.
- Add a small `CommandResult` helper or `exit_for_error(e, json) -> i32` wrapper so commands can return the same exit code and JSON/text shape.
- Replace duplicated retrieval-scope checks in `search`, `retrieve`, and `context` with a shared `validate_retrieval_scope(flat, topic, concept) -> Result<RetrievalScopeArgs, CiteError>` or equivalent helper.
- Keep error messages identical unless tests intentionally lock a clearer canonical message.
- Do not refactor every command output branch in one pass; prioritize error branches and retrieval validation first to control line count.

**Tests:**
- Unit tests for invalid `--flat + --topic`, `--flat + --concept`, and `--topic + --concept` combinations.
- JSON/text equivalence assertions for one representative command path.
- Existing CLI tests must still pass.

**Review risk:** medium, because command behavior is user-facing. Keep helper minimal and avoid broad command rewrites.

### PR-2a: Canonical Golden Fixtures and Evaluation Provider

**Goal:** remove contradictory fixture definitions and duplicated `GoldenProvider` maintenance.

**Design:**
- Make engine evaluation fixtures the canonical source for golden behavior.
- Move duplicated CLI fixture construction out of `crates/cli/src/commands/evaluate.rs` when directly tied to canonicalization. A suitable target is an engine evaluation fixture module or an existing engine evaluation API.
- Use a single provider test double for golden evaluation; if `engine::golden_provider::GoldenProvider` is production-facing only for `cite evaluate`, expose a test-friendly constructor rather than duplicating a second provider in tests.
- Keep `evaluate` command behavior and output format stable; only change where its corpus/fixtures/provider come from.

**Large-file/modularization note:** `crates/cli/src/commands/evaluate.rs` is already a large command module (>400 lines from direct read). Modularize only the fixture/corpus setup that belongs to this theme. Do not split unrelated rendering code in this PR.

**Tests:**
- Engine golden tests and `cite evaluate` use the same fixture IDs and expectations.
- A fixture expectation update should require one canonical edit.

**Review risk:** medium; fixture movement can create many changed lines. If over 390 lines, split corpus extraction from provider deduplication.

### PR-2b: Deterministic Test Infrastructure and Edge Cases

**Goal:** make default tests deterministic and fill high-value edge-case gaps.

**Design:**
- Mark provider tests that perform live HTTP as `#[ignore]` or replace them with deterministic mock HTTP behavior. Default `cargo test` must not require network.
- Add retrieval edge-case tests for cosine similarity and ranking: empty candidates, `k > candidates`, all-invalid vectors, one-dimensional vectors, orthogonal/opposite vectors, and deterministic ordering for ties if applicable.
- Add config merge/env/TOML tests for invalid env handling and precedence. Do not overhaul config loading unless a test demonstrates behavior that violates the spec.
- Reconcile already-fixed UTF-8/offset tests from first pass and mark M21/L19 as fixed, duplicate, or already covered in tracking.
- Review ignored doc tests and decide per test: keep ignored with reason, convert to normal test, or move to integration test.

**Tests:** this PR is test-heavy by design; production code changes should be minimal.

**Review risk:** low-medium; many assertions but low behavioral risk.

### PR-3: Cast Safety and Type Consistency

**Goal:** eliminate remaining lossy casts and improve type comparability without broad model rewrites.

**Design:**
- Introduce private storage conversion helpers such as `i64_to_u32_field(field, value) -> Result<u32, CiteError>`, `opt_i64_to_u32_field(...)`, and, where needed, `i64_to_u64_field(...)`.
- Replace remaining unchecked `as u32` casts in `crates/storage/src/documents.rs`, `crates/storage/src/traces.rs`, and `crates/storage/src/rate_limits.rs` with these helpers.
- For values derived from in-memory lengths (for example `Vec::len()`), use `u32::try_from(len)` if the destination type is `u32`.
- Add tests that insert negative and overflowing SQLite values where practical and assert decoding fails instead of truncating.
- Add or expose lightweight ergonomics in `common` (`ExitCode::as_i32`, missing re-exports, `PartialEq` derives for `Document`/`ErrorInfo`) only where tests or public API consistency benefit.
- For `created_at` inconsistencies (`graph::types`, `storage::{topics,concepts}`), prefer `DateTime<Utc>` in domain structs if line count remains safe. If conversion would cascade beyond budget, document the string-persistence boundary and add parsing/comparison tests.
- Add provider constructor validation for empty model/endpoint (M22) as boundary validation; keep full `ApiKey`/`ModelId`/`Endpoint` newtypes deferred to the separate newtype SDD.

**Review risk:** medium. Storage conversions touch critical persistence paths; tests must cover valid and invalid decoding.

### PR-4: Storage and Engine Correctness Cleanup

**Goal:** address medium-impact persistence/correctness debt after cast helpers exist.

**Design:**
- Add a TTL/pruning mechanism for rate-limit counters (M27) that deletes expired windows during `check_and_increment_rate_limit_at` or via a small dedicated method. Avoid background daemons.
- Refactor duplicated embedding row mapping between `list_chunk_embeddings_hierarchical` and `list_ready_chunk_embeddings` into one private mapper/query helper (M28).
- Change vector blob decode handling so corrupt blobs produce an explicit storage error or tracked rejection instead of being silently skipped (M30). If existing behavior is intentionally tolerant, document and test that policy.
- Add `updated_at` to snapshot pointer handling only if it can be done with a small migration and tests (M31); otherwise defer with reason.
- Revisit H7 snapshot activation rollback and H19 `ScoredChunk` duplication. If still deferred from first pass, handle only if the local diff is below budget; otherwise keep as separate architecture/API-change task.

**Review risk:** medium-high due to storage behavior and migration risk. Split before apply if migration plus row-mapper refactor exceeds budget.

### PR-5: Dead Code, Placeholders, and Low-Risk Dependency Cleanup

**Goal:** remove or justify unused items while avoiding speculative architecture.

**Design:**
- Remove empty placeholder structs with no methods/callers (`Engine`, `Graph`) unless public compatibility requires keeping them; if kept, add documented purpose and minimal behavior/test.
- Remove or justify `graph::SemanticLink` if unused by production code; coordinate with existing `storage::SemanticLinkRow` so a real domain type is not accidentally removed.
- Remove `into_compact_*` helpers or wire them to actual compact output behavior. If compact output is future work, delete the allow-dead-code attribute and track the future feature separately.
- Remove unused dependencies/imports such as engine `tracing` only if still unused.
- Replace unreachable match branches in chunking code with clearer control flow.
- For low-severity items such as `read.rs` manual validation, startup recovery for read-only commands, and hardcoded mock-provider detection, apply only changes that are locally testable and do not expand into a command architecture rewrite.

**Review risk:** low-medium. Main risk is public API removal; tasks must check downstream re-exports before deletion.

### PR-6: Naming, Documentation, Setup/Health UX, and Focused Modularization

**Goal:** make names/docs match behavior and clean up user-facing inconsistencies.

**Design:**
- Fix setup persistence so the model/endpoint tested is the model/endpoint saved (M5), with a config round-trip test.
- Clarify `health` behavior: either remove surprise provider/network checks from the local-state path or document/flag them explicitly (M6).
- Make `load_from` and invalid env behavior honest: either return real TOML/env errors or rename/document permissive behavior (M9/M10).
- Clarify deprecated `CITE_API_KEY` warning language so it says fallback is accepted but deprecated, not ignored (L14).
- Add targeted doc comments to public enums/structs/traits that are externally visible; do not add comments that merely restate code.
- Consider `#[non_exhaustive]` only for public enums intended for downstream crate stability; this workspace may not require it.
- Modularize files over 150 lines only when directly tied to the above remediation. Candidate extractions: setup config persistence helpers, health check modes, evaluation fixture setup already covered by PR-2a. Do not perform a broad folder restructure.

**Review risk:** medium because setup/health UX can affect users. Keep behavior changes test-backed.

### Phase N: Newtype Migration Scope (C9/M33)

**Decision:** full migration is deferred to a separate SDD change, recommended ID `id-newtype-migration`.

**Rationale:** `DocumentId`, `ChunkId`, and `TraceId` touch common, storage, engine, ingest, retrieval, CLI, traces, tests, and fixtures. The expected footprint is around 50 files, which is not compatible with the 400-line review budget or the second-pass remediation scope.

**Allowed in this pass:**
- Update tracking to record C9/M33 as deferred to a named SDD.
- Optionally add a tiny common-only task to re-export the types and document the intended boundary if approved, but do not partially migrate unrelated call sites.

**Separate SDD should define:**
- Boundary conversions at storage and CLI inputs.
- Whether IDs implement `Deref<Target=str>`, `AsRef<str>`, `Display`, `FromStr`, and serde transparent encoding.
- Crate-by-crate PR chain with each PR below 400 lines.

## Data and Control Flow Changes

- CLI commands continue to receive parsed args and config, but error rendering and retrieval-scope validation flow through shared helpers before command-specific work.
- Evaluation command continues to seed an in-memory database, but fixture definitions and golden provider behavior come from one canonical engine-side source.
- Storage row decoding converts SQLite `i64` values through checked helper functions before constructing domain records.
- Provider unit tests no longer call external services in the default suite; integration/network tests are opt-in.
- Dead code removal either deletes unused API surface or documents why public compatibility requires retaining it.

## Verification Plan

After each approved PR:

```bash
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

Additional targeted checks:

- `cargo test -p cli` after PR-1 and PR-6.
- `cargo test -p engine -p providers -p retrieval -p config` after PR-2a/2b as applicable.
- `cargo test -p storage -p common -p graph` after PR-3/4/5 as applicable.
- `grep -R "as u32" crates/storage/src` after PR-3, then classify any remaining cast as safe, converted, or outside scope.
- Update `openspec/reports/error-tracking.md` during verify with fixed/deferred/false-alarm status for every T3/T4/C9/cast item.

## Rollout / Review Policy

- No code implementation starts in design phase.
- Before apply, ask the user to approve the chained strategy and whether to include the optional common-only newtype re-export/documentation slice.
- If any PR exceeds 400 changed lines, stop, split the PR by subtheme, and ask again.
- Prefer small local helpers over new crates or broad module moves.
- Preserve existing CLI output unless a spec scenario requires unifying behavior.

## Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Inventory count mismatch (78 vs detailed 75 + C9/deferred items) | Tracking gaps | Reconcile in tasks/verify and record duplicates/false alarms explicitly. |
| CLI DRY refactor changes user-facing output | UX regression | Add equivalence tests and avoid unnecessary message changes. |
| Golden fixture consolidation creates large diff | Review budget breach | Split fixture extraction from provider deduplication if forecast exceeds 390 lines. |
| Cast safety changes surface latent bad data | Tests may fail with old fixtures | Treat failures as useful; add clear storage errors and migration notes if needed. |
| Timestamp type migration cascades | Scope explosion | Prefer documented conversion boundaries if direct `DateTime<Utc>` migration exceeds budget. |
| Dead code removal breaks public API | Compile/API regression | Check re-exports and tests; keep documented compatibility stubs only when justified. |
| Newtype migration too broad | Violates scope/budget | Defer to separate SDD; do not partial-migrate without boundary approval. |
| Engram persistence requested but unavailable in delegated toolset | Persistence gap | Persist OpenSpec artifact and report memory unavailability in phase result. |
