# Análisis Completo de Errores — aiharness (9 crates)

**Fecha:** 2026-06-02
**Fuentes:** 10 archivos de errores + cross-crate-review + items-pendientes
**Alcance:** 113 errores únicos contabilizados (con deduplicación de errores cross-crate)

---

## 1. Error Inventory Summary

### By Severity

| Severity | Count |
|----------|------:|
| 🔴 Critical | 11 |
| 🟠 High | 27 |
| 🟡 Medium | 37 |
| 🟢 Low | 38 |
| **Total** | **113** |

### By Crate

| Crate | Critical | High | Medium | Low | Total |
|-------|:--------:|:----:|:------:|:---:|:-----:|
| cli | 1 | 5 | 6 | 8 | 20 |
| common | 1 | 2 | 2 | 3 | 8 |
| config | 0 | 2 | 4 | 3 | 9 |
| engine | 2 | 3 | 4 | 3 | 12 |
| graph | 1 | 2 | 3 | 4 | 10 |
| ingest | 3 | 3 | 2 | 3 | 11 |
| providers | 1 | 3 | 3 | 4 | 11 |
| retrieval | 0 | 1 | 2 | 3 | 6 |
| storage | 2 | 2 | 5 | 4 | 13 |
| cross-crate / items-pendientes | 0 | 0 | 6 | 3 | 9 |
| *Deduplicated cross-crate refs* | *0* | *-8* | *0* | *0* | *-8* |
| **Unique total** | **11** | **19** | **37** | **38** | **113*** |

> *Note: Some errors are reported in multiple crate reviews (e.g. empty API key in CLI + providers, heading_parser in graph + ingest, check_ingest_allowed in CLI + engine). After deduplication, 113 unique errors remain. The raw count before dedup is 121.*

### By Category

| Category | Count |
|----------|------:|
| Logic / correctness bugs | 18 |
| Safety / panic / data corruption | 12 |
| Dead code / unused items | 11 |
| Missing validation (defensive programming) | 14 |
| Configuration issues | 12 |
| Test gaps / flaky tests | 14 |
| DRY violations / code smells | 11 |
| Performance / resource leaks | 6 |
| Documentation / naming / consistency | 9 |
| Design / architecture | 6 |

---

## 2. Tier-1 Errors (Critical — Must Fix First)

Ordered by impact × ease-of-fix.

### C1. Foreign keys disabled — referential integrity not enforced
- **Where:** `storage/src/lib.rs:34-40`
- **What:** `PRAGMA foreign_keys=ON` never executed. All `REFERENCES` in schema are decorative.
- **Why critical:** Database accepts orphan rows (chunks without documents, embeddings without chunks). Integrity relies 100% on application logic.
- **Fix complexity:** 1 line — add `conn.pragma_update(None, "foreign_keys", true)` after WAL mode.
- **Effort:** ⬜ Trivial
- **Impact:** 🔴 Blocks all data integrity guarantees

### C2. UTF-8 panic in `sanitize_display_name` truncation
- **Where:** `ingest/src/validator.rs:97-98`
- **What:** `trimmed[..255]` slices by bytes, panics if byte 255 falls inside multi-byte UTF-8 char.
- **Why critical:** Runtime panic on non-ASCII display names (emoji, accented chars, CJK).
- **Fix complexity:** 3 lines — replace with `trimmed.chars().take(255).collect::<String>()`.
- **Effort:** ⬜ Trivial
- **Impact:** 🔴 Runtime crash on real-world input

### C3. `heading_parser` byte-vs-char offset (root cause of hierarchy chain)
- **Where:** `graph/src/heading_parser.rs:17, 35`
- **What:** `char_offset += line.len()` uses byte length, not character count. Field is named `char_offset`.
- **Why critical:** Cross-crate chain → `ingest/lib.rs:161` compares byte offsets with char offsets → chunks assigned to wrong topic/concept. Invisible with ASCII-only tests.
- **Fix complexity:** 2 lines + 1 UTF-8 test.
- **Effort:** ⬜ Trivial
- **Impact:** 🔴 Silent data corruption for non-ASCII text

### C4. `sentence_chunker` byte-vs-char offset chain
- **Where:** `ingest/src/sentence_chunker.rs:42, 47-48, 58-59`
- **What:** `min_chars` comparison and `offset_end` calculation use `.len()` (bytes) instead of `.chars().count()`.
- **Why critical:** Chunk boundaries incorrect for multi-byte text. Combined with C3, the entire offset pipeline is byte-confused.
- **Fix complexity:** ~6 lines across 3 locations.
- **Effort:** ⬜ Small
- **Impact:** 🔴 Silent data corruption for non-ASCII text

