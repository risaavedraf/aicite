# Phase 12 Archive — Agent UX

## Status: COMPLETE ✅

## Summary

Implemented compact/full JSON response mode for CLI commands and evaluation improvements. Default compact mode reduces token usage by 60-70% for agents.

## Deliverables

| Deliverable | Status |
|-------------|--------|
| Compact response types + transform functions | ✅ |
| --full flag on context/search/retrieve | ✅ |
| Breadcrumb passthrough in search/retrieve JSON | ✅ |
| Shared EvalProvider (consolidated) | ✅ |
| 2 hierarchical evaluation fixtures | ✅ |
| 6 compact transform tests | ✅ |
| Verify report | ✅ |

## Test Results

- **228 tests pass** (was 223, +5 compact transforms, +1 count update)
- clippy clean
- fmt clean
- 10 golden fixtures pass (8 + 2 hierarchical)

## Token Usage

| Mode | Tokens | Reduction |
|------|--------|-----------|
| `--json --full` | ~645-1500 | baseline |
| `--json` (compact, default) | ~200-250 | **60-70%** |

## Files Changed

| File | Change |
|------|--------|
| `crates/cli/src/output.rs` | New — compact types + transforms |
| `crates/providers/src/eval.rs` | New — shared EvalProvider |
| `crates/providers/src/lib.rs` | +pub mod eval |
| `crates/cli/src/commands/context.rs` | +--full flag, compact JSON default |
| `crates/cli/src/commands/search.rs` | +--full flag, +breadcrumb fields |
| `crates/cli/src/commands/retrieve.rs` | +--full flag, +breadcrumb fields |
| `crates/cli/src/commands/evaluate.rs` | Uses shared EvalProvider |
| `crates/engine/tests/golden_test.rs` | Uses shared EvalProvider |
| `crates/engine/tests/golden/fixtures.rs` | +hier-001, hier-002 |
| `crates/engine/tests/golden/fixtures.json` | +2 hierarchical fixtures |

## SDD Artifacts

- `openspec/changes/phase-12-agent-ux/proposal.md`
- `openspec/changes/phase-12-agent-ux/specs/` (2 domain specs)
- `openspec/changes/phase-12-agent-ux/design.md`
- `openspec/changes/phase-12-agent-ux/tasks.md`
- `openspec/changes/phase-12-agent-ux/verify-report.md`
