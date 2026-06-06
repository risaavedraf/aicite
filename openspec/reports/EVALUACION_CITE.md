# Evaluación de Cite CLI v0.3.0

Evaluación exhaustiva de todos los comandos de Cite, realizada el 2026-06-06.
Actualizada el 2026-06-06 (sesión vespertina) — ver sección "Actualización post-fix" al final.

---

## Resumen Ejecutivo

Cite es un motor de recuperación semántica para agentes AI. La herramienta funciona bien en su core (búsqueda y recuperación), pero tiene problemas con los filtros jerárquicos y la documentación desactualizada.

**Calificación general: 7/10 → 8/10** (post-fix)

---

## Comandos Evaluados

### 1. `health` ✅

**Propósito:** Verificar estado del sistema

**Salida:**
```json
{
  "status": "ok",
  "version": "0.3.0",
  "schema_version": "context-v1",
  "runtime_mode": "local_private_demo",
  "provider": {
    "provider_id": "gemini",
    "model": "gemini-embedding-001",
    "status": "reachable",
    "latency_ms": 1218
  },
  "database": {
    "status": "exists",
    "document_count": 40,
    "chunk_count": 351
  }
}
```

**Evaluación:**
- ✅ Información completa y clara
- ✅ Incluye latencia del provider
- ✅ Muestra estado de la API key enmascarada
- ⚠️ `latency_ms: 1218` es alto para una simple verificación

---

### 2. `list` ✅

**Propósito:** Listar documentos en el corpus

**Salida:** Lista de 40 documentos con `document_id`, `display_name`, `status`, `chunk_count`, `retry_count`, `created_at`

**Evaluación:**
- ✅ Información completa
- ✅ JSON bien estructurado
- ⚠️ Sin paginación — con muchos documentos podría ser problemático
- ❌ No hay opción de filtrar por status (ready/failed)

---

### 3. `get` ✅

**Propósito:** Obtener metadata de un documento específico

**Salida:**
```json
{
  "document_id": "doc_347a7d3808fc",
  "display_name": "README.md",
  "status": "ready",
  "chunk_count": 18,
  "retry_count": 0,
  "max_retry_count": 3,
  "next_retry_at": null,
  "error": null
}
```

**Evaluación:**
- ✅ Completo y claro
- ✅ Incluye `max_retry_count` y `next_retry_at` para debugging

---

### 4. `search` ⚠️

**Propósito:** Búsqueda rápida con scores y snippets

**Modos:**
- Compact (default): `id`, `source`, `score`, `preview`
- Full (`--full`): agrega `document_id`, `chunk_id`, `page`, `offset`, `topic_name`, `concept_name`, `breadcrumb`

**Evaluación:**
- ✅ Compact es liviano (~40 tokens por resultado)
- ✅ Full tiene toda la metadata para debugging
- ❌ **`--topic` y `--concept` NO funcionan** — retornan los mismos resultados sin filtrar
- ❌ `preview` es muy corto (~100 chars) en compact

**Ejemplo problemático:**
```bash
cite search "retrieval" --topic "Authentication" --json
# Retorna los mismos resultados que sin --topic
```

---

### 5. `retrieve` ✅

**Propósito:** Recuperar chunks con texto completo

**Modos:**
- Compact: `id`, `source`, `score`, `text` (completo)
- Full: agrega metadata detallada

**Evaluación:**
- ✅ `text` retorna el chunk completo (500-1000 chars)
- ✅ Diferencia clara con `search` (que solo da snippets)
- ❌ **Mismo problema con `--topic` y `--concept`** — no filtran

---

### 6. `context` ⚠️

**Propósito:** Construir context pack completo para agentes

**Modos:**
- Compact (default): `result_kind`, `citations[]` con `id`, `source`, `snippet`, `score`, `trace_id`
- Full (`--full`): agrega `context_pack_id`, `query_id`, `instructions`, metadata completa por citation

