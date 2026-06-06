# Revisión: `crates/engine` — Lógica de Negocio

**Fecha:** 2026-06-02
**Revisor:** el Gentleman (review subagent)
**Archivos analizados:** 9 fuente + 4 test

---

## Resumen del Crate

**Propósito:** El crate `engine` es la capa de orquestación central del proyecto aiharness. Coordina los pipelines de ingest (extracción → chunking → almacenamiento → embedding), retrieval (búsqueda vectorial + ranking), context pack assembly (construcción de paquetes de contexto con citas), refresh de corpus (snapshot swap atómico), recovery de procesamiento interrumpido, evaluación golden dataset, y guards de modo de runtime.

**Estructura:**

| Módulo | Líneas | Responsabilidad |
|--------|--------|-----------------|
| `lib.rs` | ~10 | Root del crate, re-exports de módulos + struct `Engine` (vacía) |
| `context.rs` | ~940 | `build_context`, `read_context`, `get_trace` — ensamblaje de context packs |
| `ingest.rs` | ~510 | Pipeline completo de ingest con lock, backlog queue, retry |
| `retrieve.rs` | ~930 | Búsqueda vectorial `search`/`retrieve`, ranking, jerarquía |
| `refresh.rs` | ~175 | Snapshot swap atómico del corpus |
| `recovery.rs` | ~95 | Recovery de documentos en estado `Processing` |
| `runtime_guard.rs` | ~28 | Guards de modo de runtime |
| `evaluate.rs` | ~220 | Evaluación contra golden dataset |
| `golden_provider.rs` | ~35 | Provider mock determinista para evaluación |

**Dependencias externas:**
- `common` — tipos compartidos (`CiteError`, `ContextResponse`, `Citation`, etc.)
- `storage` — `Database` (SQLite via rusqlite)
- `ingest` — `extractor`, `chunker`, `validator`
- `providers` — `EmbeddingProvider` trait + `EvalProvider`
- `config` — `RetrievalConfig`, `IngestConfig`, `RateLimitConfig`, `RuntimeMode`
- `retrieval` — `rank_by_similarity`, `ScoredChunk`
- `uuid`, `tracing`, `chrono`

---

## Módulos y Archivos

### `lib.rs` — Root del Crate

Declara los 8 módulos públicos y un struct `Engine` vacío que no se usa en ningún lugar. El struct es esencialmente dead code — toda la funcionalidad se accede a través de funciones libres en cada módulo.

### `context.rs` — Construcción de Context Pack

El módulo más sustancial del crate. Implementa tres APIs públicas:

**`build_context`** — Función principal que construye un `ContextResponse` completo:
1. Valida que el corpus tenga al menos un documento Ready (`validate_corpus_ready`)
2. Ejecuta el pipeline de retrieval (`ranked_candidates`) para obtener chunks rankeados
3. Calcula el `ResultKind` basado en scores y thresholds (`compute_result_kind`)
4. Construye citas (`Citation`) desde los hits rankeados
5. Persiste trace header y citas en la DB para auditoría
6. Retorna el `ContextResponse` con metadata completa

**`read_context`** — Resuelve un selector de lectura (por citation+trace o por chunk directo).

**`get_trace`** — Obtiene el envelope de trace para una request de context/retrieval completada.

**Funciones auxiliares:**
- `compute_result_kind` — Clasifica resultados en `Context`, `NoResults`, o `InsufficientContext` basado en `evidence_floor`, `confidence_threshold`, y cobertura de facetas.
- `required_facets_for_query` — Heurística para detectar queries multi-facét usando conjunciones ("and", "y", "et", etc.) y cláusulas separadas por comas.
- `build_citations_from_ranked` — Convierte `ScoredChunk` a `Citation` con breadcrumb, confidence labels, y offsets.
- `persist_trace` — Guarda el trace completo (header + citas) en la base de datos.

