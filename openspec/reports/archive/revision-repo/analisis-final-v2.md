# Análisis Consolidado v2 — aiharness

**Fecha:** 2026-06-02
**Fuentes:** `analisis-funcionamiento.md`, `analisis-errores-completo.md`, `analisis-final.md`
**Objetivo:** Cross-reference de 113 errores contra la arquitectura funcional del sistema, con estrategia de fix priorizada y recomendación SDD.

---

## 1. Executive Summary

### Project Purpose
aiharness es un harness de desarrollo con IA en Rust que produce un CLI (`cite`) para ingerir documentos, generar embeddings vectoriales, y ejecutar búsqueda semántica con citas y trazabilidad sobre un corpus local en SQLite.

### Error Inventory (113 unique errors)

| Tier | Severity | Count |
|------|----------|------:|
| T1 | 🔴 Critical | 11 |
| T2 | 🟠 High | 19 |
| T3 | 🟡 Medium | 37 |
| T4 | 🟢 Low | 38 |
| **Total** | | **113** |

### Top 3 Systemic Risks

1. **UTF-8 bytes-vs-characters confusion (4 crates, 6 locations).** Every offset, truncation, and character-count operation uses `str::len()` (bytes) instead of `chars().count()`. This silently corrupts hierarchy assignment, chunk boundaries, and metadata for any non-ASCII text — and causes a runtime panic in display name sanitization. All existing tests use ASCII-only data, making this invisible to CI. This is the single highest-impact bug in the entire project.

2. **Config-consumption gap (4 crates, 5+ fields).** `IngestConfig` and other config structs define fields like `embedding_timeout_secs`, `max_chunk_chars`, and `production_mode` guard — but downstream crates never read them. Users believe they've configured timeouts, chunk limits, and runtime guards; none of which actually work. This creates a false sense of control and blocks production readiness.

3. **Silenced error pattern (3 crates, 6 locations).** Storage uses `.ok()` to convert all DB errors to `None`, engine uses `let _ =` to ignore cleanup failures, CLI uses `.unwrap_or_default()` to swallow missing API keys. The pattern is consistent across the codebase: genuine errors are silenced at crate boundaries, causing orphan data, cryptic downstream failures, and invisible data corruption.

### Recommended Fix Scope for First SDD Pass

**Include:** All 11 Critical (T1) errors + 12 High (T2) errors that directly affect data integrity, security, onboarding, or pipeline correctness. Total: **~23 errors, ~180-220 lines of change across 7 crates.**

**Exclude:** DRY refactoring, dead code cleanup, test infrastructure, newtype migration, and architectural redesign. These are important but lower risk and higher effort — defer to second pass.

---

## 2. Architecture Map with Error Overlay

### 2.1 Crate Dependency Graph with Error Counts

```
                          ┌───────────┐
                          │  common   │  🔴1 🟠2 🟡2 🟢3 = 8 errors
                          │ (types,   │  ← CiteError missing PartialEq
                          │  errors,  │  ← Newtypes dead code
                          │  exits)   │  ← Re-exports incomplete
                          └─────┬─────┘
                                │
           ┌────────────────────┼────────────────────┐
           │                    │                    │
     ┌─────▼─────┐      ┌──────▼──────┐      ┌─────▼──────┐
     │   config   │      │  providers  │      │   graph    │
     │            │      │             │      │            │
     │ 🟠2 🟡4 🟢3│      │ 🔴1 🟠3 🟡3│      │ 🔴1 🟠2 🟡3│
     │  = 9 err   │      │  🟢4 =11err│      │  🟢4 =10err│
     │            │      │             │      │            │
     │←timeout    │      │←hardcoded   │      │←byte offset│
     │  ignored   │      │  30s, empty │      │  ROOT CAUSE│
     │←TOML gaps  │      │  key silent │      │  of chain  │
     │←field      │      │             │      │            │
     │  confusion │      │             │      │            │
     └─────┬─────┘      └──────┬──────┘      └─────┬──────┘
           │                   │                    │
     ┌─────▼───────────────────┼────────────────────▼──────┐
     │                    ┌────▼─────┐                      │
     │                    │  ingest  │                      │
     │                    │ 🔴3 🟠3 🟡2                      │
     │                    │ 🟢3 =11err│                      │
     │                    │          │                      │
     │                    │←UTF-8    │                      │
     │                    │  panic   │                      │
     │                    │←byte     │                      │
     │                    │  offsets │                      │
     │                    │←wrong    │                      │
     │                    │  metadata│                      │
     │                    └────┬─────┘                      │
     │                         │                            │
     │                   ┌─────▼──────┐                     │
     │                   │  storage   │                     │
     │                   │ 🔴2 🟠2    │                     │
     │                   │ 🟡5 🟢4    │                     │
     │                   │  = 13 err  │                     │
     │                   │            │                     │
     │                   │←FK off     │                     │
     │                   │←.ok()      │                     │
     │                   │←i64→u32    │                     │
     │                   │←no TTL     │                     │
     │                   └─────┬──────┘                     │
     │                         │                            │
     │                   ┌─────▼──────┐  ┌─────────────┐    │
     │                   │  retrieval │  │             │    │
     │                   │ 🟠1 🟡2    │  │             │    │
     │                   │ 🟢3 = 6err │  │             │    │
     │                   │            │  │             │    │
     │                   │←ScoredChunk│  │             │    │
     │                   │  dup fields│  │             │    │
     │                   └─────┬──────┘  │             │    │
     │                         │         │             │    │
     └─────────────────────────┼─────────┘             │
                               │                       │
                         ┌─────▼───────┐               │
                         │   engine    │ ◄─────────────┘
                         │ 🔴2 🟠3     │
                         │ 🟡4 🟢3     │
                         │  = 12 err   │
                         │             │
                         │←dead guard  │
                         │←false pos.  │
                         │←silenced    │
                         │  errors     │
                         │←orphan      │
                         │  snapshots  │
                         └─────┬───────┘
                               │
                         ┌─────▼───────┐
                         │     cli     │
                         │ 🔴1 🟠5     │
                         │ 🟡6 🟢8     │
                         │  = 20 err   │
                         │             │
                         │←empty key   │
                         │←json dup    │
                         │←unwrap      │
                         │  inconsist. │
                         │←DRY x14    │
                         └─────────────┘
```

