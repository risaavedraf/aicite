# Prompt: SDD Third Pass — Error Remediation V3 (Deferred Items)

## Contexto

Estamos en el proyecto aiharness (E:\Proyectos\Intento_de_conseguir_pega\aiharness).

Completamos el **first pass** (error-remediation) y **second pass** (error-remediation-v2):
- First pass: 35 errores T1+T2 arreglados en 3 PRs
- Second pass: 78 errores T3+T4 + 11 casts arreglados en 6 PRs
- 297 tests pass, 0 clippy warnings, clean format
- Branch: `refactor/error-remediation-v2-waves-1-2`

**Objetivo:** Resolver los **items deferred** que no entraron en los passes anteriores.

## Items Deferred

### C9: Newtype Migration (~50 archivos) — PRIORIDAD ALTA

`DocumentId`, `ChunkId`, `TraceId` están definidos en `crates/common/src/types.rs` pero no se usan. La migración completa toca ~50 archivos.

**Scope:**
- `crates/common/src/types.rs` — definir newtypes con `Deref<Target=str>`, `Display`, `FromStr`, `AsRef<str>`, serde transparent
- `crates/storage/src/` — actualizar row decoding para retornar newtypes
- `crates/engine/src/` — actualizar callers
- `crates/cli/src/` — actualizar argument parsing
- `crates/retrieval/src/` — actualizar ScoredChunk fields
- `crates/graph/src/` — actualizar Topic/Concept
- Tests y fixtures en todos los crates

**Riesgo:** Alto — cambio transversal que toca public APIs

**Estrategia recomendada:** Crate-by-crate, PR por PR, con 400-line budget

### H7: Snapshot Activation Rollback

En `crates/engine/src/refresh.rs`, si `activate_snapshot` falla parcialmente, no hay rollback explícito. El código actual maneja esto por la transacción SQLite, pero se necesitan tests que prueben el comportamiento en fallo.

**Archivos:** `crates/engine/src/refresh.rs`, `crates/storage/src/snapshots.rs`

**Riesgo:** Medio — requiere entender el estado de la transacción

### H19: ScoredChunk Full Dedup

En el second pass se agregó `impl From<ChunkEmbeddingRecord> for ScoredChunk`. La dedup completa requeriría que `rank_by_similarity` use el From impl consistentemente. La API de `ScoredChunk` tiene más campos que `ChunkEmbeddingRecord` (topic/concept), así que el From impl llena con `None`.

**Evaluación:** El From impl ya está. La dedup completa es un cambio de API (posiblemente hacer ScoredChunk un wrapper sobre ChunkEmbeddingRecord). Deferir si no hay beneficio claro.

**Archivos:** `crates/retrieval/src/lib.rs`

**Riesgo:** Bajo si se limita a reutilizar el From impl; Alto si se reestructura la API

### Snapshot Pointer `updated_at`

La tabla `snapshot_pointer` no tiene columna `updated_at`. Agregarla requiere migración SQL.

**Archivos:** `crates/storage/src/snapshots.rs`, posible nueva migración

**Riesgo:** Bajo — migración additive

### `created_at` String vs DateTime<Utc>`

Varios tipos usan `created_at: String` en lugar de `DateTime<Utc>`:
- `graph::types::Topic`
- `graph::types::Concept`
- `storage::SemanticLinkRow`

La conversión cascada a través de storage (SQLite persistence) y CLI.

**Riesgo:** Medio — cambio de tipo en structs públicos

## SDD Preflight

Antes de arrancar, confirmar:

- **execution_mode:** ¿interactive o auto?
- **artifact_store:** openspec, engram, o both?
- **chained_pr_strategy:** ask_always, auto-forecast, single-pr-default, o force-chained?
- **review_budget_lines:** 400 (default)

## Prioridad sugerida

1. **C9 Newtype migration** — el item más grande y más valioso
2. **H7 Snapshot rollback** — correctness issue
3. **created_at type consistency** — mejora type safety
4. **H19 ScoredChunk** — evaluar si vale la pena
5. **Snapshot updated_at** — additive, bajo riesgo

## Archivos relevantes

- `openspec/changes/error-remediation-v2/tasks.md` — tasks del second pass (ver sección "Deferred / Out of Scope")
- `openspec/changes/error-remediation-v2/design.md` — diseño del second pass (ver "Phase N: Newtype Migration")
- `openspec/reports/error-tracking.md` — tracking consolidado
- `openspec/reports/revision-repo/analisis-errores-completo.md` — inventario completo de errores

## Requisitos SDD

1. Dividir en fases ejecutables (paralelo/async/encadenado)
2. Agrupar por TEMA, no por crate
3. Cada fase independiente o declarar dependencias
4. Priorizar: newtypes > snapshot > type consistency > ScoredChunk
5. Review budget < 400 líneas por PR
6. Strategy: confirmar con usuario (ask-always default)

## Expected output

- Artefactos SDD en `openspec/changes/error-remediation-v3/`
- Proposal → Spec → Design → Tasks → Apply → Verify
- Tracking actualizado en `openspec/reports/error-tracking.md`
- Version bump si amerita (v0.3.0 probable por scope de newtypes)