**Tests:** 16 tests cubren ResultKind, corpus parcial, rate limiting, trace persistence, read modes, jerarquía, y breadcrumbs.

### `ingest.rs` — Pipeline de Ingest

Implementa el pipeline completo de ingest con manejo de concurrencia:

**APIs públicas:**
- `ingest` — Ingest directo de un archivo (valida → crea doc → procesa → marca ready/failed)
- `enqueue_ingest` — Agrega un archivo a la cola de backlog sin procesarlo
- `ingest_next` — Procesa el próximo item del backlog
- `retry_document` — Reintenta un documento fallido (limpia datos parciales, reset a Pending)

**Flujo interno (`ingest_internal`):**
1. Valida archivo (tipo, tamaño, path policy)
2. Intenta adquirir lock distribuido (`ingest_pipeline`)
3. Si lock falla y `queue_on_lock_conflict=true`, encola en backlog
4. Crea documento con status `Pending`
5. Marca como `Processing`
6. Ejecuta pipeline (`run_pipeline`): extracción → chunking → almacenamiento → embedding
7. En éxito: marca `Ready` + actualiza chunk_count
8. En fallo: ejecuta `cleanup_partial` + marca `Failed` con error info
9. Siempre libera el lock al final

**Manejo de errores parciales:**
- `cleanup_partial` limpia embeddings y chunks del documento fallido
- El backlog maneja `OperationInProgress` reencolando el item
- Otros errores marcan el item del backlog como `failed`

**Lock pattern:** Usa un lock named `ingest_pipeline` con UUID como owner. Garantiza que solo un ingest corra a la vez. Si el lock está ocupado, la operación se encola.

### `retrieve.rs` — Búsqueda Vectorial y Ranking

Implementa la búsqueda semántica con soporte jerárquico:

**APIs públicas:**
- `search` — Retorna `Vec<Hit>` con preview truncado (~160 chars)
- `retrieve` — Retorna `Vec<Hit>` con texto completo

**Pipeline compartido (`ranked_candidates`):**
1. Valida query (no vacía, no solo puntuación, max 4000 chars)
2. Enforza rate limit por provider+route
3. Embed la query via provider
4. Fetch candidates (jerárquico o flat según config + datos disponibles)
5. Rank por similitud coseno
6. Enriquece con metadata jerárquica (topic, concept, breadcrumb)

**Soporte jerárquico (Phase 11):**
- `fetch_candidates` decide automáticamente entre path jerárquico y flat
- `enrich_with_hierarchy` agrega topic_name, concept_name a cada hit
- `build_breadcrumb` construye paths como "doc > topic > concept"
- Fallback automático: si `use_hierarchy=true` pero no hay datos jerárquicos, usa flat

**Filtros:** Soporta `topic_filter` y `concept_filter` para restringir búsqueda a subconjuntos del corpus.

**Tipo unificado `Hit`:** Unifica `SearchHit` y `RetrieveHit` (aliases backward-compat). Incluye campos jerárquicos opcionales.

### `refresh.rs` — Atomic Snapshot Swap

Implementa refresh del corpus creando un nuevo snapshot:

**Flujo:**
1. Crea un snapshot en estado "building" con ID único
2. Recolecta todos los documentos con status `Ready`
3. Adjunta cada documento al snapshot
4. Activa el snapshot atómicamente (supersedes al anterior)

**Propiedades:**
- Si no hay documentos Ready, marca el snapshot como failed y retorna error
- Si la activación falla, el snapshot anterior permanece intacto
- El snapshot anterior se preserva como referencia (`previous_snapshot_id`)

### `recovery.rs` — Recovery de Procesamiento Interrumpido

Maneja documentos que quedaron en estado `Processing` (por crash o interrupción):

**Política:** Todos los documentos en `Processing` se mueven a `Failed` con un código/mensaje de error estable.

**Safeguard:** Solo ejecuta recovery si el lock `ingest_pipeline` NO está activo (evita interferir con un ingest real en curso).