### 2.2 Error Distribution by Pipeline Relevance

| Crate | Ingest Pipeline | Search/Context Pipeline | Refresh Pipeline | Recovery | Evaluate |
|-------|:---:|:---:|:---:|:---:|:---:|
| common | C11 | C11 | — | — | C11 |
| config | H13, H14 | H13, H14 | — | — | — |
| providers | C7, H15, H16 | C7, H15 | — | — | M36, M37 |
| graph | **C3**, H8, H9 | H8, H9 | — | — | — |
| ingest | **C2, C4, C5**, H10-H12 | — | — | — | — |
| storage | **C1**, H17, H18 | C1, H17, H18 | H17 | — | — |
| retrieval | — | H19, M25, M26 | — | — | H19 |
| engine | **C6**, C8, H6, H7 | C8, H5, H7 | H7 | L3 | M13, M14 |
| cli | C7, H3, H4 | H1, H3 | — | L3 | H1 |

**Bold** = Critical tier errors in the pipeline.

### 2.3 Cross-Crate Error Chains

**Chain 1: UTF-8 Hierarchy Corruption (CRITICAL — affects ingest pipeline)**
```
graph/heading_parser.rs:17,35     char_offset += line.len() [bytes]
         │
         ▼
ingest/lib.rs:130-161             topic_boundaries uses heading.char_offset [bytes]
                                  chunk_offsets from chunker are char-based
         │
         ▼
Chunks assigned to WRONG topic/concept for non-ASCII text
         │
         ▼
storage persists wrong hierarchy → retrieval returns wrong metadata
```

**Chain 2: UTF-8 Metadata Propagation (CRITICAL — affects ingest pipeline)**
```
ingest/extractor.rs:37            total_chars = content.len() [bytes]
         │
         ▼
storage/documents stores inflated char count
         │
         ▼
Any consumer trusting total_chars gets wrong data
```

**Chain 3: UTF-8 Runtime Panic (CRITICAL — affects ingest pipeline)**
```
ingest/validator.rs:97-98         trimmed[..255] byte-slice truncation
         │
         ▼
Multi-byte UTF-8 boundary → panic! at runtime
         │
         ▼
cite ingest crashes on emoji/accented/CJK filenames
```

**Chain 4: Dead Security Guard (CRITICAL — affects ingest pipeline)**
```
engine/runtime_guard.rs:23-34     check_ingest_allowed() defined + tested, never called
         │
         ▼
cli/commands/ingest.rs:64         production_mode bool only controls display name
         │
         ▼
engine/ingest.rs:41-57            production_mode → sanitize only, no block
         │
         ▼
Production mode users can ingest without restriction
```

**Chain 5: Silent Empty API Key (CRITICAL — affects ingest + search pipelines)**
```
cli/commands/mod.rs:94            resolve_api_key().unwrap_or_default() → ""
         │
         ▼
providers/gemini.rs:24             accepts "" silently
providers/openai.rs:29             accepts "" silently
         │
         ▼
First embed() → HTTP 401 "Unauthorized" (cryptic, no hint about missing key)
```

**Chain 6: Config-Disconnect (HIGH — affects ingest + search pipelines)**
```
config/lib.rs:101                  embedding_timeout_secs: 30 (from env/file)
config/lib.rs:126                  min_chunk_size_chars: 100
config/lib.rs:94                   min_chunk_chars: 30 (confusing duplicate)
config/lib.rs:128                  max_chunk_chars: 200 (< chunk_size_chars: 1000!)
         │
         ▼
providers/gemini.rs:31             Duration::from_secs(30) hardcoded
providers/openai.rs:34             Duration::from_secs(30) hardcoded
         │
         ▼
ingest/sentence_chunker.rs         ignores max_chunk_chars entirely
         │
         ▼
Config fields exist but have zero runtime effect
```

**Chain 7: Silenced Storage Errors (HIGH — affects all pipelines)**
```
storage/snapshots.rs:68-73        .ok() converts ALL errors to None
storage/embeddings.rs:122,155     continue on decode failure (invisible)
engine/ingest.rs:215-220          let _ = cleanup_partial() errors
         │
         ▼
DB errors silently swallowed → orphan rows, corrupt BLOBs invisible, cleanup failures accumulate
```

---

## 3. Error Impact on Data Flows

### 3.1 Ingest Pipeline

**Path:** `cli → engine → ingest → graph → storage → providers`

**Errors affecting this pipeline (by tier):**