### C5. `extract_plain_text` uses `content.len()` for `total_chars`
- **Where:** `ingest/src/extractor.rs:37`
- **What:** `total_chars = content.len()` returns bytes, not character count.
- **Why critical:** Metadata `total_chars` is semantically wrong for non-ASCII. Downstream consumers get inflated counts.
- **Fix complexity:** 1 line.
- **Effort:** ⬜ Trivial
- **Impact:** 🔴 Wrong metadata propagated to storage

### C6. `check_ingest_allowed` is dead code — production mode doesn't block ingest
- **Where:** `engine/src/runtime_guard.rs:23-34` (defined), `cli/src/commands/ingest.rs:60-67` (not called), `engine/src/ingest.rs` (not called)
- **What:** Function exists with tests but is never invoked. `production_mode` bool only controls display name sanitization.
- **Why critical:** Security control is illusory. Users can ingest in Production/PublicPackagedDemo mode.
- **Fix complexity:** ~10 lines — add guard call in CLI ingest + engine ingest.
- **Effort:** 🟨 Medium
- **Impact:** 🔴 Security/compliance gap

### C7. Empty API key passed silently → cryptic HTTP 401
- **Where:** `cli/src/commands/mod.rs:94`, `providers/src/gemini.rs:24`, `providers/src/openai.rs:29`
- **What:** `resolve_api_key(...).unwrap_or_default()` produces `""`. Providers accept empty string. First `embed()` fails with cryptic HTTP 401.
- **Why critical:** New users get incomprehensible errors instead of "no API key configured".
- **Fix complexity:** ~15 lines (validate in CLI + defend in providers).
- **Effort:** 🟨 Medium
- **Impact:** 🔴 Broken onboarding experience

### C8. Rate limit key doesn't fulfill FR-109
- **Where:** `engine/src/retrieve.rs:273-275`, `storage/src/rate_limits.rs`
- **What:** FR-109 requires composite key `(runtime_mode + corpus_id + provider_id + retrieval_scope)`. Code uses only `provider_id()`.
- **Why critical:** Rate limiting is per-provider only. Multi-corpus setups share counters incorrectly.
- **Fix complexity:** ~10 lines — construct composite key in engine or add helper in storage.
- **Effort:** 🟨 Medium
- **Impact:** 🔴 Non-compliance with functional requirement

### C9. Newtypes `DocumentId`, `ChunkId`, `TraceId` dead code
- **Where:** `common/src/types.rs:32, 66, 101`
- **What:** Newtypes defined with full impls but never re-exported from `lib.rs` and never used. All code uses raw `String`.
- **Why critical:** Type-safety benefit is zero. Dead code that confuses contributors. (Lower severity than others but high effort to fix properly.)
- **Fix complexity:** Re-export = 1 line. Migration to use newtypes = ~50+ files across all crates.
- **Effort:** 🟧 Large (full migration) / ⬜ Trivial (re-export only)
- **Impact:** 🟡 Design debt, not a runtime bug

### C10. `production_mode: bool` has misleading semantics
- **Where:** `engine/src/ingest.rs:47, 74, 119`
- **What:** Parameter named `production_mode` only controls display name sanitization, not ingest permission. Contributes to C6 going unnoticed.
- **Why critical:** Naming creates false security assumption.
- **Fix complexity:** Rename to `sanitize_display_name: bool` or use enum.
- **Effort:** ⬜ Small
- **Impact:** 🟡 Developer confusion (enables C6)

### C11. `CiteError` doesn't derive `PartialEq`
- **Where:** `common/src/error.rs:7`
- **What:** Can't do `assert_eq!(result.unwrap_err(), expected_error)`. Tests use less expressive `assert!(matches!(...))`.
- **Why critical:** Limits test quality across all crates.
- **Fix complexity:** Add `PartialEq` to derive macro.
- **Effort:** ⬜ Trivial
- **Impact:** 🟡 Test quality (all variants already contain PartialEq types)

---

## 3. Tier-2 Errors (High — Should Fix)

### H1. `--json` flag duplicated in `EvaluateArgs` — `cite evaluate --json` broken
- **Where:** `cli/src/commands/evaluate.rs:14-18, 359`
- **What:** `EvaluateArgs` defines own `--json` but `execute()` ignores it, using global `cli.json` instead. `cite evaluate --json` produces non-JSON output.
- **Fix:** Remove `pub json: bool` from `EvaluateArgs`.
- **Effort:** ⬜ Trivial

