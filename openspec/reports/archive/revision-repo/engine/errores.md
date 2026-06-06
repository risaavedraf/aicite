# Errores Engine — Pendientes de Fix

**Fecha:** 2026-06-02
**Revisor:** el Gentleman (review subagent)

---

## 🔴 CRITICAL

### C1. PARCIALMENTE RESUELTO: `check_ingest_allowed` se aplica en CLI, no dentro del engine

**Archivos:**
- `crates/engine/src/runtime_guard.rs` (definición y tests)
- `crates/cli/src/commands/ingest.rs` (invoca el guard en `execute()`)
- `crates/engine/src/ingest.rs` (`ingest`, `ingest_next`, `ingest_internal` no lo revalidan)

**Estado actual verificado en CR-2 (2026-06-04):** el CLI llama `engine::runtime_guard::check_ingest_allowed(&config.runtime.mode)` antes de `engine::ingest::ingest`, `ingest_next` o `enqueue_ingest`. Eso corrige la afirmación anterior de que el guard no se usaba en ningún path real.

**Riesgo restante:** el engine no hace defensa en profundidad. Un caller directo de `engine::ingest::ingest`, `ingest_next` o `ingest_internal` puede bypassar el control si no aplica `check_ingest_allowed` en su propio boundary.

**Recomendación:** decidir si el boundary oficial es el CLI (y documentarlo) o mover/agregar el guard en `engine::ingest::ingest` / `ingest_next` para proteger todos los callers.

---

### C2. `production_mode: bool` tiene semántica engañosa — NO es lo mismo que Production mode

**Archivos:**
- `crates/engine/src/ingest.rs:47, 74, 119` (parámetro `production_mode`)
- `crates/cli/src/commands/ingest.rs:86` (`let production_mode = config.runtime.mode == config::RuntimeMode::Production`)
- `crates/engine/src/ingest.rs:141` (uso: solo para `derive_display_name`)

**Problema:** El parámetro `production_mode: bool` en `ingest()` solo controla si `derive_display_name` usa el nombre del archivo raw o sanitizado. No tiene relación con el concepto de "Production runtime mode" que debería bloquear ingest. Un desarrollador que lea `production_mode: true` podría asumir que el ingest está restringido, cuando en realidad solo cambia el naming.

**Impacto:** Confusión de diseño. Contribuye a que C1 pase desapercibido — alguien podría pensar que pasar `production_mode` ya es el guard.

**Fix sugerido:** Renombrar a `sanitize_display_name: bool` para reflejar su real propósito, o reemplazar con un enum:

```rust
pub enum DisplayNameStrategy {
    FromPath,      // Usa nombre del archivo directamente
    Sanitized,     // Sanitiza el nombre (modo production)
}

pub fn ingest(
    db: &Database,
    provider: &dyn EmbeddingProvider,
    config: &IngestConfig,
    path: &Path,
    display_name_override: Option<&str>,
    name_strategy: DisplayNameStrategy,
) -> Result<IngestResult, CiteError> { ... }
```

---

## 🟠 HIGH

### H1. `required_facets_for_query` tiene falsos positivos con palabras comunes

**Archivo:** `crates/engine/src/context.rs:62-80`

**Problema:** La heurística de detección de queries multi-facét usa `contains()` para palabras conjunción. Esto genera falsos positivos:

- `" e "` matchea en inglés: `"A, B, C, D e F"` → detecta multi-faceta incorrectamente (portugués/italiano)
- `" en "` matchea en inglés: `"how to set up en environment"` → detecta multi-faceta (neerlandés/francés)
- `" y "` matchea en inglés: `"X and Y"` → OK, pero también matchearía `"y chromosome"` → falso positivo

**Impacto:** Queries con estas palabras comunes pueden requerir ≥2 citas cuando solo necesitan 1, resultando en `InsufficientContext` incorrecto.

**Código actual:**
```rust
fn required_facets_for_query(query: &str) -> u32 {
    let q = query.to_lowercase();
    if q.contains(" and ")
        || q.contains(" y ")
        || q.contains(" et ")
        || q.contains(" und ")
        || q.contains(" e ")   // ← problemático: "e" es palabra inglesa común
        || q.contains(" en ")  // ← problemático: "en" aparece en inglés
    {
        return 2;
    }
    // ...
}
```

**Fix sugerido:** Usar word boundary regex o un parser más robusto:

```rust
use regex::Regex;

fn required_facets_for_query(query: &str) -> u32 {
    let q = query.to_lowercase();
    
    // Solo "and" y "y" son confiables como conjunciones multi-facét en corpus inglés/español
    let conjunction_re = Regex::new(r"\b(and|y)\b").unwrap();
    if conjunction_re.is_match(&q) {
        return 2;
    }
    
    // Comma-separated clauses siguen igual
    let clause_count = q.split(',').filter(|c| c.trim().len() > 10).count();
    if clause_count >= 2 { 2 } else { 1 }
}
```

Alternativa sin regex: verificar que la palabra conjunción esté entre palabras alfanuméricas (no al inicio/final).

---

### H2. `cleanup_partial` ignora errores de DB — datos huérfanos posibles

**Archivo:** `crates/engine/src/ingest.rs:215-220`

**Problema:** `cleanup_partial` usa `let _ =` para ignorar errores de `delete_embeddings_for_document` y `delete_chunks_for_document`. Si la DB tiene un error (lock, I/O), los datos parciales quedan huérfanos.

**Código actual:**
```rust
fn cleanup_partial(db: &Database, document_id: &str) -> Result<(), CiteError> {
    let _ = db.delete_embeddings_for_document(document_id);
    let _ = db.delete_chunks_for_document(document_id);
    Ok(())
}
```

**Impacto:** Datos huérfanos en las tablas de embeddings y chunks. No crítico porque el documento se marca `Failed` y no se recupera en retrieval, pero acumula basura en la DB.

**Fix sugerido:** Propagar el primer error o al menos loggear:

```rust
fn cleanup_partial(db: &Database, document_id: &str) -> Result<(), CiteError> {
    let emb_result = db.delete_embeddings_for_document(document_id);
    let chunk_result = db.delete_chunks_for_document(document_id);
    
    // Log errors but don't fail the cleanup
    if let Err(ref e) = emb_result {
        tracing::warn!("Failed to delete embeddings for {}: {}", document_id, e);
    }
    if let Err(ref e) = chunk_result {
        tracing::warn!("Failed to delete chunks for {}: {}", document_id, e);
    }
    
    // Return first error if any
    emb_result.or(chunk_result)
}
```

---

### H3. Snapshot refresh no es fully atomic — attaches quedan huérfanos si activación falla

**Archivo:** `crates/engine/src/refresh.rs:34-54`

**Problema:** El flujo es: `begin_snapshot_build` → `attach_document_to_snapshot × N` → `activate_snapshot`. Si `activate_snapshot` falla después de los attaches, el snapshot queda en estado "building" con documentos adjuntos. No hay rollback de los attaches.

**Código actual:**
```rust
pub fn refresh_corpus(db: &Database) -> Result<RefreshResult, CiteError> {
    // ...
    db.begin_snapshot_build(&snapshot_id)?;
    // ...
    for doc in &ready_docs {
        db.attach_document_to_snapshot(&snapshot_id, &doc.document_id)?;
    }
    // Si activate_snapshot falla aquí, attaches quedan huérfanos
    let activate_result = db.activate_snapshot(&snapshot_id)?;
    // ...
}
```

**Impacto:** Snapshots "building" huérfanos en la DB. Bajo volumen (una refresh a la vez), el impacto es mínimo. Pero no hay mecanismo de cleanup para estos snapshots abandonados.

**Fix sugerido:** Agregar cleanup en caso de fallo de activación, o un garbage collector periódico:

```rust
match db.activate_snapshot(&snapshot_id) {
    Ok(result) => Ok(RefreshResult { ... }),
    Err(e) => {
        // Intentar marcar como failed para cleanup futuro
        let _ = db.mark_snapshot_failed(&snapshot_id, &ErrorInfo {
            code: "activation_failed".into(),
            message: e.to_string(),
        });
        Err(e)
    }
}
```

---

## 🟡 MEDIUM

### M1. Duplicación de Golden Fixtures en 4 ubicaciones con inconsistencias

**Archivos:**
- `crates/engine/tests/golden/fixtures.rs` — 10 fixtures con campos extendidos (`must_contain_chunk_texts`, `confidence_label_required`)
- `crates/engine/tests/golden/fixtures.json` — JSON de los mismos 10 fixtures
- `crates/cli/src/commands/evaluate.rs` — 10 fixtures inline con campos reducidos
- `crates/engine/src/evaluate.rs` — Framework con `GoldenFixture` struct