| Error ID | Tier | Description | Location | Impact |
|----------|:----:|-------------|----------|--------|
| **C1** | 🔴 | FK disabled | storage/lib.rs:34-40 | Orphan rows: chunks without docs, embeddings without chunks |
| **C2** | 🔴 | UTF-8 panic in truncation | ingest/validator.rs:97-98 | Runtime crash on non-ASCII filenames |
| **C3** | 🔴 | heading_parser byte/char offset | graph/heading_parser.rs:17,35 | Chunks assigned to wrong topic/concept |
| **C4** | 🔴 | sentence_chunker byte/char offsets | ingest/sentence_chunker.rs:42,47-48,58-59 | Wrong chunk boundaries for multi-byte text |
| **C5** | 🔴 | extractor `total_chars` uses bytes | ingest/extractor.rs:37 | Inflated metadata propagated to storage |
| **C6** | 🔴 | `check_ingest_allowed` dead code | engine/runtime_guard.rs:23-34 | Production mode has no ingest guard |
| **C7** | 🔴 | Empty API key → cryptic 401 | cli/commands/mod.rs:94 | Onboarding broken |
| **C10** | 🔴 | `production_mode` misleading name | engine/ingest.rs:47,74,119 | Developer confusion (enables C6) |
| H3 | 🟠 | Provider unwrap inconsistency | cli/commands/ingest.rs:66 | Panic if provider creation fails |
| H4 | 🟠 | `ingest --queued` creates provider needlessly | cli/commands/ingest.rs:60-67 | Fails offline for queued mode |
| H6 | 🟠 | `cleanup_partial` ignores DB errors | engine/ingest.rs:215-220 | Orphan data on failure |
| H10 | 🟠 | sentence_chunker `min_chars` uses bytes | ingest/sentence_chunker.rs:42 | Wrong merge decisions |
| H11 | 🟠 | sentence_chunker `offset_end` uses bytes | ingest/sentence_chunker.rs:47-48,58-59 | Wrong chunk boundaries |
| H12 | 🟠 | Off-by-one in `min_chars` threshold | ingest/sentence_chunker.rs:42 | Exact-length chunks merged incorrectly |
| H13 | 🟠 | IngestConfig field confusion | config/lib.rs:94-103,126-128 | Ambiguous chunk parameters |
| H14 | 🟠 | TOML can't set most fields | config/lib.rs:356-411 | Config-file users lack access to half of settings |
| H15 | 🟠 | Timeout config ignored by providers | providers/gemini.rs:31, openai.rs:34 | Timeout not configurable |
| H18 | 🟠 | i64→u32 casts without overflow check | storage/util.rs:37,42,47 + more | Silent data corruption for large values |

**Worst-case user-visible impact:**
- Runtime crash (`cite ingest documento.pdf` with emoji in filename → panic)
- Silent data corruption (Japanese/Spanish/French documents → wrong topic assignment, wrong chunk boundaries)
- Security bypass (Production mode users can ingest without guard)
- Failed onboarding (new user without API key gets cryptic HTTP 401)

**This pipeline has the highest error density: 21 errors, 8 critical.**

### 3.2 Search/Context Pipeline

**Path:** `cli → engine → providers → retrieval → storage`

**Errors affecting this pipeline:**