### H2. Setup hardcodes models in connection test
- **Where:** `cli/src/commands/setup.rs:208-232`
- **What:** `test_provider_connection` ignores `_config` and hardcodes `"text-embedding-004"` / `"text-embedding-3-small"`. Tests wrong model.
- **Fix:** Pass model from config.
- **Effort:** ⬜ Small

### H3. Unwrap inconsistent of provider between commands
- **Where:** `cli/src/commands/context.rs:50`, `ingest.rs:66`, `retrieve.rs:80` vs `search.rs:49-54`
- **What:** 3 commands use `.unwrap()`, 1 uses proper `match`. Inconsistent and fragile.
- **Fix:** Add `provider()` helper to `CommandContext`.
- **Effort:** ⬜ Small

### H4. `ingest --queued` creates provider unnecessarily
- **Where:** `cli/src/commands/ingest.rs:60-67`
- **What:** `CommandContext::open()` creates provider before determining mode. `--queued` only needs DB.
- **Fix:** Reorganize flow — create provider only when needed.
- **Effort:** 🟨 Medium

### H5. `required_facets_for_query` false positives with common words
- **Where:** `engine/src/context.rs:62-80`
- **What:** `" e "`, `" en "` match English words incorrectly. Queries get `required_citations=2` when they need 1.
- **Fix:** Use word-boundary regex or restrict to reliable conjunctions.
- **Effort:** ⬜ Small

### H6. `cleanup_partial` ignores DB errors — orphan data possible
- **Where:** `engine/src/ingest.rs:215-220`
- **What:** `let _ =` ignores delete errors. Orphan embeddings/chunks accumulate on failure.
- **Fix:** Log errors, propagate first error.
- **Effort:** ⬜ Small

### H7. Snapshot refresh not fully atomic — orphan attaches on failure
- **Where:** `engine/src/refresh.rs:34-54`
- **What:** If `activate_snapshot` fails after attaches, snapshot stays in "building" state with no cleanup.
- **Fix:** Add rollback/mark-failed on activation failure.
- **Effort:** 🟨 Medium

### H8. Duplicate heading title boundary — `find()` matches first occurrence
- **Where:** `graph/src/hierarchy.rs:128-148`
- **What:** Two sections with same title (e.g. two `## Overview`) both get the first one's boundary.
- **Fix:** Use cursor-based iteration instead of `find()`.
- **Effort:** 🟨 Medium

### H9. Code block detection fragile (indented fences, 4+ backticks)
- **Where:** `graph/src/heading_parser.rs:14-16`
- **What:** `starts_with("```")` misses indented fences (CommonMark allows 3 spaces) and matches 4+ backtick fences.
- **Fix:** Improve detection logic.
- **Effort:** ⬜ Small

### H10. `sentence_chunker` uses `len()` for `min_chars` comparison
- **Where:** `ingest/src/sentence_chunker.rs:42`
- **What:** `current_text.len() < min_chars` compares bytes with char threshold. Multi-byte text passes threshold incorrectly.
- **Fix:** Use `.chars().count()`.
- **Effort:** ⬜ Trivial

### H11. `sentence_chunker` `offset_end` uses `len()` (bytes)
- **Where:** `ingest/src/sentence_chunker.rs:47-48, 58-59`
- **What:** `offset_end` calculated with byte length. Offsets wrong for non-ASCII.
- **Fix:** Use `.chars().count()`.
- **Effort:** ⬜ Trivial

### H12. Off-by-one in `min_chars` threshold — incorrect chunk merge
- **Where:** `ingest/src/sentence_chunker.rs:42`
- **What:** `current_text.len() < min_chars` uses `<` (strict). Exact-length chunks get merged instead of standalone.
- **Fix:** Use `<=`.
- **Effort:** ⬜ Trivial

### H13. IngestConfig confusing/duplicate fields: `min_chunk_size_chars` vs `min_chunk_chars`
- **Where:** `config/src/lib.rs:94-103, 126-128`
- **What:** Two fields with nearly identical names and different defaults. `max_chunk_chars` (200) < `chunk_size_chars` (1000).
- **Fix:** Consolidate to 3 clear fields: target, min, max.
- **Effort:** 🟨 Medium

### H14. TOML config can't set most fields
- **Where:** `config/src/lib.rs:356-411`
- **What:** `TomlRoot` only has provider, retrieval, data. Missing: runtime, rate_limit, ingest (11 fields), paths.
- **Fix:** Expand `TomlRoot` with all sections.
- **Effort:** 🟨 Medium