**Problema:** Hay 4 versiones de los fixtures. Las versiones CLI y test difieren en expectations:

| Fixture | CLI evaluate | Golden test |
|---------|-------------|-------------|
| `amb-001` | `ResultKind::Context`, min_citations=2 | `ResultKind::InsufficientContext`, min_citations=1 |
| `pi-001` | `ResultKind::InsufficientContext`, min_citations=1 | `ResultKind::Context`, min_citations=1 |

**Impacto:** El CLI evaluate y los tests de integración evalúan con expectativas opuestas para los mismos fixtures. Uno de los dos debe estar incorrecto. Además, los fixtures están hardcodeados en lugar de cargarse desde JSON.

**Fix sugerido:** Consolidar fixtures en una sola fuente (`fixtures.json`) y que tanto CLI como tests carguen desde ahí. Resolver las inconsistencias de expectations.

---

### M2. `GoldenProvider` duplicado en src y tests

**Archivos:**
- `crates/engine/src/golden_provider.rs` — Provider en el crate source
- `crates/engine/tests/golden/provider.rs` — Provider en tests (con `with_embeddings` extra)

**Problema:** Dos implementaciones casi idénticas de `GoldenProvider`. La de tests tiene un método extra `with_embeddings`. Ambas usan `EvalProvider::compute_vector` como fallback.

**Impacto:** Mantenimiento — cambios en una no se reflejan en la otra. Sin embargo, ninguna parece usarse realmente en los tests principales (golden_test.rs usa `EvalProvider` directamente).

**Fix sugerido:** Eliminar la duplicación. Si `GoldenProvider` del src no se usa externamente, considerar moverlo a tests o eliminarlo.

---

### M3. `Engine` struct vacía es dead code

**Archivo:** `crates/engine/src/lib.rs:9`

**Problema:** `pub struct Engine;` se declara pero nunca se instancia ni usa.

**Impacto:** Mínimo — es solo una línea. Pero indica un diseño incompleto o un placeholder olvidado.

**Fix sugerido:** Eliminar si no hay planes de usarlo, o agregar funcionalidad si era la intención.

---

### M4. `tracing` importado como dependencia pero no usado explícitamente

**Archivo:** `crates/engine/Cargo.toml` (dependencia `tracing`)

**Problema:** `tracing` está en las dependencias pero no hay llamadas a `tracing::info!`, `tracing::warn!`, etc. en el código de producción del engine.

**Impacto:** No hay instrumentación de tracing en el engine. En un pipeline de ingest con múltiples pasos, no hay forma de diagnosticar problemas sin debuggear paso a paso.

**Fix sugerido:** Agregar tracing en puntos clave del pipeline (inicio/fin de ingest, retrieval latency, refresh, recovery).

---

## 🟢 LOW

### L1. `#[allow(clippy::too_many_arguments)]` en `build_context` y `persist_trace`

**Archivos:**
- `crates/engine/src/context.rs:175` (`build_context` — 8 parámetros)
- `crates/engine/src/context.rs:119` (`persist_trace` — 10 parámetros)

**Problema:** Funciones con muchos parámetros, silenciando el warning de Clippy.

**Fix sugerido:** Usar structs de parámetros (ya existe `RetrievalRequest` como patrón). Crear `BuildContextRequest` y `PersistTraceRequest`.

---

### L2. `is_real_provider` no cubre todos los posibles providers mock

**Archivo:** `crates/engine/src/runtime_guard.rs:12`

**Problema:** La lista de providers no-reales es hardcodeada: `"eval" | "golden" | "mock" | "test"`. Si se agrega un nuevo provider mock con otro nombre, no se detectará.

**Fix sugerido:** Considerar un approach inverso — lista de providers reales conocidos, o un flag en la config del provider.

---

### L3. Rate limit por route no es persistente entre restarts

**Archivos:**
- `crates/engine/src/retrieve.rs:167-175` (`enforce_rate_limit`)

**Problema:** El rate limit se almacena en la DB SQLite. Si la DB se recrea (in-memory para tests, o reset en producción), los contadores se pierden. Para una herramienta CLI esto es aceptable, pero limita su uso como servicio.

---

## ✅ Completados

| # | Error | Estado | Notas |
|---|-------|--------|-------|
| — | — | — | Tabla vacía — ningún error ha sido fixeado aún |