| Error ID | Tier | Description | Location | Impact |
|----------|:----:|-------------|----------|--------|
| C1 | 🔴 | FK disabled | storage/lib.rs:34-40 | Can retrieve orphan embeddings |
| C7 | 🔴 | Empty API key → cryptic 401 | cli/commands/mod.rs:94 | Search fails with unhelpful error |
| C8 | 🔴 | Rate limit key doesn't fulfill FR-109 | engine/retrieve.rs:273-275 | Multi-corpus rate limit shared incorrectly |
| C11 | 🔴 | CiteError no PartialEq | common/error.rs:7 | Test quality degraded |
| H1 | 🟠 | `--json` flag duplicated in evaluate | cli/commands/evaluate.rs:14-18 | `cite evaluate --json` produces non-JSON |
| H3 | 🟠 | Provider unwrap inconsistency | cli/commands/context.rs:50, retrieve.rs:80 | Panic on provider failure |
| H5 | 🟠 | `required_facets` false positives | engine/context.rs:62-80 | Wrong citation count required |
| H7 | 🟠 | Snapshot refresh not fully atomic | engine/refresh.rs:34-54 | Stale snapshot data after failed refresh |
| H17 | 🟠 | `activate_snapshot` uses `.ok()` | storage/snapshots.rs:68-73 | DB errors invisible |
| H18 | 🟠 | i64→u32 casts unchecked | storage/embeddings.rs:144,148,155 | Wrong embedding IDs |
| H19 | 🟠 | ScoredChunk duplicates fields | retrieval/lib.rs:40-64 | Maintenance burden, alignment risk |
| M13 | 🟡 | Golden fixtures inconsistent 4× | engine/tests/golden/* + cli | Evaluate results unreliable |

**Worst-case user-visible impact:**
- Wrong citation counts (queries with "e" or "en" get `required_citations=2` when `1` is correct)
- Rate limiting applies to wrong scope (multi-corpus setups share counters)
- Search works but hierarchy metadata is corrupted (if ingest ran on non-ASCII docs first)
- `cite evaluate --json` returns human-readable output (broken agent integration)

**Priority assessment:** This pipeline has fewer critical errors than ingest, but errors C8 and H5 directly affect search result quality. Fix after ingest pipeline.

### 3.3 Refresh Pipeline

**Path:** `cli → engine → storage`

**Errors affecting this pipeline:**

| Error ID | Tier | Description | Location | Impact |
|----------|:----:|-------------|----------|--------|
| H7 | 🟠 | Snapshot refresh not fully atomic | engine/refresh.rs:34-54 | Orphan "building" snapshots if activate fails |
| H17 | 🟠 | `activate_snapshot` uses `.ok()` | storage/snapshots.rs:68-73 | DB errors silenced during swap |
| L32 | 🟢 | Superseded snapshots never cleaned | storage/snapshots.rs | Linear storage growth |

**Worst-case user-visible impact:**
- Snapshot stuck in "building" state after activation failure (requires manual DB cleanup)
- Silent storage growth from accumulated superseded snapshots

**Priority:** Low. Refresh is infrequent and the blast radius is contained. Fix H7 in the second pass.

### 3.4 Recovery Pipeline

**Path:** `cli → engine → storage` (runs on startup for every command except `health`/`setup`)

**Errors affecting this pipeline:**

| Error ID | Tier | Description | Location | Impact |
|----------|:----:|-------------|----------|--------|
| L3 | 🟢 | Runs on read-only commands | cli/main.rs | Unnecessary latency on `list`, `get`, `read`, `trace` |

**Worst-case user-visible impact:** Extra ~50-100ms latency on read-only commands. No data corruption.

**Priority:** Lowest. Cosmetic/performance only.

### 3.5 Evaluate Pipeline

**Path:** `cli → engine → in-memory DB + EvalProvider`

**Errors affecting this pipeline:**

| Error ID | Tier | Description | Location | Impact |
|----------|:----:|-------------|----------|--------|
| H1 | 🟠 | `--json` flag duplicated | cli/commands/evaluate.rs:14-18 | Broken JSON output |
| M13 | 🟡 | Golden fixtures inconsistent 4× | engine/tests/golden/* + cli/evaluate.rs | Opposite expectations for same fixtures |
| M14 | 🟡 | GoldenProvider duplicated in src + tests | engine/src/golden_provider.rs | Maintenance risk |
| M36 | 🟡 | Gemini test depends on network | providers/gemini.rs:141-153 | Flaky CI |
| M37 | 🟡 | OpenAI test depends on network | providers/openai.rs:143-157 | Flaky CI |

**Worst-case user-visible impact:**
- `cite evaluate --json` returns human-readable output (agents can't parse it)
- Evaluate results may be inconsistent between CLI and test harnesses

**Priority:** Medium. H1 is a trivial fix. M13/M14 are test infrastructure debt.

### 3.6 Pipeline Fix Priority

| Priority | Pipeline | Reason |
|:--------:|----------|--------|
| **1st** | **Ingest** | 8 critical errors, runtime crash risk, data corruption for non-ASCII text, security bypass. Every other pipeline depends on ingested data being correct. |
| **2nd** | **Search/Context** | 4 critical errors including rate limiting compliance and onboarding. Directly user-facing with quality impact. |
| **3rd** | **Evaluate** | H1 trivial fix. M13/M14 needed for test confidence before other fixes. |
| **4th** | **Refresh** | Low frequency, contained blast radius. H7 fix deferred. |
| **5th** | **Recovery** | Cosmetic latency only. No data impact. |

---

## 4. Fix Strategy — Prioritized by Theme

### Theme 1: UTF-8 Bytes-vs-Characters Confusion 🔴

**What:** Replace all `str::len()` usage in offset/truncation/character-counting contexts with `str::chars().count()`. Create a common helper to prevent recurrence.

**Where:**

| File | Lines | Current Code | Fix |
|------|-------|-------------|-----|
| `graph/src/heading_parser.rs` | 17, 35 | `char_offset += line.len()` | `char_offset += line.chars().count()` |
| `ingest/src/validator.rs` | 97-98 | `trimmed[..255]` | `trimmed.chars().take(255).collect::<String>()` |
| `ingest/src/extractor.rs` | 37 | `total_chars = content.len()` | `total_chars = content.chars().count()` |
| `ingest/src/sentence_chunker.rs` | 42 | `current_text.len() < min_chars` | `current_text.chars().count() < min_chars` |
| `ingest/src/sentence_chunker.rs` | 47-48 | `current_text.len()` for offset_end | `current_text.chars().count()` |
| `ingest/src/sentence_chunker.rs` | 58-59 | `current_text.len()` for offset_end | `current_text.chars().count()` |
| `common/src/lib.rs` | new | (does not exist) | Add `pub fn char_len(s: &str) -> usize` + `pub fn char_truncate(s: &str, max: usize) -> String` |

**Why:** Affects ingest pipeline (all 3 sub-steps: validation, extraction, chunking) and search pipeline (hierarchy metadata is wrong → wrong enrichment). The root cause in `heading_parser` cascades through the entire hierarchy assignment chain.

**Dependencies:** None. This is the first theme to fix.

**Effort:** ~15 lines production code + ~30 lines of UTF-8 test cases across 3 crates.

**Risk:** Existing ASCII tests will still pass (ASCII `len() == chars().count()`). New UTF-8 tests needed to validate. The only risk is missing a `len()` call — add a CI grep check:
```bash
grep -rn '\.len()' crates/*/src/ | grep -viE 'test|vec|slice|byte|buf|\.as_bytes|blob|bin' | grep -iE 'offset|char|count|position|truncat'
```

### Theme 2: Referential Integrity 🔴

**What:** Enable `PRAGMA foreign_keys=ON` after database open.

**Where:**

| File | Lines | Current Code | Fix |
|------|-------|-------------|-----|
| `storage/src/lib.rs` | 34-40 | WAL mode configured, no FK pragma | Add `conn.pragma_update(None, "foreign_keys", true)` |

**Why:** Without FK enforcement, the database accepts orphan rows (chunks without documents, embeddings without chunks, traces without documents). All data integrity relies on application logic — a single code path bug creates permanent orphan data. Affects ALL pipelines.

**Dependencies:** None.

**Effort:** 1 line production code + integration tests verifying FK violations are rejected.

**Risk:** If existing data has orphan rows (from previous runs without FK), enabling FK will not retroactively fail — SQLite only enforces on new writes. But future foreign key violations will surface as errors. Test with a corrupt DB to confirm error messages are clear.

### Theme 3: Security — Production Mode Guard 🔴

**What:** Wire `check_ingest_allowed()` into the actual ingest path and rename `production_mode` to clarify its semantics.

**Where:**

| File | Lines | Current Code | Fix |
|------|-------|-------------|-----|
| `cli/src/commands/ingest.rs` | 60-67 | `production_mode: bool` only sanitizes display name | Call `runtime_guard::check_ingest_allowed(mode, provider)` before proceeding |
| `engine/src/ingest.rs` | 47, 74, 119 | `production_mode: bool` parameter | Rename to `sanitize_display_name: bool` or use enum |
| `engine/src/runtime_guard.rs` | 23-34 | Function defined + tested, never called | Already correct — just needs wiring |

**Why:** Compliance gap. PRD says runtime guard blocks ingest in Production mode. Currently it's dead code. Users in Production mode can ingest without restriction.

**Dependencies:** Rename `production_mode` first to avoid confusion, then add guard call.

**Effort:** ~15 lines across 2 crates (cli + engine) + integration test.

**Risk:** If someone depends on ingest working in Production mode (unlikely for a CLI tool), this will break them. Consider adding a `--force` flag to bypass with warning.

### Theme 4: Onboarding — Empty API Key Validation 🔴

**What:** Fail fast with clear error when API key is empty or missing, and defend in providers.

**Where:**

| File | Lines | Current Code | Fix |
|------|-------|-------------|-----|
| `cli/src/commands/mod.rs` | 94 | `.unwrap_or_default()` → `""` | Validate non-empty, return `CiteError::ConfigError { message: "No API key configured..." }` |
| `providers/src/gemini.rs` | 24 | Accepts empty string | Add `if key.is_empty() { return Err(...) }` |
| `providers/src/openai.rs` | 29 | Accepts empty string | Add `if key.is_empty() { return Err(...) }` |

**Why:** New users get HTTP 401 "Unauthorized" with no indication that the cause is a missing API key. Broken onboarding experience.

**Dependencies:** None.

**Effort:** ~15 lines across 3 crates.

**Risk:** None. Empty key was never valid — it just failed later with a worse error.

### Theme 5: Rate Limit Compliance — FR-109 Composite Key 🔴

**What:** Construct composite rate limit key from `(runtime_mode, corpus_id, provider_id, retrieval_scope)` instead of just `provider_id()`.

**Where:**

| File | Lines | Current Code | Fix |
|------|-------|-------------|-----|
| `engine/src/retrieve.rs` | 273-275 | `rate_limit_key = provider.provider_id()` | Build composite key: `format!("{}:{}:{}:{}", mode, corpus_id, provider_id, scope)` |

**Why:** FR-109 requires composite key for rate limiting. Current implementation means two corpora with the same provider share rate limit counters — incorrect for multi-corpus setups.

**Dependencies:** Need to determine how `corpus_id` is derived (likely from `data_dir` hash or snapshot ID).

**Effort:** ~10 lines in engine + possible helper in storage.

**Risk:** Changing the rate limit key format invalidates existing counters in the DB. Acceptable for a CLI tool (counters reset on restart anyway for the in-memory case, but storage persists them).

### Theme 6: Config-Disconnect Fix 🟠

**What:** Connect config fields to their consumers. Consolidate confusing chunk fields. Expand TOML support.

**Where:**

| File | Lines | Current Code | Fix |
|------|-------|-------------|-----|
| `providers/src/gemini.rs` | 31 | `Duration::from_secs(30)` hardcoded | Accept `timeout_secs: u64` parameter |
| `providers/src/openai.rs` | 34 | `Duration::from_secs(30)` hardcoded | Accept `timeout_secs: u64` parameter |
| `cli/src/commands/mod.rs` | ~70-90 | Provider creation ignores timeout | Pass `config.ingest.embedding_timeout_secs` |
| `config/src/lib.rs` | 94, 126 | `min_chunk_size_chars: 100` + `min_chunk_chars: 30` | Consolidate to single `min_chunk_chars` |
| `config/src/lib.rs` | 128 | `max_chunk_chars: 200` (< `chunk_size_chars: 1000`) | Set to `1500` or make > target |
| `config/src/lib.rs` | 356-411 | `TomlRoot` has only 3 sections | Add `runtime`, `rate_limit`, `ingest`, `paths` sections |

**Why:** Users configure timeouts, chunk sizes, and other parameters that have zero effect. The `max_chunk_chars: 200 < chunk_size_chars: 1000` contradiction means sentence chunking has a nonsensical default.

**Dependencies:** Consolidate config fields BEFORE fixing sentence_chunker (Theme 1) to avoid fixing against confused field names.

**Effort:** ~50 lines across config, providers, and cli.

**Risk:** Renaming/removing config fields is a breaking change for anyone using env vars (`CITE_MIN_CHUNK_SIZE_CHARS`, `CITE_MIN_CHUNK_CHARS`). Document the migration.

### Theme 7: Silenced Error Elimination 🟠

**What:** Replace `.ok()`, `let _ =`, and `.unwrap_or_default()` patterns with proper error handling at crate boundaries.

**Where:**

| File | Lines | Current Code | Fix |
|------|-------|-------------|-----|
| `storage/src/snapshots.rs` | 68-73 | `.ok()` → converts ALL errors to `None` | Use `.optional()` (only `QueryReturnedNoRows` → `None`) |
| `engine/src/ingest.rs` | 215-220 | `let _ = db.delete_chunks(...)` | Log error + propagate first error |
| `storage/src/embeddings.rs` | 31-37, 122, 155 | `continue` on `None` from `decode_vector_blob` | Log warning with row ID |

**Why:** Genuine DB errors are silently swallowed, causing orphan data accumulation, invisible corrupt BLOBs, and snapshot state inconsistency.

**Dependencies:** `.optional()` requires the `diesel` optional helper or equivalent from `rusqlite`. Verify availability.

**Effort:** ~15 lines across 3 crates.

**Risk:** Previously silenced errors will now surface as failures. This is correct behavior but may surprise users who never saw these errors before. Consider logging first (warn level), then failing in a follow-up.

### Theme 8: Integer Cast Safety 🟠

**What:** Replace `as u32` casts with `u32::try_from()` across storage.

**Where:**

| File | Lines | Current Code | Fix |
|------|-------|-------------|-----|
| `storage/src/util.rs` | 37, 42, 47 | `val as u32` | `u32::try_from(val).map_err(...)` |
| `storage/src/embeddings.rs` | 144, 148, 155 | `val as u32` | `u32::try_from(val).map_err(...)` |
| `storage/src/traces.rs` | 124, 128, 150 | `val as u32` | `u32::try_from(val).map_err(...)` |
| `storage/src/documents.rs` | 72-75 | `val as u32` | `u32::try_from(val).map_err(...)` |

**Why:** `as u32` silently truncates. Negative SQLite values or counts > 4.2B produce wrong data without error.

**Dependencies:** None.

**Effort:** ~20 lines across 4 files.

**Risk:** Low. `try_from` will surface edge cases as errors instead of corrupting data. For a CLI tool processing normal documents, these casts will never fail — the fix is purely defensive.

### Theme 9: CLI Provider Unwrap Consistency 🟠

**What:** Create `CommandContext::provider()` helper that returns `Result<&dyn EmbeddingProvider, CiteError>`.

**Where:**

| File | Lines | Current Code | Fix |
|------|-------|-------------|-----|
| `cli/src/commands/context.rs` | 50 | `.unwrap()` | `.provider()?` |
| `cli/src/commands/ingest.rs` | 66 | `.unwrap()` | `.provider()?` |
| `cli/src/commands/retrieve.rs` | 80 | `.unwrap()` | `.provider()?` |
| `cli/src/commands/search.rs` | 49-54 | `match` (correct) | Use same `.provider()?` |

**Why:** 3 of 4 commands that need a provider use `.unwrap()`, which panics on failure. Only `search.rs` handles it correctly. Pattern should be consistent and safe.

**Dependencies:** None.

**Effort:** ~15 lines (add helper method + update 4 call sites).

**Risk:** None. Existing behavior preserved; just adds graceful error handling.

### Theme 10: Graph Parsing Robustness 🟠

**What:** Fix duplicate heading boundary matching and improve code block detection.

**Where:**

| File | Lines | Current Code | Fix |
|------|-------|-------------|-----|
| `graph/src/hierarchy.rs` | 128-148 | `find()` matches first occurrence of title | Use cursor-based iteration tracking last-used position |
| `graph/src/heading_parser.rs` | 14-16 | `starts_with("```")` | Check `trim_start().starts_with("```")` and handle 4+ backtick fences |

**Why:** Duplicate section titles (e.g., two `## Overview`) cause the second section's chunks to be assigned to the first. Indented code blocks are missed, causing headings inside code blocks to be parsed as real headings.