**Evaluación:**
- ✅ `result_kind` es útil (`context`, `insufficient_context`, `no_results`)
- ✅ `instructions` en full mode guía al agente
- ⚠️ **Compact no retorna texto completo** — snippet es ~200 chars
- ⚠️ Para responder preguntas, necesitás `--full` o usar `retrieve`
- ❌ **`--topic` y `--concept` NO funcionan**
- ❌ `topic_name`, `concept_name`, `breadcrumb` siempre son `null` en la salida

**Problema fundamental:**
```
compact: snippet = ~200 chars (no suficiente para responder)
full: text = 500-1000 chars (suficiente pero 60% más tokens)
```

---

### 7. `read` ✅

**Propósito:** Leer una citation o chunk por ID

**Parámetros:**
- `--citation-id` + `--trace-id` (requiere ambos)
- `--chunk-id` + `--document-id` (requiere ambos)

**Evaluación:**
- ✅ Funciona correctamente
- ✅ Retorna texto completo de la citation
- ⚠️ Requiere `trace-id` para citation — no es intuitivo

---

### 8. `trace` ✅

**Propósito:** Inspeccionar metadata de una request de retrieval

**Salida:**
```json
{
  "trace_id": "...",
  "query_id": "...",
  "context_pack_id": "...",
  "timestamp": "...",
  "embedding_model_registry_id": "gemini-embedding-001",
  "provider": "gemini",
  "document_ids": [...],
  "citation_ids": ["c1", "c2", "c3", "c4", "c5"],
  "retrieval_top_k": 5,
  "evidence_floor": 0.5,
  "confidence_threshold": 0.7,
  "ranking_method": "vector_cosine_v1"
}
```

**Evaluación:**
- ✅ Muy útil para debugging
- ✅ Información completa del pipeline de retrieval

---

### 9. `evaluate` ✅

**Propósito:** Ejecutar golden dataset para verificar calidad

**Salida:**
```json
{
  "total": 10,
  "passed": 10,
  "failed": 0,
  "hit_rate": 1.0,
  "threshold": 0.8,
  "overall_pass": true,
  "results": [...]
}
```

**Evaluación:**
- ✅ Excelente para CI/CD
- ✅ 10 fixtures cubren: direct_fact, no_results, ambiguous, multi_chunk, prompt_injection, hierarchical
- ✅ Hit rate 1.0 — todos pasan
- �ary ❓ ¿Cómo se agregan nuevos fixtures? No hay documentación clara

---

### 10. `refresh` ⏭️ (no testeado)

**Propósito:** Refresh corpus con atomic snapshot swap

**Nota:** No testeado porque podría modificar el corpus existente.

---

### 11. `retry` ⏭️ (no testeado)

**Propósito:** Reintentar un documento fallido

**Nota:** No hay documentos fallidos en el corpus actual para testear.

---

### 12. `ingest` ⏭️ (no testeado)

**Propósito:** Ingestar un documento

**Nota:** No testeado para evitar modificar el corpus de evaluación.

---

### 13. `setup` ⏭️ (no testeado)

**Propósito:** Configurar API keys y provider

**Nota:** Ya está configurado, no es necesario testear.

---

## Comparativa: search vs retrieve vs context

| Característica | search | retrieve | context |
|----------------|--------|----------|---------|
| **Uso principal** | Explorar | Obtener texto | Responder |
| **Modo compact** | preview (~100 chars) | text (full) | snippet (~200 chars) |
| **Modo full** | + metadata | + metadata | + instructions + metadata |
| **Tokens (compact)** | ~50 | ~150 | ~80 |
| **Tokens (full)** | ~100 | ~200 | ~1000 |
| **result_kind** | ❌ | ❌ | ✅ |
| **instructions** | ❌ | ❌ | ✅ (full) |
| **--topic/--concept** | ❌ no funciona | ❌ no funciona | ❌ no funciona |

---

## Problemas Encontrados

### Críticos

1. **Filtros `--topic` y `--concept` no funcionan**
   - Retornan los mismos resultados que sin filtro
   - Afecta a `search`, `retrieve` y `context`

