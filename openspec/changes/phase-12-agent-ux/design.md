# Phase 12 Design — Agent UX

## Architecture

### Compact/Full flow

```
CLI command (--json)
  │
  ├─ --full flag present?
  │   ├─ YES → serialize full response types directly (current behavior)
  │   └─ NO → transform to compact types, serialize compact
  │
  └─ No --json → human-readable output (unchanged)
```

### Compact type mapping

```
ContextResponse ──→ CompactContextResponse
  result_kind       → result_kind
  trace_id          → trace_id
  citations[]       → citations[]
    citation_id       → id
    display_name      → source
    text              → snippet (truncated to 200 chars)
    score             → score

SearchOutput ──→ CompactSearchOutput
  results[]         → results[]
    chunk_id          → id
    display_name      → source
    score             → score
    preview           → preview (unchanged, already ≤160 chars)

RetrieveOutput ──→ CompactRetrieveOutput
  results[]         → results[]
    chunk_id          → id
    display_name      → source
    score             → score
    text              → text (unchanged)
```

## Slices

### Slice A — Compact Response Types + Transform

**Files**:
- `crates/cli/src/output.rs` (new)

**Changes**:

1. Add compact types:
```rust
#[derive(Serialize)]
struct CompactContextResponse {
    result_kind: ResultKind,
    citations: Vec<CompactCitation>,
    trace_id: String,
}

#[derive(Serialize)]
struct CompactCitation {
    id: String,
    source: String,
    snippet: String,
    score: Option<f64>,
}

#[derive(Serialize)]
struct CompactSearchOutput {
    results: Vec<CompactSearchItem>,
}

#[derive(Serialize)]
struct CompactSearchItem {
    id: String,
    source: String,
    score: f32,
    preview: String,
}

#[derive(Serialize)]
struct CompactRetrieveOutput {
    results: Vec<CompactRetrieveItem>,
}

#[derive(Serialize)]
struct CompactRetrieveItem {
    id: String,
    source: String,
    score: f32,
    text: String,
}
```

2. Add transform functions:
```rust
pub fn to_compact_context(resp: &ContextResponse) -> CompactContextResponse
pub fn to_compact_search(output: &SearchOutput) -> CompactSearchOutput
pub fn to_compact_retrieve(output: &RetrieveOutput) -> CompactRetrieveOutput
```

3. `to_compact_context` truncates snippet to 200 chars max

**Estimated lines**: ~80

---

### Slice B — --full Flag on CLI Commands

**Files**:
- `crates/cli/src/commands/context.rs` (modify)
- `crates/cli/src/commands/search.rs` (modify)
- `crates/cli/src/commands/retrieve.rs` (modify)

**Changes**:

1. Add `--full` clap arg to each command:
```rust
#[arg(long, help = "Return full JSON response (default: compact)")]
full: bool,
```

2. In JSON output path, check `--full`:
```rust
if args.full {
    println!("{}", serde_json::to_string_pretty(&response)?);
} else {
    let compact = to_compact_context(&response);
    println!("{}", serde_json::to_string_pretty(&compact)?);
}
```

3. Apply to all three commands

**Estimated lines**: ~140

---

### Slice C — Fix Search/Retrieve Breadcrumb Passthrough

**Files**:
- `crates/cli/src/commands/search.rs` (modify)
- `crates/cli/src/commands/retrieve.rs` (modify)

**Changes**:

1. Add to `SearchResultItem`:
```rust
topic_name: Option<String>,
concept_name: Option<String>,
breadcrumb: Option<String>,
```

2. Add to `RetrieveResultItem`:
```rust
topic_name: Option<String>,
concept_name: Option<String>,
breadcrumb: Option<String>,
```

3. Populate from engine's `SearchHit`/`RetrieveHit` fields

**Estimated lines**: ~20

---

### Slice D — Consolidate Eval Providers

**Files**:
- `crates/providers/src/eval.rs` (new or modify)
- `crates/cli/src/commands/evaluate.rs` (modify)
- `crates/engine/tests/golden_test.rs` (modify)

**Changes**:

1. Move `EvalProvider` from CLI to `crates/providers/src/eval.rs`
2. Re-export from `crates/providers/src/lib.rs`
3. Update CLI evaluate to use shared `EvalProvider`
4. Update engine golden_test to use shared `EvalProvider` instead of `GoldenProvider`
5. Remove duplicate `GoldenProvider`

**Estimated lines**: ~60

---

### Slice E — Hierarchical Fixtures

**Files**:
- `crates/engine/tests/golden/fixtures.json` (modify)
- `crates/cli/src/commands/evaluate.rs` (modify)

**Changes**:

1. Add 2 hierarchical fixtures to fixtures.json:
   - **hier-001**: "What database does the system use?" → expects context with breadcrumb
   - **hier-002**: "How are passwords validated?" → expects context, breadcrumb contains topic

2. Add corresponding entries in CLI evaluate's `build_fixtures()`

3. Update fixture count assertions (8 → 10)

**Estimated lines**: ~80

---

### Slice F — Tests + Verification

**Files**:
- `crates/cli/src/output.rs` (tests)
- Various test files

**Tests**:
1. Compact context transform produces correct fields
2. Compact context truncates snippet to 200 chars
3. Compact search transform
4. Compact retrieve transform
5. Full mode preserves all fields
6. Search/retrieve breadcrumb passthrough
7. All 223 existing tests pass
8. All 10 fixtures pass

**Estimated lines**: ~150

---

## Dependency graph

```
A (compact types) ──┐
C (breadcrumb fix) ─┼── B (--full flag) ── F (tests)
D (eval consolidate) ┘── E (hierarchical fixtures) ── F
```

A, C, D can start in parallel. B depends on A. E depends on D. F depends on all.

## Workload forecast

| Slice | Lines | Risk |
|-------|-------|------|
| A | ~80 | Low |
| B | ~140 | Low |
| C | ~20 | Low |
| D | ~60 | Low |
| E | ~80 | Medium |
| F | ~150 | Low |
| **Total** | **~530** | |
