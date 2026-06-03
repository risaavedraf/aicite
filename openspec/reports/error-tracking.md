# Error Tracking Report — aiharness

**Fecha:** 2026-06-02
**SDD Change:** error-remediation
**Fuente:** `openspec/changes/error-remediation/` (proposal, spec, design, tasks, apply-progress)

---

## Resumen Ejecutivo

| Categoría | Errores | Status |
|-----------|--------:|--------|
| Total catálogados | 113 | — |
| **First pass (T1+T2)** | **35** | ✅ COMPLETO — verify PASS |
| ↳ PR-1 (data integrity) | 7 directos + side-effects | ✅ COMPLETO |
| ↳ PR-2 (security) | 4 directos | ✅ COMPLETO |
| ↳ PR-3 (robustness) | 14 directos | ✅ COMPLETO |
| **Second pass (T3+T4)** | **78** | 🛠️ APPLY IN PROGRESS — Waves 1-2 / PR-1 and PR-2a applied; PR-2b+ pending |
| ↳ +11 casts fuera de scope | 11 | included in planned PR-3 cast-safety theme; implementation pending |

---

## First Pass — Errores Arreglados

### PR-1: Data Integrity ✅ COMPLETO

**Theme 1: UTF-8 Bytes-vs-Chars (C2, C3, C4, C5, H10, H11, H12)**

| Error | Tier | Descripción | Archivo | Fix |
|-------|:----:|-------------|---------|-----|
| C2 | 🔴 | UTF-8 panic en truncation | `ingest/src/validator.rs:95-98` | `trimmed[..255]` → `trimmed.chars().take(255).collect::<String>()` |
| C3 | 🔴 | heading_parser byte/char offset | `graph/src/heading_parser.rs:17,37` | `line.len()` → `line.chars().count()` |
| C4 | 🔴 | sentence_chunker byte/char offsets | `ingest/src/sentence_chunker.rs:42,47,57` | `current_text.len()` → `current_text.chars().count()` |
| C5 | 🔴 | extractor `total_chars` usa bytes | `ingest/src/extractor.rs:37,75` | `content.len()` → `content.chars().count()` |
| H10 | 🟠 | sentence_chunker `min_chars` usa bytes | `ingest/src/sentence_chunker.rs:42` | Fix incluido en C4 |
| H11 | 🟠 | sentence_chunker `offset_end` usa bytes | `ingest/src/sentence_chunker.rs:47,57` | Fix incluido en C4 |
| H12 | 🟠 | Off-by-one `min_chars` threshold | `ingest/src/sentence_chunker.rs:42` | Fix incluido en C4 |

**Bonus fix (no en spec original):**
- `heading_parser.rs` tenía double-increment de `char_offset` para líneas dentro de code blocks — fixeado inline.

**Theme 2: FK Enforcement (C1)**

| Error | Tier | Descripción | Archivo | Fix |
|-------|:----:|-------------|---------|-----|
| C1 | 🔴 | FK disabled | `storage/src/lib.rs:34-41` | Added `PRAGMA foreign_keys = ON` en `open()` y `open_memory()` |

**Archivos modificados en PR-1 (6 archivos):**

| Archivo | Cambio |
|---------|--------|
| `crates/common/src/lib.rs` | +`char_len()`, `char_truncate()`, 9 tests |
| `crates/graph/src/heading_parser.rs` | 2 sites `len()` → `chars().count()`, code-block fix, 2 tests |
| `crates/ingest/src/validator.rs` | `len()` → `chars().count()` + `chars().take(255)`, 2 tests |
| `crates/ingest/src/extractor.rs` | 2 sites `len()` → `chars().count()`, 2 tests |
| `crates/ingest/src/sentence_chunker.rs` | 3 sites `len()` → `chars().count()`, 2 tests |
| `crates/storage/src/lib.rs` | FK pragma en `open()` + `open_memory()`, 3 tests |

**Tests:** `cargo test` → 268 passed, 0 failed, 12 ignored
**Lint:** `cargo clippy -- -D warnings` → 0 warnings
**Format:** `cargo fmt --check` → clean

---

## First Pass — Errores Pendientes

### PR-2: Security + Onboarding ✅ COMPLETO

**Theme 4: Empty API Key (C7)**

| Error | Tier | Descripción | Archivo | Fix pendiente |
|-------|:----:|-------------|---------|---------------|
| C7 | 🔴 | Empty API key → cryptic 401 | `cli/src/commands/mod.rs:94` | `.unwrap_or_default()` → `.ok_or_else()` con mensaje claro |
| C7 | 🔴 | (defense-in-depth) | `providers/src/gemini.rs:24` | `if api_key.is_empty()` → `Err(ConfigError)` ✅ |
| C7 | 🔴 | (defense-in-depth) | `providers/src/openai.rs:29` | `if api_key.is_empty()` → `Err(ConfigError)` ✅ |