2. **Jerarquía no se refleja en salida**
   - `topic_name`, `concept_name`, `breadcrumb` siempre son `null`
   - La v0.2.0 prometía hierarchical retrieval pero no está implementado

### Moderados

3. **Documentación desactualizada**
   - `agent-usage-guide.md` dice que compact es una propuesta
   - En realidad ya está implementado como default

4. **Compact mode no da texto completo**
   - Para responder preguntas, necesitás `--full` o `retrieve`
   - El patrón search→context no funciona bien

5. **Sin paginación en `list`**
   - Con muchos documentos podría ser problemático

### Menores

6. **Latencia alta en `health`** (1218ms)
7. **`read` requiere `trace-id`** para citations — no es intuitivo
8. **`evaluate` no documenta cómo agregar fixtures**

---

## Preguntas para Mejorar la Herramienta

### Sobre Filtros Jerárquicos

1. **¿Los filtros `--topic` y `--concept` están implementados?**
   - Si no, ¿cuándo se planea implementarlos?
   - Si sí, ¿por qué no funcionan?

2. **¿La jerarquía Document → Topic → Concept → Chunk está ingiriendo datos?**
   - Los campos `topic_name`, `concept_name`, `breadcrumb` siempre son `null`
   - ¿Es un problema de ingestion o de retrieval?

### Sobre Compact Mode

3. **¿El snippet en compact debería ser más largo?**
   - Actual: ~200 chars
   - Propuesto: ~300-400 chars
   - ¿O debería existir un `--snippet-length` flag?

4. **¿Debería existir un modo `--compact-full-text`?**
   - Compact metadata pero texto completo del chunk
   - Balance entre tokens y utilidad

### Sobre Uso de Agentes

5. **¿Cuál es el patrón de uso recomendado para agentes?**
   - ¿search → context (con --full)?
   - ¿O search → retrieve?
   - ¿O solo context --full?

6. **¿Se planea un modo streaming para agentes?**
   - Para queries largas, streaming mejoraría latencia percibida

### Sobre Rendimiento

7. **¿Por qué `health` tarda 1218ms?**
   - ¿Está haciendo un health check al provider?
   - ¿Se puede cachear?

8. **¿Se planea batching de queries?**
   - `cite context-batch` con múltiples queries en una llamada
   - Reduciría overhead de embedding

### Sobre Evaluación

9. **¿Cómo se agregan nuevos fixtures al golden dataset?**
   - ¿Es solo agregar JSON?
   - ¿Hay un schema formal?

10. **¿Se planea evaluación automática en CI?**
    - GitHub Action que ejecute `cite evaluate` en cada PR

### Sobre Documentación

11. **¿Quién mantiene la documentación actualizada?**
    - `agent-usage-guide.md` está desactualizado
    - ¿Hay un proceso de sincronización doc/código?

12. **¿Se planea documentación para cada flag?**
    - Ej: cuándo usar `--flat` vs jerárquico

---

## Recomendaciones

### Para la v0.3.1

1. **Fixear filtros `--topic` y `--concept`**
2. **Actualizar documentación** (agent-usage-guide.md)
3. **Agregar `--max-snippet-chars`** para controlar tamaño de snippets

### Para la v0.4.0

4. **Implementar streaming** para queries largas
5. **Agregar paginación** a `list`
6. **Documentar golden dataset** y cómo contribuir fixtures

### Para el Agente

7. **Usar `search` para explorar** (liviano en tokens)
8. **Usar `retrieve` para obtener texto** (mejor que context --full)
9. **Usar `context --full` solo cuando necesites `result_kind` e `instructions`**

### Sobre Documentación y Sincronización

13. **¿Existe algún mecanismo para detectar doc desactualizada?**
    - Actualmente no hay sync automático entre código y openspec
    - El CHANGELOG-docs.md es manual

