# Revisión: `crates/retrieval` — Motor de similitud coseno y ranking top-k

**Revisor:** el Gentleman subagent de revisión
**Fecha:** 2026-06-02
**Alcance:** 1 archivo en crates/retrieval/src/
**Metodología:** Inspección manual del archivo fuente + verificación de dependencias cruzadas con `storage`, `providers`, `engine`, `config` y PRD

---

## Resumen del Crate

`retrieval` es el crate más pequeño del proyecto: un único archivo (`src/lib.rs`, ~260 líneas) que expone operaciones puras de matemáticas de recuperación. No tiene I/O, no accede a base de datos, no llama a providers externos.

**Propósito:** Comparar un vector de query contra vectores de embeddings de chunks y devolver los top-k resultados ordenados por similitud coseno.

**Exports públicos:**
- `ScoredChunk` — struct que representa un chunk con su score de relevancia y metadata jerárquica opcional
- `cosine_similarity(a, b) → Option<f32>` — cálculo puro de similitud coseno entre dos slices de f32
- `rank_by_similarity(query, candidates, k) → Vec<ScoredChunk>` — pipeline completo: score + sort + truncate

**Dependencias:** Solo `common` (workspace) y `storage` (para el tipo `ChunkEmbeddingRecord`).

**Tests:** 5 tests unitarios que pasan. Cubren: vectores idénticos, dimensión incompatible, norma cero, ranking top-k, y salteo silencioso de candidatos inválidos.

---

## Flujo Principal

```
1. engine/src/retrieve.rs::ranked_candidates() orquesta el pipeline completo
   ├── valida query (vacía, solo puntuación, largo máximo)
   ├── enforce_rate_limit() vía storage::rate_limits
   ├── provider.embed(query) → query_vector: Vec<f32>
   ├── fetch_candidates() → Vec<ChunkEmbeddingRecord> (de storage)
   ├── rank_by_similarity() ← AQUÍ ENTRA RETRIEVAL  ◄──
   │   ├── para cada candidato: cosine_similarity(query_vector, candidate.vector)
   │   ├── filtra los que devuelven None (dimensión inválida o norma cero)
   │   ├── ordena por score descendente (total_cmp)
   │   └── trunca a k resultados
   └── enrich_with_hierarchy() (en engine, agrega topic/concept a los ScoredChunk)

2. engine/src/context.rs usa los ScoredChunk para armar el ContextResponse
   con citations, breadcrumb, metadata, etc.
```

El crate `retrieval` participa solo en el paso de ranking. Todo lo demás (validación, rate limiting, embedding, fetch de candidatos, enriquecimiento jerárquico) ocurre en `engine`.

---

## Módulos/Archivos Clave

### `src/lib.rs` — Archivo único

#### `ScoredChunk` (struct, líneas 40-64)
- 14 campos: los 9 primeros reflejan `ChunkEmbeddingRecord` (chunk_id, document_id, display_name, section_id, chunk_index, text, page, offset_start, offset_end), más `score: f32`, más 4 campos opcionales de jerarquía (topic_id, topic_name, concept_id, concept_name).
- Los campos de jerarquía se inicializan en `None` por `rank_by_similarity` y se poblan después por `engine::enrich_with_hierarchy`.
- `#[derive(Debug, Clone)]` — correcto, no implementa `PartialEq` (f32 score hace que la igualdad exacta sea problemática).

#### `cosine_similarity(a, b) → Option<f32>` (líneas 99-120)
- **Acumulación en f64:** Usa `f64` para dot product y normas. Esto reduce errores de acumulación flotante con vectores largos (embeddings típicos de 384-1536 dimensiones). Decisión correcta.
- **Edge cases cubiertos:** dimensión diferente → None, slice vacío → None, norma cero → None.
- **Firma `Option<f32>`:** Elegante. Los callers usan `?` para propagar silenciosamente.

#### `rank_by_similarity(query_vector, candidates, k) → Vec<ScoredChunk>` (líneas 152-178)
- Pipeline funcional: `filter_map` → `collect` → `sort_by` → `truncate`.
- `filter_map` + `?` sobre `cosine_similarity`: candidatos inválidos se descartan silenciosamente (documentado en docstring).
- `sort_by` con `total_cmp`: maneja NaN correctamente (NaN se ordena como mayor que cualquier valor, queda al final después del truncate).
- Construcción directa de `ScoredChunk` mapeando campos desde `ChunkEmbeddingRecord` — duplicación manual de campos.