**Dependencies:** Theme 1 (byte→char fix) should be applied first to ensure cursor positions are correct.

**Effort:** ~25 lines across 2 files + test cases.

**Risk:** Medium. The cursor-based approach changes boundary matching behavior. Test with documents that have duplicate headings, nested code blocks, and mixed content.

### Theme 11: Misc High-Tier Fixes 🟠

**What:** Collection of isolated high-tier fixes that don't form a theme.

**Where:**

| Error | File | Fix | Effort |
|-------|------|-----|--------|
| H1: `--json` duplicate | cli/evaluate.rs:14-18 | Remove `pub json: bool` from `EvaluateArgs` | 1 line |
| H2: Setup hardcodes models | cli/setup.rs:208-232 | Pass model from config | 5 lines |
| H5: `required_facets` false positives | engine/context.rs:62-80 | Word-boundary matching | 10 lines |
| H8: Duplicate heading boundaries | graph/hierarchy.rs:128-148 | Cursor-based (see Theme 10) | 15 lines |
| H9: Code block detection | graph/heading_parser.rs:14-16 | Trim + fence width (see Theme 10) | 5 lines |
| H16: tokio/tracing unused deps | providers/Cargo.toml:8-9 | Remove lines | 2 lines |
| H19: ScoredChunk field duplication | retrieval/lib.rs:40-64 | Wrap `ChunkEmbeddingRecord` or `From` impl | 30 lines |
| C11: CiteError no PartialEq | common/error.rs:7 | Add `PartialEq` to derive | 1 line |