14. **¿Se podría implementar un comando `cite check-docs`?**
    - Comparar output de `--help` con lo documentado
    - Detectar si ejemplos de la doc funcionan contra el binario actual
    - Marcar archivos como stale si el código se modificó después

15. **¿Cómo trackear qué se implementó vs qué se documentó?**
    - No hay campo `last_modified` en documentos
    - No hay forma automática de detectar desincronización

---

## Conclusión

Cite es una herramienta sólida para recuperación semántica. El core funciona bien:
- `search` es rápido y liviano
- `retrieve` da texto completo
- `context` tiene buena metadata para agentes
- `evaluate` es excelente para testing

Los principales problemas son:
- Filtros jerárquicos no funcionan
- Documentación desactualizada
- Compact mode no da texto completo

**Prioridad de fixes:**
1. Filtros (crítico)
2. Doc (moderado)
3. Snippet length (menor)

---

## Actualización post-fix (2026-06-06 sesión vespertina)

### Cambios realizados

#### 1. Fix de filtros `--topic`/`--concept` ✅

**Problema original:** Los filtros retornaban los mismos resultados que sin filtro.

**Causa raíz encontrada:** Dos problemas superpuestos:

1. **SQL comparaba nombre contra ID**: El filtro pasaba `"Authentication"` (nombre) pero el SQL comparaba contra `c.topic_id` (ej: `"t1"`). Fix: agregar `OR t.name = ?1` al WHERE.

2. **Flat path ignoraba filtros**: Cuando no hay datos jerárquicos (`has_hierarchy_data() == false`), el código tomaba el path "flat" que llamaba `list_ready_chunk_embeddings()` sin filtros. Fix: si hay filtros activos pero no hay jerarquía, retornar `[]` vacío.

**Estado actual:**
- `cite search "retrieval" --topic "nonexistent"` → `[]` ✅ (antes retornaba resultados)
- `cite search "retrieval" --concept "JWT"` → `[]` ✅ (idem)
- Los filtros ahora soportan nombre Y ID: `--topic "Authentication"` y `--topic "t1"` ambos funcionan
- **Nota:** El corpus actual fue ingerido SIN jerarquía (flat), así que los filtros siempre retornan `[]`. Para probar con datos reales, hay que re-ingresar documentos con `--hierarchy`.

#### 2. Fix de `cosine_similarity` con NaN/Inf ✅

**Problema:** Vectores con NaN o Infinity retornaban `Some(NaN)` en vez de `None`.

**Fix:** Early rejection de inputs NaN/Inf en `cosine_similarity()`.

#### 3. Tests añadidos: +36 tests nuevos

| Crate | Tests nuevos | Total |
|-------|-------------|-------|
| common | +15 | 31 |
| storage | +11 | 114 |
| retrieval | +4 | 20 |
| graph | +3 | 22 |
| config | +3 | 14 |

**Total suite: 388 tests, 0 fallos, clippy limpio.**

### Preguntas de la evaluación original — Respuestas

| # | Pregunta | Respuesta |
|---|----------|-----------|
| 1 | ¿Los filtros `--topic`/`--concept` están implementados? | **Sí**, estaban implementados pero con 2 bugs. Ahora funcionan correctamente. |
| 2 | ¿La jerarquía Document→Topic→Concept→Chunk está ingiriendo datos? | **No en este corpus.** Los docs se ingerieron sin `--hierarchy`. La infraestructura está lista, falta re-ingesta. |
| 3 | ¿El snippet en compact debería ser más largo? | **Sí**, ~100 chars es corto. `--full` da texto completo pero ~60% más tokens. |
| 4 | ¿Debería existir un modo `--compact-full-text`? | **Pendiente.** No implementado. |
| 5 | ¿Cuál es el patrón de uso recomendado para agentes? | `search` para explorar (liviano), `retrieve` para texto completo, `context --full` para respuestas con metadata. |
| 6 | ¿Se planea streaming? | **No implementado.** Post-MVP. |
| 7 | ¿Por qué `health` tarda 1218ms? | Latencia del provider (Gemini API). No es cacheable porque es health check real. |
| 8 | ¿Se planea batching? | `context-batch` no existe aún. `check-docs` lo detectó como `unrecognized subcommand`. |
| 9 | ¿Cómo se agregan fixtures al golden dataset? | JSON en `crates/engine/src/evaluate.rs` función `golden_fixtures()`. |
| 10 | ¿Se planea evaluación automática en CI? | `evaluate` ya corre en los tests. GitHub Action pendiente. |
| 11 | ¿Quién mantiene la documentación? | **`check-docs` ya funciona** — detectó 6 items desactualizados en `agent-usage-guide.md`. |
| 12 | ¿Se planea documentación para cada flag? | Los docstrings están, pero la guía de uso está desactualizada. |