---

## Decisiones de Diseño

### 1. Crate puro sin I/O
**Decisión:** `retrieval` no toca base de datos ni providers. Toda la orquestación vive en `engine`.
**Tradeoff:** Separación limpia. El crate se puede testear sin mocks de storage/providers. Pero la lógica completa de retrieval está fragmentada entre `retrieval` (math) y `engine` (pipeline), lo que hace que `retrieval` sea casi demasiado pequeño.

### 2. ScoredChunk como flat struct vs wrapper sobre ChunkEmbeddingRecord
**Decisión:** `ScoredChunk` replica los campos de `ChunkEmbeddingRecord` en lugar de envolverlo (`chunk: ChunkEmbeddingRecord`).
**Tradeoff:** Evita un nivel de indirection (`item.chunk.text` vs `item.text`), pero requiere mantener la alineación manual entre ambos structs. Si `ChunkEmbeddingRecord` agrega un campo, `ScoredChunk` y `rank_by_similarity` deben actualizarse manualmente.

### 3. Jerarquía fuera del crate retrieval
**Decisión:** `rank_by_similarity` no conoce `HierarchicalChunkEmbedding`. Los campos topic/concept en `ScoredChunk` siempre salen `None` del crate retrieval. El engine los pobla después.
**Tradeoff:** Buena separación de responsabilidades. El crate retrieval no depende de la lógica jerárquica de storage. Pero los consumidores deben saber que `ScoredChunk` está incompleto hasta que el engine lo enriquezca.

### 4. Acumulación en f64
**Decisión:** Internamente `cosine_similarity` usa `f64` para dot product y normas, casteando de vuelta a `f32` al final.
**Tradeoff:** Mejor precisión numérica para vectores de alta dimensión. Costo: un cast extra por componente. Razónable para la carga de trabajo (embeddings de 384-1536 dims).

### 5. Silencio ante candidatos inválidos
**Decisión:** `rank_by_similarity` descarta candidatos con dimensión incorrecta o norma cero sin error.
**Tradeoff:** Robusto ante datos corruptos, pero puede hacer debugging difícil si todos los embeddings están mal dimensionados (devuelve `Vec` vacío sin aviso).

---

## Conexiones con Otros Crates

| Crate | Conexión | Detalle |
|-------|----------|---------|
| **storage** | Importa `ChunkEmbeddingRecord` | Tipo de input para `rank_by_similarity`. Retrieval lee los campos pero no accede a la DB. |
| **common** | Implícito (workspace dep) | No se usa directamente en `lib.rs` — el crate no devuelve `CiteError`, solo `Option` y `Vec`. |
| **engine** (`src/retrieve.rs`) | Caller principal | Construye `RetrievalRequest`, llama a `rank_by_similarity`, enriquece con jerarquía, convierte a `Hit`. |
| **engine** (`src/context.rs`) | Usa `ScoredChunk` | Pasa `ScoredChunk` a la construcción de `ContextResponse` con citations. |
| **providers** | Ninguna conexión directa | El embedding se hace en engine antes de llamar a retrieval. |
| **config** | Ninguna conexión directa | Config (top_k, thresholds) se resuelve en engine. |

### Observación sobre FR-109 (Rate Limiting)

El crate `retrieval` **no interactúa con rate limiting**. La clave compuesta `(runtime_mode + corpus_id + provider_id + retrieval_scope)` que requiere FR-109 se construye (incorrectamente) en `engine/src/retrieve.rs::rate_limit_key()`, que usa solo `provider_id()`. Esto ya fue reportado en las revisiones de storage y compliance. Dado que retrieval no participa en rate limiting, no hay acción que tomar en este crate — el fix es responsabilidad del engine.

---

## Veredicto General

El crate `retrieval` es un ejemplo de diseño limpio: pequeño, puro, testable, con responsabilidad única. No tiene bugs. Las decisiones de diseño (f64, silencio ante inválidos, separación de jerarquía) son razonables. La principal deuda es la duplicación de campos entre `ScoredChunk` y `ChunkEmbeddingRecord`, que es un tradeoff deliberado por simplicidad de acceso.