**Why:** Each is a localized fix with clear impact. Grouped for efficiency.

**Dependencies:** H8/H9 depend on Theme 1 (byte offsets).

**Effort:** ~70 lines total across 7 crates.

**Risk:** Low per fix. H19 (ScoredChunk) has medium risk as it touches the retrieval→engine boundary.

---

## 5. SDD Scope Recommendation

### First Pass: Critical + High-Impact Fixes

| Theme | Errors Fixed | Tier | Crates Affected | Est. Lines |
|-------|:---:|:---:|:---:|:---:|
| 1. UTF-8 bytes/chars | C2, C3, C4, C5, H10, H11, H12 | T1+T2 | common, graph, ingest | ~45 |
| 2. FK enforcement | C1 | T1 | storage | ~5 |
| 3. Production mode guard | C6, C10 | T1 | cli, engine | ~15 |
| 4. Empty API key | C7 | T1 | cli, providers | ~15 |
| 5. Rate limit composite key | C8 | T1 | engine, storage | ~10 |
| 6. Config-disconnect | H13, H14, H15 | T2 | config, providers, cli | ~50 |
| 7. Silenced errors | H6, H17 | T2 | storage, engine | ~15 |
| 8. Integer cast safety | H18 | T2 | storage | ~20 |
| 9. Provider unwrap | H3 | T2 | cli | ~15 |
| 10. Graph robustness | H8, H9 | T2 | graph | ~25 |
| 11. Misc high-tier | H1, H2, H5, H16, H19, C11 | T1+T2 | 7 crates | ~70 |
| **TOTAL** | **~35 errors** | **T1+T2** | **7 crates** | **~285** |