### Resultados de `check-docs` (nuevo)

El comando `cite check-docs` ya está implementado y funcional:

```
Results: 3 ok, 6 outdated, 0 warnings
```

**Items desactualizados detectados:**
1. ❌ `context-batch` subcommand — no existe
2. ❌ `--min-score` flag — no implementado
3. ❌ `--max-snippet-chars` flag — no implementado
4. ❌ `--fields` flag — no implementado
5. ❌ Formato de output de `context` cambiado
6. ❌ `--max-chars` flag output difiere

### Nueva calificación: **8/10**

**Mejoras:**
- ✅ Filtros arreglados (crítico)
- ✅ `check-docs` implementado y funcionando
- ✅ NaN/Inf fix en cosine_similarity
- ✅ +38 tests nuevos (388 total)

**Pendiente:**
- ⚠️ Re-ingesta con jerarquía para probar filtros con datos reales
- ⚠️ 6 flags/features documentadas pero no implementadas
- ⚠️ Snippet length control

---

## Actualización: Jerarquía probada con datos reales (2026-06-06 sesión nocturna)

### Re-ingesta con `CITE_BUILD_HIERARCHY=true`

Se re-ingirieron documentos del corpus con jerarquía activa.

**Resultado:**
- 19 documentos ingeridos exitosamente (5 fallaron por rate limit de Gemini API 429)
- 186 chunks, 152 topics, 70 concepts
- Jerarquía H2→Topic, H3→Concept correctamente mapeada

**Documentos ingeridos:**
- PRDs: 01-15 (excepto 03, 06, prd_changelog por 429)
- Guides: agent-usage-guide, demo (installation falló por 429)
- Architecture: cite-notes-hybrid, front-lobe-engine, rename-to-cite (v0.2.0 falló por 429)
- README.md

**Nota sobre 429:** El embedder provider (Gemini) es el cuello de botella principal. 5 documentos no se pudieron ingestar por rate limiting. Esto se resolverá en v0.3.2/0.3.3 con un cambio de arquitectura del embedder.

### Filtros `--topic`/`--concept` con datos jerárquicos ✅

**Antes (sin jerarquía):** Los filtros retornaban `[]` siempre.

**Ahora (con jerarquía):**

```bash
# --topic filtra correctamente
cite search "ingestion pipeline" --topic "Corpus ingestion" --json
# → Solo chunks del topic "Corpus ingestion" en 04-functional-requirements.md

# --concept filtra correctamente
cite search "who uses this tool" --concept "Corpus owner / operator" --json
# → Solo chunks del concept "Corpus owner / operator" en 02-users-and-problems.md

# Sin filtro vs con filtro muestra diferencia real:
cite search "API endpoints" --json                    # → 5 results (api-contract, acceptance-criteria)
cite search "API endpoints" --topic "Non-goals" --json # → 2 results (solo product-brief Non-goals)
```

### Breadcrumb en salida `--full` ✅

**search --full:**
```json
{
  "topic_name": "Corpus management",
  "concept_name": null,
  "breadcrumb": "04-functional-requirements.md > Corpus management"
}
```

**retrieve --full:**
```json
{
  "topic_name": "Process model — single-shot durable CLI",
  "concept_name": null,
  "breadcrumb": "07-system-architecture.md > Process model — single-shot durable CLI"
}
```

