# Code Review — Items Pendientes y Sugerencias

**Fecha:** 2026-05-28
**Fuente:** Revisión completa de calidad de código con skill code-quality-review (Clean Code + Rust Idioms + GitHub Structure)

---

## Items Skipeados (No requieren acción)

Estos items de la revisión fueron investigados y se encontró que ya están manejados o no aplican:

### 1. Refactor de `ingest_internal` (119 líneas)
**Estado**: Ya hecho
**Detalle**: El codebase ya tiene `run_pipeline()` extraído como función separada manejando el pipeline core (extracción → chunking → storage → embedding). `ingest_internal` solo maneja adquisición/release de locks y error cleanup — que es la separación correcta.

### 2. Clones innecesarios en building de citations
**Estado**: Estructuralmente necesario
**Detalle**: `build_citations_from_ranked` toma `&[ScoredChunk]` (borrowed) porque `ranked` se necesita después para `persist_trace` y el count de `retrieved_chunks`. Los campos de Citation deben ser strings owned ya que `Citation` es un tipo separado retornado en `ContextResponse`. Cambiar a `.into_iter()` requeriría extraer todos los datos dependientes primero, agregando complejidad sin ganancia neta.

### 3. Optimización de batch insert en ingest
**Estado**: Ya hecho
**Detalle**: El codebase ya usa operaciones batch: `db.insert_chunks(document_id, &chunks)` y `db.insert_embeddings(&embeddings)`. Ambas APIs aceptan slices `&[...]`.

### 4. Agrupación de campos del struct Document
**Estado**: Demasiado invasivo
**Detalle**: Dividir `Document` en sub-structs (`DocumentMetadata`, `ProcessingInfo`, `Timestamps`) requeriría cambiar cada sitio de construcción en storage, ingest, y CLI crates. No vale la pena para una mejora a nivel sugerencia.

---

## Mejoras Diferidas (Trabajo Futuro)

### 1. Migración de Newtypes — `DocumentId`, `ChunkId`, `TraceId`

**Prioridad**: Media
**Esfuerzo**: Mediano-Grande (muchos archivos)
**Estado**: Newtypes definidos en `common/src/types.rs`, todavía no usados en call sites

Los wrappers newtype están listos:
```rust
pub struct DocumentId(pub String);
pub struct ChunkId(pub String);
pub struct TraceId(pub String);
```
Cada uno tiene `Display`, `From<String>`, `AsRef<str>`, `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `Serialize`, `Deserialize`.

**Path de migración:**
1. Empezar con `storage/src/documents.rs` — cambiar signatures de método de `&str` a `&DocumentId`
2. Actualizar call sites de `engine/src/ingest.rs` y `engine/src/context.rs`
3. Actualizar `cli/src/commands/*` para construir newtypes
4. Propagar a los crates restantes

**Riesgo**: Compilation chain breaks a través de muchos archivos. Debería hacerse incrementalmente, un crate a la vez.

### 2. Fix de `ResultKind` Use-After-Move Pre-existente

**Prioridad**: Baja (pre-existente, no causado por fixes de revisión)
**Esfuerzo**: Pequeño (1-2 líneas)
**Archivo**: `crates/engine/src/context.rs` líneas 318, 338

**Problema**: `ResultKind` se mueve y luego se usa de nuevo después.

**Opciones de fix:**
- Agregar `.clone()` en la línea 299 antes del move
- O borrow `&result_kind` en vez de consumirlo

### 3. Mejorar Cobertura de Doc Tests

**Prioridad**: Baja
**Esfuerzo**: Pequeño
**Estado actual:**
- 11 storage doc tests están en `ignore` (requieren instancia de Database)
- 1 retrieval doc test está en `ignore` (requiere `ChunkEmbeddingRecord`)

**Mejora:**
- Cambiar `ignore` a `no_run` si se agregan test helpers para construir `Database` en ejemplos de docs
- Agregar utilidades de test `#[doc(hidden)]` para setup de doc tests

---

## Fixes Completados (Aplicados 2026-05-28)

| Issue | Severidad | Archivos | Estado |
|-------|-----------|----------|--------|
| unwrap() en producción | Critical | trace.rs, search.rs | ✅ Fixeado |
| build_context 212 líneas | Warning | context.rs | ✅ Refactorizado |
| Violación DRY API key | Warning | mod.rs, health.rs | ✅ Fixeado |
| Doc comments retrieval | Suggestion | retrieval/src/lib.rs | ✅ Agregados |
| Doc comments graph | Suggestion | graph/src/lib.rs | ✅ Agregados |
| Doc comments storage | Suggestion | storage/src/documents.rs | ✅ Agregados |
| Doc comments common | Suggestion | common/src/types.rs | ✅ Agregados |
| Definiciones newtypes | Suggestion | common/src/types.rs | ✅ Definidos |
| Ejemplos de docs | Suggestion | Múltiples crates | ✅ Agregados |

**Validación:** 260 tests pasan, 0 fallos, 0 warnings del compilador.