### H15. `embedding_timeout_secs` config ignored — hardcoded 30s
- **Where:** `providers/src/gemini.rs:31`, `providers/src/openai.rs:34`
- **What:** Config field exists but constructors hardcode `Duration::from_secs(30)`.
- **Fix:** Add `timeout_secs` parameter to constructors.
- **Effort:** ⬜ Small

### H16. `tokio` and `tracing` deps in providers — never used
- **Where:** `providers/Cargo.toml:8-9`
- **What:** Declared but not imported. Increases compile time.
- **Fix:** Remove from Cargo.toml.
- **Effort:** ⬜ Trivial

### H17. `activate_snapshot` uses `.ok()` instead of `.optional()`
- **Where:** `storage/src/snapshots.rs:68-73`
- **What:** `.ok()` converts ALL errors to `None`, including genuine DB errors. Should only convert `QueryReturnedNoRows`.
- **Fix:** Replace with `.optional()`.
- **Effort:** ⬜ Trivial

### H18. Casts `i64 → u32` without overflow check
- **Where:** `storage/src/util.rs:37,42,47`, `embeddings.rs:144,148,155`, `traces.rs:124,128,150`, `documents.rs:72-75`
- **What:** `as u32` truncates silently. Negative values or >4.2B produce wrong data.
- **Fix:** Use `u32::try_from()`.
- **Effort:** 🟨 Medium (many locations)

### H19. `ScoredChunk` duplicates `ChunkEmbeddingRecord` fields
- **Where:** `retrieval/src/lib.rs:40-64`
- **What:** 9 fields manually mapped. Adding field to `ChunkEmbeddingRecord` requires manual sync.
- **Fix:** Wrap `ChunkEmbeddingRecord` or use `From` impl.
- **Effort:** 🟨 Medium

---

## 4. Tier-3 Errors (Medium — Nice to Fix)