**Errors resolved in first pass:** All 11 Critical + 12 High = **23 errors** directly fixed, with ~12 additional errors resolved as side effects (e.g., fixing UTF-8 in heading_parser also fixes the downstream hierarchy chain).

### Deferred to Second Pass

| Category | Error Count | Examples |
|----------|:---:|---------|
| DRY refactoring | 3 themes | Error display ×14, HTTP handler ×2, flag validation ×3 |
| Dead code cleanup | 6+ items | Engine struct, SemanticLink, into_compact_*, Graph struct, tokio/tracing |
| Test infrastructure | 14 errors | UTF-8 test standard, golden fixture consolidation, network-dependent tests |
| Newtype migration | ~50 files | DocumentId/ChunkId/TraceId usage across all crates |
| Type consistency | 3 themes | created_at DateTime vs String, offset u32 vs usize vs i64 |
| Minor/lows | 38 items | All L-tier errors |

**Estimated deferred:** ~80 errors, ~400+ lines.

### Recommended PR Strategy

**Chained PRs (3 PRs), not a single monolith.**

| PR | Theme(s) | Rationale | Est. Lines |
|----|----------|-----------|:---:|
| **PR-1** | Theme 1 (UTF-8) + Theme 2 (FK) | Data integrity foundation. These are the highest-impact, lowest-risk fixes. UTF-8 touches common + graph + ingest; FK touches only storage. Clean separation. | ~50 |
| **PR-2** | Themes 3, 4, 5 (Security + Onboarding + Compliance) | Security and compliance fixes. These affect cli + engine + providers. Coherent "production readiness" scope. | ~40 |
| **PR-3** | Themes 6-11 (Config + Errors + Casts + Graph + Misc) | Everything else. Config-disconnect is the anchor theme; the rest are smaller fixes that benefit from fresh review context. | ~195 |

**Why chained:** 
- PR-1 is pure data integrity (no behavioral change for ASCII text, just correctness for non-ASCII + FK).
- PR-2 is security/compliance (blocks production deployment if not fixed).
- PR-3 is config + defensive + robustness (largest but lowest risk per change).
- Each PR can be reviewed independently. PR-1 has zero dependencies on PR-2/3.
- Reviewer workload: PR-1 (~50 lines, 3 crates), PR-2 (~40 lines, 3 crates), PR-3 (~195 lines, 7 crates).

---

## 6. Risks and Non-Goals

### What This Fix Pass Does NOT Address

1. **Newtype migration** (`DocumentId`/`ChunkId`/`TraceId`). Defined but unused across all crates. Full migration touches ~50 files and is a separate effort with its own risk profile.

2. **`Database` as god object**. The 50-method handle is pragmatic for storage-only code. Splitting into aggregate-specific traits is a larger architectural refactor not justified by current error count.

3. **Snapshot rollback completeness**. H7 (orphan snapshots on activation failure) is included, but the broader question of snapshot lifecycle management (cleanup of superseded snapshots, TTL) is deferred.

4. **Concurrency model**. The lock-based single-ingest model is adequate for CLI. Moving to concurrent ingest would require architectural changes beyond bug fixes.

5. **Provider retry/backoff**. HTTP calls currently make a single attempt. Retry with exponential backoff is a feature addition, not a bug fix.

6. **Test coverage gaps**. While we add UTF-8 tests for the fixes we make, the broader test gaps (10 CLI commands untested, storage concurrency tests, config merge tests) are deferred.

7. **`Engine` empty struct, `SemanticLink` dead code, `Graph` unit struct**. Dead code cleanup is cosmetic. Deferred.

8. **Golden fixture consolidation**. 4 inconsistent copies exist. Consolidation is deferred to the test infrastructure pass.

### Known Limitations of the Fix Approach

