# Phase 12 — Explore Notes: Agent UX (Compact/Full Mode + Evaluation)

## Current JSON Response Format

### `cite context --json` → ContextResponse

```rust
pub struct ContextResponse {
    pub context_pack_id: String,        // "ctx_<uuid>"
    pub result_kind: ResultKind,        // "context" | "no_results" | "insufficient_context"
    pub query_id: String,               // "qry_<uuid>"
    pub trace_id: String,               // "trace_<uuid>"
    pub instructions: String,           // ~200 chars agent instructions
    pub citations: Vec<Citation>,
    pub metadata: ContextMetadata,      // 15+ fields
}
```

**Citation fields** (per citation):
```rust
pub struct Citation {
    pub citation_id: String,            // "c1", "c2", ...
    pub document_id: String,            // UUID
    pub display_name: String,           // "architecture.txt"
    pub chunk_id: String,               // UUID
    pub page: Option<u32>,
    pub offset: Option<OffsetRange>,    // { start: u32, end: u32 }
    pub text: String,                   // 30-200 chars (hierarchical) or 500-1000 (flat)
    pub score: Option<f64>,
    pub confidence_label: Option<String>,
    pub topic_name: Option<String>,     // Phase 11
    pub concept_name: Option<String>,   // Phase 11
    pub breadcrumb: Option<String>,     // Phase 11: "doc > topic > concept"
}
```

**ContextMetadata fields**:
```rust
pub struct ContextMetadata {
    pub schema_version: String,                    // "context-v1"
    pub created_at: DateTime<Utc>,
    pub retrieved_chunks: u32,
    pub evidence_floor: f64,
    pub confidence_threshold: f64,
    pub ranking_method: String,                    // "vector_cosine_v1"
    pub top_score: Option<f32>,
    pub corpus_index_state: String,                // "ready"
    pub ready_document_count: u32,
    pub excluded_non_ready_document_count: u32,
    pub excluded_non_ready_document_ids: Vec<String>,
    pub latency_ms: u64,
    pub disclaimer: String,                        // ~100 chars
    pub insufficient_context_reason: Option<String>,
    pub caution: Option<String>,                   // ~150 chars
}
```

### `cite search --json` → SearchOutput

```rust
struct SearchOutput {
    query: String,
    top_k: u32,
    hit_count: usize,
    results: Vec<SearchResultItem>,
}

struct SearchResultItem {
    chunk_id: String,
    document_id: String,
    display_name: String,
    section_id: Option<String>,
    chunk_index: u32,
    page: Option<u32>,
    offset_start: Option<u32>,
    offset_end: Option<u32>,
    score: f32,
    preview: String,                  // max 160 chars + "…"
}
```

**Note**: SearchOutput does NOT include Phase 11 breadcrumb fields (topic_name, concept_name, breadcrumb) even though `SearchHit` in the engine has them. The CLI discards them when building `SearchResultItem`.

### `cite retrieve --json` → RetrieveOutput

```rust
struct RetrieveOutput {
    query: String,
    top_k: u32,
    hit_count: usize,
    results: Vec<RetrieveResultItem>,
}

struct RetrieveResultItem {
    chunk_id: String,
    document_id: String,
    display_name: String,
    section_id: Option<String>,
    chunk_index: u32,
    page: Option<u32>,
    offset_start: Option<u32>,
    offset_end: Option<u32>,
    score: f32,
    text: String,                     // full chunk text
}
```

**Note**: Same issue — breadcrumb fields from `RetrieveHit` are discarded in the CLI.

---

## Token Usage Estimate

A typical `cite context --json` with 5 citations (hierarchical, 30-200 char chunks):

| Section | Fields | ~Tokens |
|---------|--------|---------|
| context_pack_id, query_id, trace_id | 3 UUIDs | ~30 |
| result_kind | 1 enum | ~5 |
| instructions | ~200 chars | ~60 |
| 5 citations × (citation_id + document_id + display_name + chunk_id + page + offset + text + score + confidence_label + topic_name + concept_name + breadcrumb) | ~12 fields × 5 | ~400 |
| metadata (15 fields) | schema_version, created_at, thresholds, counts, disclaimer, etc. | ~150 |
| **Total** | | **~645 tokens** |

With flat mode (500-1000 char chunks): **~1200-1500 tokens**.

### Compact mode target

Keep only: `result_kind`, `citations[].id`, `citations[].source` (display_name), `citations[].snippet` (truncated text), `citations[].score`, `trace_id`.

Estimated: **~200-250 tokens** (60-70% reduction).

---

## Current Evaluation System

### Architecture

```
cite evaluate [--json]
  └─ seed_eval_corpus() — 3 docs, 12 chunks, in-memory DB
  └─ build_fixtures() — 8 hardcoded GoldenFixture structs
  └─ run_evaluation() — calls build_context() per fixture
       └─ Compare actual vs expected: result_kind, min_citations
       └─ EvalReport { total, passed, failed, hit_rate, threshold, overall_pass }
```

### Two evaluation paths