**context --full:**
```json
{
  "topic_name": "Commands",
  "concept_name": "`cite list`",
  "breadcrumb": "09-api-contract.md > Commands > `cite list`"
}
```

**Breadcrumb de 3 niveles funciona:** `document > topic > concept` se muestra correctamente.

### Evaluate con nueva DB ✅

```
Total cases: 10 | Passed: 10 | Failed: 0
```

El golden dataset sigue pasando al 100% con la nueva base de datos jerárquica.

### Calificación actualizada: **8.5/10**

**Mejoras desde última sesión:**
- ✅ Jerarquía probada con datos reales (152 topics, 70 concepts)
- ✅ Filtros `--topic`/`--concept` funcionando con datos reales
- ✅ Breadcrumb de 3 niveles en search/retrieve/context `--full`
- ✅ Evaluate sigue 10/10 con nueva DB

**Pendiente para v0.3.2/0.3.3:**
- ⚠️ Resolver dependencia del embedder provider (rate limit 429)
- ⚠️ 6 flags/features documentadas pero no implementadas
- ⚠️ Snippet length control
- ⚠️ Considerar `cite delete` o `cite reset` (ahora se hace manualmente con Python/SQLite)

### Análisis de calidad de retrieval (2026-06-06)

**Problema identificado:** Los scores de similarity son bajos — ninguna query real llega a 0.8.

```
Query                              Top score   Spread (1-5)
"document ingestion"                0.7002     0.68-0.70
"how to configure API keys"         0.6361     0.58-0.64
"acceptance criteria for retrieval"  0.7365     0.69-0.74
"what happens when doc fails"       0.7628     0.70-0.76
```

**Causas:**
1. **Embedding model:** gemini-embedding-001 es general-purpose, no optimizado para retrieval técnico
2. **Chunking:** 1000 chars fixed split corta contexto a mitad de conceptos
3. **Sin re-ranking:** Solo cosine similarity pura, no second-pass
4. **Sin hybrid search:** No combina vector + keyword (FTS5)

**Nota:** evaluate pasa 10/10 con golden fixtures, pero los fixtures fueron escritos para matchear con este modelo, no al revés. Los scores reales son más bajos que lo que el evaluate sugiere.

**Roadmap de mejora:**
- v0.3.2: Ollama provider + `cite doctor` + error messages accionables + ingesta resumible (RFC: rfc-embedding-providers.md)
- v0.3.2/0.3.3: Tags + Note Add + re-embed migration (RFC: rfc-tags-and-note-add.md)
- v0.3.3: ONNX provider + HuggingFace API + setup wizard
- v0.4.0: Semantic chunking (respetar headings, sentence boundaries, 300-800 chars) + re-ranking (cross-encoder)
- v0.5.0: Hybrid search (vector * 0.7 + FTS5 keyword * 0.3)

**Embedder local — modelos considerados:**

| Modelo | MTEB | Params | VRAM | Velocidad (GPU) | Notas |
|---|---|---|---|---|---|
| qwen3-embedding 4B (Q4) | ~67 | 4B | 2.5GB | <50ms | Recomendado para GPU |
| qwen3-embedding 0.6B | ~60 | 600M | 1.2GB | ~100ms | Fallback ligero |
| nomic-embed-text v1.5 | 62.28 | 137M | 300MB | ~30ms | Mejor CPU |
| gemini-embedding-001 | 68.32 | N/A | N/A | ~1000ms | Status quo (rate limited) |

**RTX 3070 8GB:** Todos los modelos entran sobrados. qwen3-embedding 4B es el sweet spot (MTEB ~67, casi igual que Gemini 68.32).

**RFCs detallados:**
- `openspec/rfc/active/rfc-embedding-providers.md` — Sistema de providers pluggable
- `openspec/rfc/active/rfc-tags-and-note-add.md` — Tags, retrieval quality roadmap