1. **UTF-8 fix is surgical, not systemic.** We replace specific `len()` calls but don't prevent future misuse. The CI grep check is a best-effort lint, not a compile-time guarantee. A future contributor could add a new `len()` call in an offset context and reintroduce the bug.

2. **FK enforcement doesn't fix existing orphan data.** Enabling `PRAGMA foreign_keys=ON` only affects new writes. Existing orphan rows from previous runs will persist. A data migration or cleanup script may be needed for production databases.

3. **Config field consolidation is breaking.** Renaming `min_chunk_size_chars` → `min_chunk_chars` (or whichever survives) changes the env var name (`CITE_MIN_CHUNK_SIZE_CHARS` → `CITE_MIN_CHUNK_CHARS`). Users with existing `.env` files will need to update.

4. **`check_ingest_allowed` semantics need product decision.** The current implementation blocks ALL ingest in Production mode. Should there be a `--force` flag? Should it block only specific file types? This needs a product decision that the fix pass assumes "block all, no override."

5. **Rate limit key change invalidates existing counters.** Switching from `provider_id` to composite key means any rate limit counters in the DB from previous runs are effectively reset. Acceptable for CLI (counters should restart with new semantics), but document it.

### Technical Debt That Remains

| Debt | Scope | Impact | Why Deferred |
|------|-------|--------|-------------|
| DRY violations in CLI (error display ×14) | 14 files | Maintenance burden | High effort, zero runtime impact |
| HTTP+error handler duplication in providers | 2 files | ~30 lines duplicated | Low impact, isolated |
| `CommandContext::open` returns `Result<Self, i32>` | 1 file | Loses error type | Requires CLI-wide signature change |
| `created_at` String vs DateTime<Utc> | 5 files | Type inconsistency | Cross-crate migration |
| Newtypes defined but unused | ~50 files | Zero type-safety benefit | Large migration, separate effort |
| Flag validation duplicated 3× | 3 files | Same validation copy-pasted | Extract helper, low priority |
| Golden fixtures 4× with inconsistencies | 4 files | Test reliability | Test infrastructure effort |
| Rate limit counters no TTL | 1 file | ~57K records/day growth | Feature addition, not bug |
| Doc tests with `#[ignore]` | 12 files | Coverage gaps | Per-test effort, scattered |
| Config merge logic untested | 1 file | Regression risk on config changes | Test infrastructure effort |

---

## Appendix: Full Error Cross-Reference

### Errors Fixed in First Pass (mapped to themes)

| Error ID | Tier | Theme | Crate | Description |
|----------|:----:|:-----:|-------|-------------|
| C1 | 🔴 | 2 | storage | FK disabled |
| C2 | 🔴 | 1 | ingest | UTF-8 panic in truncation |
| C3 | 🔴 | 1 | graph | heading_parser byte/char offset |
| C4 | 🔴 | 1 | ingest | sentence_chunker byte/char offsets |
| C5 | 🔴 | 1 | ingest | extractor `total_chars` uses bytes |
| C6 | 🔴 | 3 | engine | `check_ingest_allowed` dead code |
| C7 | 🔴 | 4 | cli+providers | Empty API key → cryptic 401 |
| C8 | 🔴 | 5 | engine+storage | Rate limit key incomplete |
| C10 | 🔴 | 3 | engine | `production_mode` misleading name |
| C11 | 🔴 | 11 | common | CiteError no PartialEq |
| H1 | 🟠 | 11 | cli | `--json` duplicate in evaluate |
| H2 | 🟠 | 11 | cli | Setup hardcodes models |
| H3 | 🟠 | 9 | cli | Provider unwrap inconsistency |
| H5 | 🟠 | 11 | engine | `required_facets` false positives |
| H6 | 🟠 | 7 | engine | `cleanup_partial` ignores errors |
| H8 | 🟠 | 10 | graph | Duplicate heading boundaries |
| H9 | 🟠 | 10 | graph | Code block detection fragile |
| H10 | 🟠 | 1 | ingest | sentence_chunker `min_chars` bytes |
| H11 | 🟠 | 1 | ingest | sentence_chunker `offset_end` bytes |
| H12 | 🟠 | 1 | ingest | Off-by-one `min_chars` threshold |
| H13 | 🟠 | 6 | config | IngestConfig field confusion |
| H14 | 🟠 | 6 | config | TOML missing sections |
| H15 | 🟠 | 6 | providers | Timeout config ignored |
| H16 | 🟠 | 11 | providers | tokio/tracing unused deps |
| H17 | 🟠 | 7 | storage | `activate_snapshot` uses `.ok()` |
| H18 | 🟠 | 8 | storage | i64→u32 casts unchecked |
| H19 | 🟠 | 11 | retrieval | ScoredChunk field duplication |
| C9 | 🔴 | deferred | common | Newtypes dead code |

### Errors Deferred to Second Pass (38 T3 + 38 T4 = 76 errors + C9 + remaining H-tier)

All M-tier (37) and L-tier (38) errors are deferred, plus C9 (newtypes — requires ~50-file migration). These are catalogued in `analisis-errores-completo.md` sections 4 and 5.

---

*Documento consolidado: 2026-06-02*
*Fuentes cross-referenciadas: analisis-funcionamiento.md (arquitectura, 5 pipelines, 15 gaps), analisis-errores-completo.md (113 errores, 38 prioridades), analisis-final.md (51 errores deduplicados, recomendaciones)*
*Resultado: 35 errores en first pass, ~285 líneas, 3 PRs encadenados*
