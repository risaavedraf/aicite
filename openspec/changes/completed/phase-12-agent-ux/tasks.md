# Phase 12 Tasks — Agent UX (Compact/Full Mode + Evaluation)

## Slice A — Compact Response Types + Transform

**Goal**: Create compact response types and transform functions in CLI.

**Allowlist**:
- `crates/cli/src/output.rs` (new)
- `crates/cli/src/lib.rs` or `crates/cli/src/main.rs` (mod declaration)

**Estimated lines**: ~80

**Changes**:
1. Create `output.rs` with compact types: `CompactContextResponse`, `CompactCitation`, `CompactSearchOutput`, `CompactSearchItem`, `CompactRetrieveOutput`, `CompactRetrieveItem`
2. Add `to_compact_context(resp: &ContextResponse) -> CompactContextResponse` — maps fields, truncates snippet to 200 chars
3. Add `to_compact_search(output: &SearchOutput) -> CompactSearchOutput`
4. Add `to_compact_retrieve(output: &RetrieveOutput) -> CompactRetrieveOutput`
5. Re-export from CLI module

**Dependencies**: None

---

## Slice B — --full Flag on CLI Commands

**Goal**: Add --full flag to context, search, retrieve commands.

**Allowlist**:
- `crates/cli/src/commands/context.rs` (modify)
- `crates/cli/src/commands/search.rs` (modify)
- `crates/cli/src/commands/retrieve.rs` (modify)

**Estimated lines**: ~140

**Changes**:
1. Add `#[arg(long)] full: bool` to each command struct
2. In JSON output path: if `--full`, serialize full response; else serialize compact
3. Non-JSON output path: `--full` has no effect

**Dependencies**: Slice A

---

## Slice C — Fix Search/Retrieve Breadcrumb Passthrough

**Goal**: Include Phase 11 breadcrumb fields in search/retrieve JSON output.

**Allowlist**:
- `crates/cli/src/commands/search.rs` (modify)
- `crates/cli/src/commands/retrieve.rs` (modify)

**Estimated lines**: ~20

**Changes**:
1. Add `topic_name`, `concept_name`, `breadcrumb` to `SearchResultItem`
2. Add same to `RetrieveResultItem`
3. Populate from engine's `SearchHit`/`RetrieveHit`

**Dependencies**: None

---

## Slice D — Consolidate Eval Providers

**Goal**: Unify EvalProvider and GoldenProvider into single shared implementation.

**Allowlist**:
- `crates/providers/src/eval.rs` (new or modify)
- `crates/providers/src/lib.rs` (re-export)
- `crates/cli/src/commands/evaluate.rs` (modify)
- `crates/engine/tests/golden_test.rs` (modify)

**Estimated lines**: ~60

**Changes**:
1. Create/move `EvalProvider` to `crates/providers/src/eval.rs`
2. Re-export from providers lib
3. Update CLI evaluate to use shared provider
4. Update engine golden_test to use shared provider
5. Remove duplicate `GoldenProvider`

**Dependencies**: None

---

## Slice E — Hierarchical Fixtures

**Goal**: Add 2 hierarchical evaluation fixtures.

**Allowlist**:
- `crates/engine/tests/golden/fixtures.json` (modify)
- `crates/cli/src/commands/evaluate.rs` (modify)

**Estimated lines**: ~80

**Changes**:
1. Add hier-001 fixture: query about database, expects context with breadcrumb
2. Add hier-002 fixture: query about passwords, expects breadcrumb with topic
3. Add corresponding entries in CLI evaluate's `build_fixtures()`
4. Update fixture count assertions (8 → 10)

**Dependencies**: Slice D

---

## Slice F — Tests + Verification

**Goal**: Full test suite, backward compat verification.

**Allowlist**:
- All test files
- `openspec/changes/phase-12-agent-ux/` (verification artifacts)

**Estimated lines**: ~150

**Changes**:
1. Tests for compact transform functions
2. Tests for --full flag behavior
3. Tests for breadcrumb passthrough
4. Run full suite: `cargo test`, `cargo clippy`, `cargo fmt`
5. Verify all 223 existing tests still pass
6. Verify all 10 fixtures pass

**Dependencies**: All previous slices

---

## Summary

| Slice | Description | Est. Lines | Depends On |
|-------|-------------|------------|------------|
| A | Compact types + transform | ~80 | — |
| B | --full flag | ~140 | A |
| C | Breadcrumb passthrough | ~20 | — |
| D | Consolidate eval providers | ~60 | — |
| E | Hierarchical fixtures | ~80 | D |
| F | Tests + verification | ~150 | All |
| **Total** | | **~530** | |

**Implementation order**: A → C → D → B → E → F

**Parallelization**: A, C, D can start simultaneously.