**Propiedades:**
- Idempotente: ejecutarlo múltiples veces es seguro
- El código de error `interrupted_processing_recovered` permite distinguir recovery de fallos reales
- Los documentos recuperados pueden reintentarse con `retry_document`

### `runtime_guard.rs` — Guards de Runtime Mode

Dos funciones de guard:

**`check_ingest_allowed(mode)`:** Bloquea ingest para modos `PublicPackagedDemo` y `Production`. Solo permite `LocalPrivateDemo`.

**`is_real_provider(provider_id)`:** Determina si un provider envía datos a servicios externos. Retorna `false` para "eval", "golden", "mock", "test".

### `evaluate.rs` — Evaluación Golden Dataset

Framework de evaluación que ejecuta fixtures contra el pipeline de context:

**`run_evaluation`:** Ejecuta cada fixture, compara `result_kind` y `min_citations` contra expectativas, calcula hit_rate, y genera un `EvalReport`.

**`evaluate_fixture`:** Ejecuta `build_context` para cada fixture y verifica expectations. En error, marca como fail con razón.

### `golden_provider.rs` — Provider Mock

Provider determinista que mapea texto a vectores de 8 dimensiones basados en temas semánticos. Usa `EvalProvider::compute_vector` como fallback con cache por texto normalizado.

---

## Flujo de Ingest

```
Archivo → validate_file() → [lock check]
                                  ↓ (lock OK)
                            insert_document(Pending)
                                  ↓
                         update_document_status(Processing)
                                  ↓
                         extractor::extract_text(path, file_type)
                                  ↓
                         chunker::chunk_text(pages, config)
                                  ↓
                         insert_chunks(document_id, chunks)
                                  ↓
                         provider.embed(chunk.text) × N
                                  ↓
                         insert_embeddings(vectors)
                                  ↓
                    update_document_status(Ready) + update_chunk_count
```

**En caso de fallo en cualquier paso post-Processing:**
```
cleanup_partial() → delete_embeddings + delete_chunks
update_document_status(Failed, error_info)
release lock
```

**Concurrencia:** El lock `ingest_pipeline` (adquirido via `try_acquire_lock`) garantiza ejecución serial. Si el lock está ocupado:
- `ingest()`: encola en backlog y retorna `OperationInProgress`
- `ingest_next()`: reencola el item del backlog

**Backlog queue:** Permite ingest asíncrono. `enqueue_ingest` valida y encola. `ingest_next` claim + procesa + marca done/failed.

---

## Flujo de Context/Retrieve

```
query → validate_query() → enforce_rate_limit() → provider.embed(query)
                                                        ↓
                                              fetch_candidates(db, config)
                                                    ↓           ↓
                                              [hierarchy]    [flat]
                                              list_chunk_    list_ready_
                                              embeddings_    chunk_
                                              hierarchical   embeddings
                                                    ↓           ↓
                                              rank_by_similarity(vector, candidates, k)
                                                        ↓
                                              enrich_with_hierarchy(ranked, meta)
                                                        ↓
                                              Vec<ScoredChunk>
```

**Para context (`build_context`), continúa:**
```
ranked → compute_result_kind(score, config, query, cited_count)
              ↓
         ResultKind ∈ {Context, NoResults, InsufficientContext}
              ↓
         build_citations_from_ranked(ranked, result_kind, threshold)
              ↓
         persist_trace(db, citations, ranked, ids, config)
              ↓
         ContextResponse { citations, metadata, instructions, ... }
```

**Clasificación de ResultKind:**
- `NoResults`: top_score < evidence_floor
- `InsufficientContext`: top_score < confidence_threshold O cited_chunks < required_facets
- `Context`: todo lo demás (evidencia suficiente)

**Facetas requeridas:** Heurística que detecta queries multi-tópico por conjunciones ("and", "y", "et", etc.) o cláusulas separadas por comas. Si detecta multi-faceta, requiere ≥2 citas distintas.