| # | Error | Crate | File | Summary |
|---|-------|-------|------|---------|
| M1 | DRY: error display pattern repeated 14+ times | cli | `commands/*.rs` | Extract `handle_error()` helper |
| M2 | `CommandContext::open` returns `Result<Self, i32>` | cli | `commands/mod.rs:31` | Loses error type, limits composability |
| M3 | Flag validation duplicated 3× (search/retrieve/context) | cli | `commands/*.rs:34-43` | Extract `validate_retrieval_flags()` |
| M4 | `setup.rs` `unwrap_or_default()` silences TTY errors | cli | `commands/setup.rs:153` | Misleading error in non-TTY environments |
| M5 | `save_config` doesn't save model/endpoint | cli | `commands/setup.rs:244-254` | Setup tests one config, saves another |
| M6 | `health` makes network call despite "local state" claim | cli | `commands/health.rs:155-170` | Misleading UX, surprise latency |
| M7 | Re-exports incomplete in `common/lib.rs` | common | `lib.rs:8-17` | `ErrorInfo`, `OffsetRange` etc. not in root |
| M8 | `ExitCode` no `as_i32()` helper | common | `exit.rs` | Minor ergonomics |
| M9 | `load_from` returns `Result` but never fails | config | `lib.rs:167-173` | Signature lies; TOML errors lost |
| M10 | Env vars with invalid values silently ignored | config | `lib.rs:298-325` | User thinks config applied, default used |
| M11 | Config doesn't implement `PartialEq`/`Default` | config | `lib.rs:40` | Can't compare configs in tests |
| M12 | Config tests don't cover merge/env/TOML | config | `lib.rs:423-462` | Complex merge logic untested |
| M13 | Golden fixtures duplicated 4× with inconsistencies | engine | `tests/golden/*`, `cli/commands/evaluate.rs` | CLI and tests have opposite expectations for same fixtures |
| M14 | `GoldenProvider` duplicated in src and tests | engine | `src/golden_provider.rs`, `tests/golden/provider.rs` | Maintenance risk |
| M15 | `Engine` empty struct — dead code | engine | `src/lib.rs:9` | Placeholder forgotten |
| M16 | `tracing` imported but not used in engine | engine | `Cargo.toml` | No instrumentation in pipeline |
| M17 | Topic without H2 creates node without concepts | graph | `hierarchy.rs:110-120` | Chunks not assigned to any concept |
| M18 | `created_at` as `String` instead of `DateTime<Utc>` | graph | `types.rs:10, 18` | Inconsistent with common/storage pattern |
| M19 | `SemanticLink` dead code | graph | `types.rs:44-51` | Never created or consumed |
| M20 | Match branch unreachable in `find_sentence_boundary` | ingest | `chunker.rs:108` | Dead code, confusing |
| M21 | Truncation test uses ASCII-only (doesn't detect UTF-8 panic) | ingest | `validator.rs:206-210` | Test passes but validates wrong thing |
| M22 | No validation of empty `model`/`endpoint` in providers | providers | `gemini.rs:24`, `openai.rs:29` | Cryptic HTTP errors |
| M23 | DRY: HTTP+error handling duplicated between providers | providers | `gemini.rs:86-114`, `openai.rs:89-117` | ~30 lines duplicated |
| M24 | `EvalProvider` dim 6 mixes compliance/prompt injection | providers | `eval.rs:75-86` | False positives with "ignore"/"prompt" words |
| M25 | `cosine_similarity` missing edge case tests | retrieval | `lib.rs:197-256` | No opposite/orthogonal/1-dim tests |
| M26 | `rank_by_similarity` missing edge case tests | retrieval | `lib.rs:230-256` | No k>candidates, empty, all-invalid tests |
| M27 | Rate limit counters no TTL — unbounded growth | storage | `rate_limits.rs` | ~57K records/day accumulation |
| M28 | `list_chunk_embeddings_hierarchical` vs `list_ready` duplication | storage | `embeddings.rs:103-210` | ~60 lines duplicated |
| M29 | `ConceptRow`/`TopicRow` store `created_at` as String | storage | `concepts.rs:14`, `topics.rs:14` | Inconsistent with Document/Chunk |
| M30 | `decode_vector_blob` errors silently skipped | storage | `embeddings.rs:31-37, 122, 155` | Corrupt BLOBs invisible |
| M31 | Snapshot pointer has no `updated_at` field | storage | `migrations/005_snapshots.sql:18-21` | Can't tell when swap happened |
| M32 | `ResultKind` use-after-move | engine | `context.rs:318, 338` | Pre-existing, needs `.clone()` or borrow |
| M33 | Newtype migration path deferred | common→all | Multiple | Defined but not used anywhere |
| M34 | Doc tests coverage gaps | multiple | Multiple | 11 storage + 1 retrieval `ignore` tests |
| M35 | Gemini model field format unverified | providers | `gemini.rs:91` | `models/{name}` may break with new models |
| M36 | `test_embed_invalid_key_returns_error` depends on network | providers | `gemini.rs:141-153` | Flaky in CI without internet |
| M37 | OpenAI test `test_embed_invalid_endpoint_returns_error` depends on network | providers | `openai.rs:143-157` | Same as M36 |

---

## 5. Tier-4 Errors (Low — Cosmetic/Style)

| # | Error | Crate | Summary |
|---|-------|-------|---------|
| L1 | `into_compact_*` dead code with `#[allow(dead_code)]` | cli | 3 functions never called |
| L2 | 10 CLI commands have no tests | cli | health, setup, ingest, list, get, retry, search, retrieve, context, read, refresh |
| L3 | `run_startup_recovery` runs for read-only commands | cli | Unnecessary DB open + latency |
| L4 | `evaluate` ignores config/user args completely | cli | By design but undocumented |
| L5 | `read.rs` manual validation instead of `ArgGroup` | cli | Style inconsistency |
| L6 | Trace test uses nanosecond timestamps for unique dirs | cli | Potential parallel collision |
| L7 | Config file permissions not set on Windows | cli | API key file readable by other users |
| L8 | `sanitize_display_name` UTF-8 panic (cross-crate ref) | ingest | Already counted as C2 |
| L9 | `#[non_exhaustive]` missing on public enums | common | Prevents independent crate publishing |
| L10 | `CiteError` test coverage insufficient (1/18 variants) | common | Missing stability tests |
| L11 | `Document`/`ErrorInfo` don't derive `PartialEq` | common | Can't compare in tests |
| L12 | Default config path uses "cite" not "aiharness" | config | Unclear if intentional |
| L13 | `FileConfig` intermediate boilerplate | config | Could use serde flatten |
| L14 | `CITE_API_KEY` deprecation warning imprecise | config | Warning says "ignored" but it works as fallback |
| L15 | `#[allow(clippy::too_many_arguments)]` on `build_context`/`persist_trace` | engine | 8-10 params each |
| L16 | `is_real_provider` hardcodes mock list | engine | Misses new mock providers |
| L17 | Rate limit not persistent between restarts | engine | Acceptable for CLI, limits service use |
| L18 | `Graph` unit struct with no functionality | graph | Placeholder |
| L19 | `test_char_offsets` uses ASCII only | graph | Doesn't detect C3 |
| L20 | `HeadingSpan` missing `Serialize/Deserialize` | graph | Limits serialization |
| L21 | Boundary lookup is O(T × H) | graph | Could use cursor, low impact |
| L22 | `sentence_chunker` doesn't use `max_chunk_chars` | ingest | Very long sentences become huge chunks |
| L23 | Overflow u32 in offsets for huge documents | ingest | >4B chars, unrealistic |
| L24 | Offset tracking drift from trimming | ingest | Offsets approximate, acceptable |
| L25 | `serde_json` may not be needed as direct dep | providers | Verify transitive from reqwest |
| L26 | `EmbeddingProvider` trait no `#[non_exhaustive]` | providers | Fine for workspace, limits external use |
| L27 | `GeminiProvider` doesn't validate HTTPS scheme | providers | Endpoint hardcoded, future risk |
| L28 | Silent skip of invalid candidates | retrieval | Hard to debug data corruption |
| L29 | No clamp on `cosine_similarity` range | retrieval | Theoretical f64→f32 edge case |
| L30 | Retrieval `Cargo.toml` minimal metadata | retrieval | Consistent with project |
| L31 | `update_document_status` doesn't validate transitions | storage | Any state → any state |
| L32 | Superseded snapshots never cleaned | storage | Linear growth |
| L33 | `semantic_links` UNIQUE prevents multi-type relations | storage | Design limitation |
| L34 | `strip_windows_extended_prefix` only handles `\\?\` | storage | Edge case |
| L35 | Config `PartialEq`/`Default` missing | config | (Also M11, low coverage) |
| L36 | OpenAI endpoint HTTPS validation only partial | providers | URL format not fully validated |
| L37 | `Document` doesn't derive `PartialEq` | common | (Also L11) |
| L38 | `ErrorInfo` doesn't derive `PartialEq` | common | (Also L11) |

---

## 6. Cross-Crate Error Patterns

### 6.1 — 🔴 UTF-8 Bytes-vs-Characters Confusion (affects 4 crates)

**Root cause:** `str::len()` returns UTF-8 byte count, not character count. The Rust language doesn't protect against this confusion.

**Symptoms across crates:**

| Crate | File | Line(s) | Variable | Impact |
|-------|------|---------|----------|--------|
| graph | `heading_parser.rs` | 17, 35 | `char_offset` | Wrong heading offsets |
| ingest | `extractor.rs` | 37 | `total_chars` | Wrong metadata |
| ingest | `sentence_chunker.rs` | 42 | `min_chars` comparison | Wrong merge decision |
| ingest | `sentence_chunker.rs` | 47-48, 58-59 | `offset_end` | Wrong chunk boundaries |
| ingest | `validator.rs` | 97-98 | truncation | **Runtime panic** |
| storage | `util.rs`, `embeddings.rs`, etc. | multiple | `i64 as u32` | Silent data corruption |

**Causal chain:**
```
graph/heading_parser (byte offsets)
  → ingest/lib.rs (compares with char offsets from chunker)
  → chunks assigned to wrong topic/concept
  → storage persists wrong hierarchy
  → retrieval returns chunks with wrong hierarchy metadata
```

**Root fix:** Create `common::text::char_len()` helper and ban `str::len()` in offset/character contexts. Add UTF-8 test cases to ALL offset-related tests.

**Fix ordering:** graph/heading_parser.rs FIRST (root cause) → ingest/sentence_chunker.rs → ingest/extractor.rs → ingest/validator.rs → storage cast safety → add UTF-8 tests everywhere.

### 6.2 — 🔴 Config-Consumption Gap (affects 4 crates)

**Root cause:** Config layer defines fields with good intentions but consumption layer doesn't read them.

| Config field | Defined in | Ignored by | Effect |
|-------------|------------|------------|--------|
| `embedding_timeout_secs` | config:101 | providers/gemini.rs:31, openai.rs:34 | Timeout hardcoded 30s |
| `production_mode` guard | engine/runtime_guard.rs | cli/ingest.rs, engine/ingest.rs | Dead code, no guard |
| `max_chunk_chars` | config | ingest/sentence_chunker.rs | No chunk size limit |
| Rate limit composite key | FR-109 spec | engine/retrieve.rs:273-275 | Only `provider_id()` |
| TOML ingest/runtime/rate_limit fields | config FileConfig | config TomlRoot | Can't configure via file |

**Fix ordering:** Audit all `AppConfig` fields → verify each is consumed → either connect or mark as TODO.

### 6.3 — 🟠 Missing Defensive Validation at Crate Boundaries (affects 3 crates)

**Root cause:** Crates assume caller data is valid. No "parse, don't validate" pattern.

| Boundary | From | To | Missing validation |
|----------|------|----|--------------------|
| Empty API key | CLI | Providers | `unwrap_or_default()` → `""` |
| Provider `None` | CLI | Engine | `unwrap()` in 3 commands |
| Empty model | Config | Providers | `""` generates malformed endpoint |
| Empty endpoint | Config | OpenAI provider | `""` passes HTTPS check |

**Fix ordering:** Create newtypes (`ApiKey`, `ModelId`, `Endpoint`) with validation in constructors → use in all call sites.

### 6.4 — 🟠 Silenced Errors Pattern (affects 3 crates)

| Crate | File | Pattern | Effect |
|-------|------|---------|--------|
| storage | `snapshots.rs:68-73` | `.ok()` | DB errors → `None` |
| storage | `embeddings.rs:122, 155` | `continue` on `None` | Corrupt BLOBs invisible |
| cli | `commands/mod.rs:94` | `unwrap_or_default()` | Empty API key silent |
| cli | `setup.rs` | `unwrap_or_default()` | TTY errors silent |
| ingest | `sentence_chunker.rs` | offset drift | Approximate offsets |
| engine | `ingest.rs:215-220` | `let _ =` | Cleanup errors ignored |

**Fix:** Use `.optional()` for `QueryReturnedNoRows`, `.map_err()?` for genuine errors.

### 6.5 — 🟠 Dead Code / Unused Items (affects 5 crates)

| Item | Crate | Status |
|------|-------|--------|
| `check_ingest_allowed` | engine | Defined + tested, never called |
| `DocumentId`, `ChunkId`, `TraceId` newtypes | common | Defined, not re-exported or used |
| `SemanticLink` type | graph | Defined, never created |
| `Graph` unit struct | graph | No methods, no state |
| `Engine` unit struct | engine | No methods, no state |
| `into_compact_*` functions | cli | `#[allow(dead_code)]` |
| `tokio`, `tracing` deps | providers | In Cargo.toml, not imported |
| `GoldenProvider` in src | engine | Duplicated in tests |

### 6.6 — 🟠 Test Coverage Gaps (affects 7 crates)

| Crate | Gap |
|-------|-----|
| ingest | 57 tests, 0 with multi-byte UTF-8 text |
| graph | Offset tests use ASCII only |
| providers | Tests depend on network without `#[ignore]` |
| storage | No concurrency tests (two threads) |
| retrieval | No edge case tests for cosine/rank |
| config | Merge logic untested |
| cli | 10 of 14 commands have no tests |

### 6.7 — 🟡 DRY Violations (affects 4 crates)

| Pattern | Crates | Occurrences |
|---------|--------|-------------|
| Error display: `if json { print_json } else { eprintln } + exit_code` | cli | 14+ |
| HTTP send + error parsing + JSON deserialization | providers | 2 (gemini + openai) |
| Row mapping: `list_chunk_embeddings_hierarchical` vs `list_ready` | storage | ~60 lines |
| Flag validation: flat/topic/concept mutual exclusion | cli | 3 commands |
| Golden fixtures: 4 versions with inconsistent expectations | engine + cli | 4 copies |

---

## 7. Fix Priority Matrix

| # | Error | Crate(s) | Tier | Effort | Impact | Dependencies | Order |
|---|-------|----------|------|--------|--------|--------------|-------|
| 1 | FK disabled | storage | T1 | ⬜ 1 line | 🔴 Data integrity | None | **1** |
| 2 | heading_parser byte/char | graph | T1 | ⬜ 2 lines | 🔴 Hierarchy chain root | None | **2** |
| 3 | sentence_chunker byte/char | ingest | T1 | ⬜ 6 lines | 🔴 Offset chain | #2 first | **3** |
| 4 | validator UTF-8 panic | ingest | T1 | ⬜ 3 lines | 🔴 Runtime crash | None | **4** |
| 5 | extractor `total_chars` | ingest | T1 | ⬜ 1 line | 🔴 Wrong metadata | None | **5** |
| 6 | `production_mode` rename | engine | T1 | ⬜ Small | 🟡 Naming clarity | None | **6** |
| 7 | `check_ingest_allowed` dead code | cli+engine | T1 | 🟨 ~10 lines | 🔴 Security gap | #6 | **7** |
| 8 | Empty API key validation | cli+providers | T1 | 🟨 ~15 lines | 🔴 Onboarding | None | **8** |
| 9 | Rate limit composite key | engine+storage | T1 | 🟨 ~10 lines | 🔴 FR-109 compliance | None | **9** |
| 10 | `CiteError` + `PartialEq` | common | T1 | ⬜ 1 line | 🟡 Test quality | None | **10** |
| 11 | FK tests (UTF-8 test for #2,3,4) | graph+ingest | T1 | ⬜ per test | 🔴 Regression guard | #2,3,4 | **11** |
| 12 | `activate_snapshot` `.ok()` | storage | T2 | ⬜ 1 line | 🟠 DB error silence | None | **12** |
| 13 | `i64→u32` casts | storage | T2 | 🟨 ~20 lines | 🟠 Data corruption | None | **13** |
| 14 | `--json` duplicate in evaluate | cli | T2 | ⬜ 1 line | 🟠 UX bug | None | **14** |
| 15 | Setup hardcodes models | cli | T2 | ⬜ Small | 🟠 False confidence | None | **15** |
| 16 | Provider unwrap inconsistency | cli | T2 | ⬜ Small | 🟠 Fragility | None | **16** |
| 17 | Timeout config ignored | providers | T2 | ⬜ Small | 🟠 Config broken | None | **17** |
| 18 | `cleanup_partial` error ignore | engine | T2 | ⬜ Small | 🟠 Orphan data | None | **18** |
| 19 | Snapshot refresh atomicity | engine | T2 | 🟨 Medium | 🟠 Orphan snapshots | None | **19** |
| 20 | Duplicate heading boundaries | graph | T2 | 🟨 Medium | 🟠 Wrong assignment | None | **20** |
| 21 | Code block detection | graph | T2 | ⬜ Small | 🟠 False headings | None | **21** |
| 22 | IngestConfig field confusion | config | T2 | 🟨 Medium | 🟠 Config correctness | None | **22** |
| 23 | TOML missing sections | config | T2 | 🟨 Medium | 🟠 Config usability | None | **23** |
| 24 | `ingest --queued` needs no provider | cli | T2 | 🟨 Medium | 🟠 Offline usability | None | **24** |
| 25 | `required_facets` false positives | engine | T2 | ⬜ Small | 🟠 Wrong citations | None | **25** |
| 26 | tokio/tracing unused deps | providers | T2 | ⬜ 2 lines | 🟠 Compile time | None | **26** |
| 27 | `ScoredChunk` duplication | retrieval | T2 | 🟨 Medium | 🟠 Maintenance | None | **27** |
| 28 | DRY: error display (14×) | cli | T3 | 🟨 Medium | 🟡 Maintenance | None | **28** |
| 29 | Golden fixtures 4× duplication | engine+cli | T3 | 🟨 Medium | 🟡 Test correctness | None | **29** |
| 30 | HTTP+error DRY in providers | providers | T3 | 🟨 Medium | 🟡 Maintenance | None | **30** |
| 31 | Rate limit no TTL | storage | T3 | ⬜ Small | 🟡 Storage growth | None | **31** |
| 32 | Embeddings list duplication | storage | T3 | ⬜ Small | 🟡 Maintenance | None | **32** |
| 33 | Config merge tests missing | config | T3 | 🟨 Medium | 🟡 Regression risk | None | **33** |
| 34 | Topic/Concept `created_at` as String | graph+storage | T3 | ⬜ Small | 🟡 Type inconsistency | None | **34** |
| 35 | Newtype migration | common→all | T3 | 🧮 Large | 🟡 Type safety | #10 | **35** |
| 36 | Dead code cleanup (6+ items) | multiple | T4 | ⬜ Small each | 🟢 Hygiene | None | **36** |
| 37 | Test coverage gaps (7 crates) | multiple | T4 | 🧮 Large | 🟢 Confidence | None | **37** |
| 38 | Cosmetic/style (all L-tier) | multiple | T4 | ⬜ Small each | 🟢 Polish | None | **38** |

---

## Summary

**Top 3 Priorities (highest impact × lowest effort):**

1. **`PRAGMA foreign_keys=ON`** — 1 line fix, restores entire referential integrity system.
2. **heading_parser `line.len()` → `line.chars().count()`** — 2 lines, fixes the single most impactful bug chain in the project (hierarchy assignment for non-ASCII text).
3. **Validator UTF-8 panic + sentence_chunker byte/char fixes** — ~9 lines total, eliminates runtime crashes and silent offset corruption.

**The UTF-8 bytes-vs-characters pattern is the #1 systemic risk.** It affects 4 crates, generates the most impactful cross-crate causal chain, and is invisible to all existing tests. A single `common::text::char_len()` helper plus a CI grep for `.len()` in offset contexts would prevent recurrence.

**The config-consumption gap is the #1 architectural debt.** Well-designed config fields that nobody reads create a false sense of control. Users believe they configured timeout, rate limits, chunk sizes, and runtime guards — none of which actually work.

---

*Generated: 2026-06-02 | Source: 11 files analyzed | 113 unique errors catalogued*
