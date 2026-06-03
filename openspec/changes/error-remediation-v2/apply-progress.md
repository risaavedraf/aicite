# Apply Progress: error-remediation-v2

## Wave 1 / PR-1 — CLI DRY and Retrieval Validation

**Status:** applied, verify gate passed locally.
**Workload / PR boundary:** stacked-to-main PR-1 only; code diff forecast remains under 390 changed lines (fresh review command-code numstat: 178 insertions + 164 deletions = 342). No PR-2+ scope implemented.

### Completed tasks

- 1.1 Added focused RED tests for invalid retrieval scope combinations and representative error exit-code behavior.
- 1.2 Added canonical `exit_for_error` helper in `commands/mod.rs` and used it in mechanically safe CLI command error branches.
- 1.3 Added shared `validate_retrieval_scope` helper and wired `search`, `retrieve`, and `context` to it.
- 1.3a Fixed fresh-review regression: `validate_retrieval_scope` now returns `hierarchy_override: Option<bool>` so commands preserve `config.retrieval.use_hierarchy` when `--flat` is absent, and only force hierarchy off when `--flat` is present.
- 1.4 Replaced setup API-key prompt `unwrap_or_default()` with explicit dialog error handling.
- 1.5 Kept helpers small: one renderer, one retrieval-scope validator.

### Files changed

- `crates/cli/src/commands/mod.rs`
- `crates/cli/src/commands/search.rs`
- `crates/cli/src/commands/retrieve.rs`
- `crates/cli/src/commands/context.rs`
- `crates/cli/src/commands/setup.rs`
- `crates/cli/src/commands/read.rs`
- `crates/cli/src/commands/{get,list,refresh,retry}.rs`
- `openspec/reports/error-tracking.md`

### Test / verification evidence

| Step | Command | Result |
|---|---|---|
| RED | `cargo test -p cli invalid_retrieval_scope` before helpers | failed to compile: missing `validate_retrieval_scope` / `exit_for_error` |
| GREEN | `cargo test -p cli` | pass: 19 tests |
| REVIEW FIX | `cargo test -p cli` after hierarchy-preservation fix | pass: 20 tests |
| VERIFY | `cargo test` | pass |
| VERIFY | `cargo clippy -- -D warnings` | pass |
| VERIFY | `cargo fmt --check` | pass |

### Deviations / notes

- Invalid retrieval flag errors now flow through `CiteError::InvalidParameter`, enabling the same JSON/text renderer. This intentionally canonicalizes the text prefix to `Error: Invalid parameter: ...` for those early validation paths.
- Fresh review caught a temporary behavior regression where no `--flat` forced hierarchy on. Fixed by making hierarchy changes an explicit override (`Some(false)` for `--flat`, `None` otherwise), preserving configured hierarchy behavior.
- `trace.rs` was left unchanged after the budget check to keep command-code changed lines comfortably below 390.

### Remaining tasks

- PR-2a through PR-6 remain pending.
- Full C9/M33 newtype migration remains deferred to separate SDD `id-newtype-migration`.

## Wave 2 / PR-2a — Canonical Golden Fixtures and Evaluation Provider

**Status:** applied, verify gate passed locally.
**Workload / PR boundary:** stacked-to-main PR-2a only. Initial scoped code numstat was 115 insertions + 254 deletions = 369 changed lines; fresh review requested deriving integration-test fixture metadata from canonical fixtures, which increased scoped PR-2a diff. No PR-2b+ scope implemented.

### Completed tasks

- 2a.1 Inspected golden fixture/provider drift: CLI `evaluate` had its own fixture builder, engine golden tests had file-local fixtures with two expectation mismatches, and `crates/engine/tests/golden/provider.rs` duplicated the production `engine::golden_provider::GoldenProvider` implementation.
- 2a.2 Removed the duplicated test-only golden provider and switched engine golden tests plus CLI evaluate to the authoritative `engine::golden_provider::GoldenProvider`.
- 2a.3 Added canonical `engine::evaluate::golden_fixtures()` and made `cite evaluate` use it instead of a CLI-local fixture builder. CLI output shape remains unchanged.
- 2a.3 Reconciled `amb-001` and `pi-001` fixture metadata in `crates/engine/tests/golden/fixtures.rs` with the canonical evaluate fixtures.
- 2a.3a Fixed fresh-review blocker: engine golden integration fixtures now derive shared fixture IDs, queries, categories, expected result kinds, and minimum citations from `engine::evaluate::golden_fixtures()`, overlaying only integration-test-specific assertions.
- 2a.4 Deferred provider semantic tuning for `providers::eval::EvalProvider` because changing prompt-injection/compliance dimensions would alter golden retrieval behavior and belongs in PR-2b or a focused eval-semantics slice.
- 2a.5 Kept fixture/provider dedup below the split checkpoint.

### Files changed

- `crates/cli/src/commands/evaluate.rs`
- `crates/engine/src/evaluate.rs`
- `crates/engine/tests/golden/fixtures.rs`
- `crates/engine/tests/golden/provider.rs` (deleted duplicate)
- `crates/engine/tests/golden_test.rs`
- `openspec/reports/error-tracking.md`

### Test / verification evidence

| Step | Command | Result |
|---|---|---|
| RED/CHARACTERIZE | Inspection of CLI fixture builder vs engine golden fixtures/provider | Found duplicated provider and mismatched `amb-001`/`pi-001` expectations before code changes |
| GREEN | `cargo test -p engine -p cli` | pass: cli 20 tests, engine 53 unit tests, golden integration 3 tests, runtime mode 3 tests |
| REVIEW FIX | `cargo test -p engine -p cli` after canonical fixture derivation fix | pass |
| VERIFY | `cargo test` | pass |
| VERIFY | `cargo clippy -- -D warnings` | pass |
| VERIFY | `cargo fmt --check` | pass |

### Deviations / notes

- Full corpus seeding extraction from `crates/cli/src/commands/evaluate.rs` was intentionally not moved in PR-2a to limit scope. The duplicated provider implementation is removed, and shared fixture expectations now have one canonical engine source.
- `providers::eval::EvalProvider` prompt-injection/compliance false-positive tuning was documented as deferred rather than changed to avoid destabilizing golden behavior in the same review slice.
- A first targeted `cargo test -p engine -p cli` retry hit a transient Windows linker `LNK1104` because the test executable was locked; immediate retry passed.

### Remaining tasks

- PR-2b through PR-6 remain pending.
- Full C9/M33 newtype migration remains deferred to separate SDD `id-newtype-migration`.