---

## Flujo de Refresh

```
refresh_corpus(db)
    ↓
begin_snapshot_build(snapshot_id)    ← crea snapshot "building"
    ↓
list_documents_by_status(Ready)     ← solo documentos listos
    ↓
attach_document_to_snapshot() × N   ← vincula cada doc
    ↓
activate_snapshot(snapshot_id)       ← swap atómico: nuevo activo, anterior supersedido
    ↓
RefreshResult { snapshot_id, document_count, previous_snapshot_id }
```

**Propiedades de atomicidad:**
- El swap se implementa a nivel de DB (probablemente UPDATE + transacción)
- Si no hay docs Ready, el snapshot se marca failed y el anterior permanece
- Si `activate_snapshot` falla, el snapshot anterior sigue activo
- **No hay rollback explícito** de los `attach_document_to_snapshot` si la activación falla — los attaches quedan como datos huérfanos en el snapshot "building"

---

## Recovery

**Trigger:** Llamado al inicio del CLI (`main.rs:140`):
```rust
let _ = engine::recovery::recover_interrupted_processing(&db)?;
```

**Política:** Todos los documentos en `Processing` → `Failed` con error estable:
- code: `"interrupted_processing_recovered"`
- message: `"Document was in processing state during startup recovery and was moved to failed"`

**Safeguard:** Solo actúa si el lock `ingest_pipeline` está libre. Si un ingest real está corriendo, no interfiere.

**Limitación:** No distingue entre "documento realmente interrumpido" y "documento que estaba procesándose lentamente". Si el proceso se reinicia mientras un ingest largo está corriendo, ese ingest se marcará como failed. Sin embargo, `retry_document` permite recuperarlo.

---

## Runtime Guards

### `check_ingest_allowed`

| RuntimeMode | Resultado |
|-------------|-----------|
| `LocalPrivateDemo` | ✅ Ok |
| `PublicPackagedDemo` | ❌ `RuntimeModeForbidden` |
| `Production` | ❌ `RuntimeModeForbidden` |

### `is_real_provider`

| provider_id | `is_real_provider` |
|-------------|-------------------|
| `"openai-compatible"` | `true` |
| `"gemini"` | `true` |
| `"eval"` | `false` |
| `"golden"` | `false` |
| `"mock"` | `false` |
| `"test"` | `false` |

**⚠ Hallazgo actualizado:** `check_ingest_allowed` sí se invoca en el path CLI actual: `crates/cli/src/commands/ingest.rs` llama `engine::runtime_guard::check_ingest_allowed(&config.runtime.mode)` antes de procesar ingest. El engine sigue sin revalidar el guard dentro de `ingest_next` o `ingest_internal`, así que el riesgo restante es el boundary interno si callers futuros usan `engine::ingest` sin pasar por el CLI. `runtime_guard` también se usa vía `is_real_provider` en `main.rs` para el provider disclosure banner.

---

## Decisiones de Diseño

### 1. Lock serializado para ingest
**Decisión:** Un solo lock `ingest_pipeline` serializa todos los ingests.
**Tradeoff:** Simple y seguro, pero limita throughput a un ingest a la vez. Adecuado para uso CLI/demo, pero sería bottleneck en producción con muchos archivos.

### 2. ResultKind como clasificación tri-state
**Decisión:** `Context | NoResults | InsufficientContext` con thresholds configurables.
**Tradeoff:** Permite al downstream distinguir "no hay datos" de "hay datos pero baja confianza". La heurística de facetas multi-tópico es pragmática pero imprecisa (ver errores).

### 3. Backlog queue para ingest diferido
**Decisión:** `enqueue_ingest` + `ingest_next` permiten cola sin procesamiento inmediato.
**Tradeoff:** Desacopla validación de procesamiento. Útil para batch processing. Sin embargo, no hay worker automático — depende del CLI o caller para procesar.