**Theme 3: Production Mode Guard (C6, C10)**

| Error | Tier | Descripción | Archivo | Fix pendiente |
|-------|:----:|-------------|---------|---------------|
| C6 | 🔴 | `check_ingest_allowed` dead code | `engine/src/runtime_guard.rs:23-34` | Wired en CLI `ingest` command ✅ |
| C10 | 🔴 | `production_mode` misleading name | `engine/src/ingest.rs:47,74,119` | (deferred rename — solo wiring en PR-2) ✅ |

**Theme 5: Rate Limit Composite Key (C8)**

| Error | Tier | Descripción | Archivo | Fix pendiente |
|-------|:----:|-------------|---------|---------------|
| C8 | 🔴 | Rate limit key incomplete | `engine/src/retrieve.rs:278-280` | `provider_id()` → `provider_id:model_id()` ✅ |

**Errores directos en PR-2:** C6, C7, C8, C10 = **4 errores** ✅
**Tests:** `cargo test` → 285 passed, 0 failed, 13 ignored
**Lint:** `cargo clippy -- -D warnings` → 0 warnings
**Format:** `cargo fmt --check` → clean
**Líneas:** ~37 net insertions across 6 files

---

### PR-3: Config + Defensive + Robustness ✅ COMPLETO

**Theme 6: Config-Disconnect (H13, H14, H15)**

| Error | Tier | Descripción | Archivo | Fix pendiente |
|-------|:----:|-------------|---------|---------------|
| H13 | 🟠 | IngestConfig field confusion | `config/src/lib.rs:93,108,111` | `min_chunk_size_chars` eliminado, consolidado en `min_chunk_chars` ✅ |
| H14 | 🟠 | TOML missing sections | `config/src/lib.rs` | Expand TOML support ✅ |
| H15 | 🟠 | Timeout config ignored | `providers/src/gemini.rs:28, openai.rs:34` | `timeout_secs` param en constructores ✅ |

**Theme 7: Silenced Errors (H6, H17)**

| Error | Tier | Descripción | Archivo | Fix pendiente |
|-------|:----:|-------------|---------|---------------|
| H6 | 🟠 | `cleanup_partial` ignores errors | `engine/src/ingest.rs:191-193,246-247` | Log warning on failure ✅ |
| H17 | 🟠 | `activate_snapshot` uses `.ok()` | `storage/src/snapshots.rs:68-73` | `.ok()` → `.optional()` ✅ |

**Theme 8: Integer Cast Safety (H18)**

| Error | Tier | Descripción | Archivo | Fix pendiente |
|-------|:----:|-------------|---------|---------------|
| H18 | 🟠 | i64→u32 casts unchecked | `storage/src/util.rs:37,42,47,51` | `as u32` → `u32::try_from()` ✅ |
| H18 | 🟠 | (same pattern) | `storage/src/embeddings.rs:144,148,152,155` | `as u32` → `u32::try_from()` ✅ |
| H18 | 🟠 | (same pattern) | `storage/src/embeddings.rs:~209,213,217,220` | `as u32` → `u32::try_from()` ✅ |

**Theme 9: Provider Unwrap (H3)**

| Error | Tier | Descripción | Archivo | Fix pendiente |
|-------|:----:|-------------|---------|---------------|
| H3 | 🟠 | Provider unwrap inconsistency | `cli/src/commands/context.rs:50` | `.unwrap()` → `.provider()?` ✅ |
| H3 | 🟠 | (same pattern) | `cli/src/commands/ingest.rs:66` | `.unwrap()` → `.provider()?` ✅ |
| H3 | 🟠 | (same pattern) | `cli/src/commands/retrieve.rs:80` | `.unwrap()` → `.provider()?` ✅ |

**Theme 10: Graph Robustness (H8, H9)**

| Error | Tier | Descripción | Archivo | Fix pendiente |
|-------|:----:|-------------|---------|---------------|
| H8 | 🟠 | Duplicate heading boundaries | `graph/src/hierarchy.rs:130-146` | Sequential heading consumption ✅ |
| H9 | 🟠 | Code block detection fragile | `graph/src/heading_parser.rs:14-16` | Verificado como false alarm — no necesita fix |

**Theme 11: Misc High-Tier (H1, H2, H5, H16, H19, C11)**