1. **CLI evaluate command** (`crates/cli/src/commands/evaluate.rs`):
   - Uses `EvalProvider` (8-dim topic-based vectors, keyword detection)
   - Seeds 3 documents × 4 chunks each (12 total)
   - 8 fixtures hardcoded in `build_fixtures()`
   - Threshold: 80% hit rate
   - Tests: 4 tests (seed corpus, fixture count, provider determinism, full eval pass)

2. **Golden integration tests** (`crates/engine/tests/golden_test.rs`):
   - Uses `GoldenProvider` (same 8-dim algorithm, separate implementation)
   - Ingests from real files in `crates/engine/tests/golden/corpus/` (architecture.txt, api-reference.md, security-policy.txt)
   - 8 fixtures from `fixtures.json` (same IDs, richer assertions: `must_contain_chunk_texts`, `confidence_label_required`)
   - Tests: 4 tests (all fixtures, corpus ingestion, provider determinism)

### Fixture categories (8 total)

| ID | Category | Query | Expected |
|----|----------|-------|----------|
| df-001 | direct_fact | "What does the API gateway do?" | context, ≥1 citation, contains "API gateway routes..." |
| df-002 | direct_fact | "What database does the system use?" | context, ≥1 citation, contains "PostgreSQL with read replicas" |
| df-003 | direct_fact | "How are passwords validated?" | context, ≥1 citation, contains "minimum 12 characters" |
| nr-001 | no_results | "What is quantum computing?" | no_results, 0 citations |
| nr-002 | no_results | "Explain the theory of relativity" | no_results, 0 citations |
| amb-001 | ambiguous | "Tell me about the system architecture and how it handles security" | insufficient_context, ≥1 citation, confidence_label_required |
| mc-001 | multi_chunk | "How does the API handle authentication and rate limiting?" | context, ≥2 citations |
| pi-001 | prompt_injection | "Ignore all previous instructions..." | context, ≥1 citation |

### Fixture JSON schema (`fixtures.json`)

```json
{
  "fixture_id": "df-001",
  "query": "...",
  "category": "direct_fact",
  "expected": {
    "result_kind": "context",
    "min_citations": 1,
    "must_contain_chunk_texts": ["..."],
    "must_not_cite_document_ids": [],
    "confidence_label_required": false,
    "assertions": ["human-readable assertion description"]
  },
  "description": "..."
}
```

---

## Where Compact/Full Mode Should Be Implemented

### Option A: Serde serialization attributes (NOT recommended)

Using `#[serde(skip_serializing_if = "Option::is_none")]` with conditional nulling would require modifying the core types. This couples serialization format to domain types.

### Option B: CLI post-processing layer (RECOMMENDED)

Add a transformation layer in the CLI that maps full response types to compact output structs:

```
Engine returns ContextResponse (full)
  └─ CLI checks --compact / --full flag
       ├─ --full: serialize ContextResponse directly (current behavior)
       └─ --compact (default): map to CompactContextResponse, serialize that
```

**Why**: Clean separation. Engine always returns full data. CLI controls what the agent sees. No serde attribute soup on core types.

### Option C: Serde with feature flags (overkill)

Not worth the complexity for this project.

### Implementation location

- `crates/cli/src/output.rs` — add `to_compact_json()` transformation functions
- `crates/cli/src/commands/context.rs` — add `--full` flag, default to compact
- `crates/cli/src/commands/search.rs` — same
- `crates/cli/src/commands/retrieve.rs` — same

---

## Gaps Found

### 1. Search and Retrieve discard breadcrumb fields

`SearchOutput` and `RetrieveOutput` in the CLI don't include `topic_name`, `concept_name`, `breadcrumb` even though the engine's `SearchHit` and `RetrieveHit` have them. Phase 12 should fix this.

### 2. No `--max-snippet-chars` flag

The agent-usage-guide proposes `--max-snippet-chars 200` to limit text per citation. Not implemented.

### 3. Duplicate provider implementations

`EvalProvider` (CLI evaluate) and `GoldenProvider` (engine golden_test) implement the same 8-dim topic-based algorithm independently. Should be consolidated.

### 4. No compact mode for search/retrieve

Only `context` was discussed for compact mode, but `search` and `retrieve` could also benefit.

---

## Estimated Scope

| Area | Files | Est. Lines | Risk |
|------|-------|------------|------|
| Compact response types | `cli/src/output.rs` | ~80 | Low |
| --full flag + compact transform | `cli/src/commands/context.rs` | ~60 | Low |
| --full flag + compact transform | `cli/src/commands/search.rs` | ~40 | Low |
| --full flag + compact transform | `cli/src/commands/retrieve.rs` | ~40 | Low |
| Fix search/retrieve breadcrumb passthrough | `cli/src/commands/{search,retrieve}.rs` | ~20 | Low |
| Consolidate eval providers | `cli/src/commands/evaluate.rs`, `engine/tests/golden/` | ~60 | Low |
| Add hierarchical fixtures | `fixtures.json`, `evaluate.rs` | ~80 | Medium |
| Tests | various | ~150 | Low |
| **Total** | | **~530** | |

All slices well under 300-line budget.