### 4. Hierarchical retrieval con fallback automático
**Decisión:** Si `use_hierarchy=true` pero no hay datos jerárquicos, fallback a flat.
**Tradeoff:** Nunca falla por falta de datos jerárquicos. El breadcrumb es "nice to have", no blocker. Bueno para progressive enhancement.

### 5. Cleanup parcial en fallo de ingest
**Decisión:** Borrar embeddings y chunks del documento fallido antes de marcarlo Failed.
**Tradeoff:** Evita datos huérfanos. Pero `cleanup_partial` ignora errores de eliminación (ver errores).

### 6. Golden fixtures hardcoded vs JSON
**Decisión:** Los fixtures existen en 4 variantes: `tests/golden/fixtures.rs` (Rust), `tests/golden/fixtures.json` (JSON), `cli/src/commands/evaluate.rs` (inline), y `engine/src/evaluate.rs` (framework).
**Tradeoff:** Flexibilidad de testing pero duplicación significativa con inconsistencias entre variantes (ver errores).

### 7. Trace persistence para auditoría
**Decisión:** Cada `build_context` persiste trace header + citas en DB.
**Tradeoff:** Permite auditoría completa post-hoc. Overhead de escritura en cada request de context. Adecuado para uso demo/CLI.

### 8. `Engine` struct vacía
**Decisión:** `lib.rs` declara `pub struct Engine;` que no se usa.
**Tradeoff:** Posiblemente placeholder para future stateful engine. Actualmente dead code.

---

## Conexiones con Otros Crates

### `engine` → depende de:
| Crate | Qué usa |
|-------|---------|
| `common` | Tipos (`CiteError`, `ContextResponse`, `Citation`, `Document`, `Chunk`, `ResultKind`, etc.) |
| `storage` | `Database` (todas las operaciones de persistencia) |
| `ingest` | `extractor::extract_text`, `chunker::chunk_text`, `validator::validate_file` |
| `providers` | `EmbeddingProvider` trait, `EvalProvider` |
| `config` | `RetrievalConfig`, `IngestConfig`, `RateLimitConfig`, `RuntimeMode` |
| `retrieval` | `rank_by_similarity`, `ScoredChunk` |
| `uuid` | Generación de IDs |
| `chrono` | Timestamps |
| `tracing` | (importado pero no usado explícitamente en el código visible) |

### Quién depende de `engine`:
| Crate | Qué usa |
|-------|---------|
| `cli` | Prácticamente todo: `ingest::*`, `retrieve::search/retrieve`, `context::build_context/read_context/get_trace`, `refresh::refresh_corpus`, `recovery::recover_interrupted_processing`, `runtime_guard::is_real_provider`, `evaluate::run_evaluation` |

### Flujo de dependencia:
```
cli → engine → {storage, ingest, providers, retrieval, config, common}
```

`engine` es el único consumidor directo de `storage`, `ingest`, `providers`, y `retrieval` desde el CLI. El CLI no accede a estos crates directamente (excepto `providers::eval::EvalProvider` en el comando evaluate).

---

## Cobertura de Tests

| Módulo | Tests unitarios | Tests integración |
|--------|----------------|-------------------|
| `context.rs` | 16 | — |
| `ingest.rs` | 7 | — |
| `retrieve.rs` | 15 | — |
| `refresh.rs` | 4 | — |
| `recovery.rs` | 3 | — |
| `runtime_guard.rs` | — | 3 (`tests/runtime_mode.rs`) |
| `evaluate.rs` | 3 | — |
| `golden_provider.rs` | — | 5 (`tests/golden/provider.rs`) |
| Golden integration | — | 3 (`tests/golden_test.rs`) |

**Total:** ~48 tests. Buena cobertura general. Notable ausencia de tests para:
- `cleanup_partial` con errores reales de DB
- `run_pipeline` con archivos que retornan 0 páginas (edge case)
- `required_facets_for_query` con queries que contienen "e" o "en" como palabras comunes (falsos positivos)