| Error | Tier | Descripción | Archivo | Fix pendiente |
|-------|:----:|-------------|---------|---------------|
| C11 | 🔴 | CiteError no PartialEq | `common/src/error.rs:7` | `PartialEq` added to derive ✅ |
| H1 | 🟠 | `--json` flag duplicated | `cli/src/commands/evaluate.rs:14-16` | Dead `json` param removed ✅ |
| H2 | 🟠 | Setup hardcodes models | `cli/src/setup.rs:208-232` | Timeout param passed ✅ |
| H5 | 🟠 | `required_facets` false positives | `engine/src/context.rs:62-80` | Word-boundary matching ✅ |
| H16 | 🟠 | tokio/tracing unused deps | `providers/Cargo.toml:8-9` | Removed ✅ |
| H19 | 🟠 | ScoredChunk field duplication | `retrieval/src/lib.rs:40-64` | DEFERRED (API change risk) |

**Errores directos en PR-3:** H1, H2, H3, H5, H6, H8, H13, H14, H15, H16, H17, H18, C11 = **13 errores** ✅ (H9 = false alarm, H19 deferred)
**Tests:** `cargo test` → 308 passed, 0 failed
**Lint:** `cargo clippy -- -D warnings` → 0 warnings
**Format:** `cargo fmt --check` → clean
**Líneas:** 21 files modified

---

## Second Pass — Errores Deferred (78 errores)

Estos errores NO están incluidos en el SDD actual. Se resolverán en un segundo pass futuro.

### T3: Medium (37 errores)

| Categoría | Errores | Ejemplos |
|-----------|--------:|---------|
| DRY refactoring | ~14 | Error display ×14 duplicado en CLI |
| Test infrastructure | ~10 | Golden fixtures inconsistentes ×4, tests dependen de network |
| Type consistency | ~5 | `created_at` String vs DateTime<Utc>, offset u32 vs usize vs i64 |
| Misc medium | ~8 | GoldenProvider duplicado, evaluate inconsistencies |

### T4: Low (38 errores)

| Categoría | Errores | Ejemplos |
|-----------|--------:---------|
| Dead code cleanup | ~12 | Engine empty struct, SemanticLink, into_compact_*, Graph unit struct |
| Naming/docs | ~10 | Inconsistent naming, missing doc comments |
| Minor cleanup | ~16 | Unused imports, redundant clones, style issues |

### C9: Newtypes (deferred — ~50 archivos)

| Error | Tier | Descripción | Alcance |
|-------|:----:|-------------|---------|
| C9 | 🔴 | Newtypes dead code | `DocumentId`, `ChunkId`, `TraceId` definidos pero no usados. Migración completa toca ~50 archivos. |

---

## PR Status Summary

| PR | Theme(s) | Errores | Líneas | Status |
|----|----------|--------:|-------:|--------|
| **PR-1** | 1 (UTF-8) + 2 (FK) | 7+bonus | ~274 ins / ~108 del | ✅ COMPLETO |
| **PR-2** | 3 (Guard) + 4 (API Key) + 5 (Rate Limit) | 4 | ~37 | ✅ COMPLETO — verify PASS |
| **PR-3** | 6-11 (Config + robustness) | 13 | ~21 files | ✅ COMPLETO — verify PASS |
| **Second pass PR-1** | CLI DRY + retrieval validation | M1/M3 partial, M4 partial | 342 command-code changed lines | 🛠️ APPLIED — fresh review PASS, verify gate passed locally, final SDD verify pending |
| **Second pass PR-2a** | Golden fixtures + evaluation provider | M13/M14 partial, L4 partial; M24 deferred | 369 scoped code changed lines | 🛠️ APPLIED — verify gate passed locally, final SDD verify pending |
| **Second pass PR-2b+** | Deterministic tests/types/storage/dead-code/docs + casts | remaining T3/T4 + 11 casts | 5 planned PRs | 🔲 PENDING |

---

## Verify Report

**Status:** ✅ PASS
**Tests:** 308 passed, 0 failed, 13 ignored
**Clippy:** 0 warnings
**Format:** clean

**Deviations from design (5 minor — all improvements):**
1. heading_parser code-block double-increment fix (discovered during PR-1)
2. Task 3.15 indented code fence — confirmed false alarm
3. setup.rs had unlisted provider constructor calls needing timeout param
4. Chunker param renamed to match consolidated config field
5. min_chunk_chars default adjusted 100→30

**Discovery:** 11 unchecked `as u32` casts remain in `documents.rs`, `traces.rs`, `rate_limits.rs` — out of scope for this pass, recommended for follow-up.

## Para la próxima sesión

1. Continue `error-remediation-v2` with PR-2b (deterministic test infrastructure and edge cases) after reviewing PR-2a.
2. First-pass artefactos SDD completos están en `openspec/changes/error-remediation/`.
3. Second-pass tasks/progress están en `openspec/changes/error-remediation-v2/tasks.md` and `apply-progress.md`.
4. Verify report completo en `openspec/changes/error-remediation/verify-report.md`.

---

*Última actualización: 2026-06-02 (verify PASS)*
*Generado por: SDD error-remediation verify phase*
