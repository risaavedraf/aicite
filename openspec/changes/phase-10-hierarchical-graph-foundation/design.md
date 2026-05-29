# Phase 10 Design — Hierarchical Graph Foundation

## 1. Entity-Relationship Model

```
┌─────────────┐       ┌─────────────┐       ┌─────────────┐
│  documents   │       │   topics     │       │  concepts    │
├─────────────┤       ├─────────────┤       ├─────────────┤
│ document_id ◄───┐   │ topic_id  ◄───┐   │ concept_id ◄───┐
│ display_name │   │   │ document_id ───┘   │ topic_id   ───┘
│ file_path    │   │   │ name        │       │ name        │
│ file_type    │   │   │ summary     │       │ summary     │
│ status       │   │   │ embedding   │       │ embedding   │
│ chunk_count  │   │   │ chunk_count │       │ chunk_count │
│ ...          │   │   │ created_at  │       │ created_at  │
└─────────────┘   │   └─────────────┘       └─────────────┘
                  │                                 │
                  │   ┌─────────────┐               │
                  │   │   chunks     │               │
                  │   ├─────────────┤               │
                  └───┤ document_id │               │
                      │ chunk_id  ◄─┼───┐           │
                      │ topic_id    │◄──┼───────────┘
                      │ concept_id  │   │
                      │ chunk_index │   │   ┌─────────────────┐
                      │ text        │   │   │ semantic_links   │
                      │ page        │   │   ├─────────────────┤
                      │ offset_start│   │   │ link_id         │
                      │ offset_end  │   │   │ source_chunk_id ─┘
                      │ created_at  │   │   │ target_chunk_id ───┐
                      └─────────────┘   │   │ similarity_score │
                            │           │   │ link_type        │
                            │           │   │ created_at       │
                            └───────────┘   └─────────────────┘
                                │                   │
                                └───────────────────┘
```

## 2. Análisis de Formas Normales

### 2.1 Tabla `topics`

```sql
CREATE TABLE topics (
    topic_id    TEXT PRIMARY KEY,
    document_id TEXT NOT NULL REFERENCES documents(document_id),
    name        TEXT NOT NULL,
    summary     TEXT,
    embedding   BLOB,
    chunk_count INTEGER DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
```

**1NF** ✅ — Todos los atributos son atómicos. No hay grupos repetidos.
- `embedding` es un BLOB (vector de floats serializado), no una colección.
- `chunk_count` es un escalar.

**2NF** ✅ — PK simple (`topic_id`), no hay dependencias parciales posibles.

**3NF** ✅ — No hay dependencias transitivas.
- `document_id` es FK, no determina otros atributos no-key.
- `name`, `summary` dependen directamente de `topic_id`.
- `chunk_count` depende de `topic_id` (es un atributo derivado del topic).

**BCNF** ✅ — El único determinante es `topic_id` (PK).

**⚠️ Decisión de diseño: `chunk_count` denormalizado**

`chunk_count` se puede computar: `SELECT COUNT(*) FROM chunks WHERE topic_id = ?`. 
Se denormaliza por:
- Evitar COUNT(*) en cada `topics list` (frecuente en CLI)
- El costo de mantenerlo consistente es bajo (incrementar al crear chunk)
- Riesgo: desincronización si se borran chunks sin actualizar

**Mitigación**: trigger o función que mantenga `chunk_count` actualizado.

---

### 2.2 Tabla `concepts`

```sql
CREATE TABLE concepts (
    concept_id  TEXT PRIMARY KEY,
    topic_id    TEXT NOT NULL REFERENCES topics(topic_id),
    name        TEXT NOT NULL,
    summary     TEXT,
    embedding   BLOB,
    chunk_count INTEGER DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
```

**1NF** ✅ — Atributos atómicos, sin grupos repetidos.

**2NF** ✅ — PK simple (`concept_id`).

**3NF** ✅ — Sin dependencias transitivas.
- `topic_id` es FK a `topics`, no determina otros atributos de `concepts`.
- `name`, `summary` dependen directamente de `concept_id`.

**BCNF** ✅ — Único determinante: `concept_id`.

**Misma decisión**: `chunk_count` denormalizado, misma mitigación que `topics`.

---

### 2.3 Tabla `chunks` (modificada)

```sql
-- Existente (v0.1.0)
chunks(chunk_id PK, document_id FK, section_id, chunk_index, text, page, offset_start, offset_end, created_at)

-- Nuevas columnas (Phase 10)
ALTER TABLE chunks ADD COLUMN concept_id TEXT REFERENCES concepts(concept_id);
ALTER TABLE chunks ADD COLUMN topic_id TEXT REFERENCES topics(topic_id);
```

**1NF** ✅ — Atributos atómicos.

**2NF** ✅ — PK simple (`chunk_id`).

**3NF** ⚠️ **ANÁLISIS REQUERIDO**

Dependencias transitivas potenciales:

```
chunk_id → document_id → (display_name, file_path, file_type, ...)
chunk_id → topic_id → (topic.name, topic.document_id, ...)
chunk_id → concept_id → (concept.name, concept.topic_id, ...)
```

Esto parece transitivo: `chunk_id → topic_id → topic.document_id`. Pero:
- `topic.document_id` NO es un atributo de `chunks` — vive en `topics`.
- `chunks.document_id` es independiente de `chunks.topic_id` (no se deriva de topic).
- `chunks.topic_id` es nullable y puede ser NULL.

**Veredicto**: chunks está en 3NF. Las FKs son referencias, no dependencias transitivas dentro de la tabla. Los únicos atributos no-key en `chunks` son `section_id`, `chunk_index`, `text`, `page`, `offset_start`, `offset_end` — todos dependen directamente de `chunk_id`.

**BCNF** ✅ — Único determinante: `chunk_id`.

**⚠️ Redundancia parcial: `document_id` vs `topic_id`**

Un chunk tiene `document_id` Y `topic_id`. Pero `topic` ya tiene `document_id`. 
¿Es esto transitivo? `chunk_id → topic_id → topic.document_id`?

**No**, porque:
- `chunks.document_id` es la relación directa chunk→document.
- `chunks.topic_id` es la relación directa chunk→topic.
- Un chunk puede existir sin topic (NULL topic_id).
- `document_id` no se deriva de `topic_id` — son relaciones independientes.

**Pero hay una constraint implícita**: si un chunk tiene `topic_id`, entonces `topic.document_id` debería coincidir con `chunks.document_id`. Esto es una **integrity constraint de dominio**, no una violación de normalización.

**Recomendación**: Agregar CHECK constraint o validar en la aplicación:

```sql
-- Opción A: CHECK (no se puede hacer en SQLite fácilmente porque REFERENCES topics(document_id))
-- Opción B: Validar en Rust al crear chunks
```

---

### 2.4 Tabla `semantic_links`

```sql
CREATE TABLE semantic_links (
    link_id          TEXT PRIMARY KEY,
    source_chunk_id  TEXT NOT NULL REFERENCES chunks(chunk_id),
    target_chunk_id  TEXT NOT NULL REFERENCES chunks(chunk_id),
    similarity_score REAL NOT NULL,
    link_type        TEXT NOT NULL DEFAULT 'semantic',
    created_at       TEXT NOT NULL DEFAULT (datetime('now'))
);
```

**1NF** ✅ — Atributos atómicos.

**2NF** ✅ — PK simple (`link_id`).

**3NF** ✅ — Sin dependencias transitivas.
- `source_chunk_id`, `target_chunk_id` son FKs.
- `similarity_score` depende del par (source, target), pero se almacena con PK propia.

**BCNF** ✅ — Único determinante: `link_id`.

**⚠️ Decisión: ¿PK natural vs surrogate?**

Alternativa: `PRIMARY KEY(source_chunk_id, target_chunk_id)`.

| Opción | Pros | Contras |
|--------|------|---------|
| `link_id` surrogate | Consistente con otras tablas, permite update sin cambiar PK | Dato extra, UNIQUE constraint needed |
| `(source, target)` natural | Sin surrogate, más directo | PK compuesta, difícil de referenciar |

**Decisión**: Mantener `link_id` surrogate + UNIQUE constraint:

```sql
UNIQUE(source_chunk_id, target_chunk_id)
```

---

### 2.5 Tabla `embeddings` (sin cambios, pero análisis relevante)

```sql
embeddings(chunk_id TEXT PK FK, vector BLOB, model_id TEXT, provider_id TEXT, created_at)
```

**3NF** ✅ — `vector` depende de `chunk_id` + `model_id` + `provider_id` (mismo chunk puede tener embeddings de distintos modelos). Pero PK es solo `chunk_id`.

**⚠️ Observación**: Actualmente PK es `chunk_id`, lo cual asume un solo embedding por chunk. Si en el futuro se soportan múltiples modelos, la PK debería cambiar a `(chunk_id, model_id)`. Esto es fuera de scope para Phase 10 pero vale documentar.

---

### 2.6 Tablas existentes sin cambios

| Tabla | 1NF | 2NF | 3NF | BCNF | Notas |
|-------|-----|-----|-----|------|-------|
| `documents` | ✅ | ✅ | ✅ | ✅ | PK simple, sin transitivas |
| `traces` | ✅ | ✅ | ✅ | ✅ | PK simple |
| `trace_citations` | ✅ | ✅ | ✅ | ✅ | PK compuesta (trace_id, citation_id), no hay dependencias parciales |
| `durable_locks` | ✅ | ✅ | ✅ | ✅ | PK simple |
| `ingest_backlog` | ✅ | ✅ | ✅ | ✅ | PK simple + UNIQUE(idempotency_key) |
| `rate_limit_counters` | ✅ | ✅ | ✅ | ✅ | PK compuesta natural |
| `corpus_snapshots` | ✅ | ✅ | ✅ | ✅ | PK simple |
| `snapshot_members` | ✅ | ✅ | ✅ | ✅ | PK compuesta |
| `snapshot_pointer` | ✅ | ✅ | ✅ | ✅ | Single-row con CHECK |

---

## 3. Decisiones de Diseño

### D1: Nullable FKs en chunks (topic_id, concept_id)

**Elección**: Columnas nullable en `chunks` en vez de join table.

| Opción | Pros | Contras |
|--------|------|---------|
| Nullable FKs en chunks | Simple, 3NF-compliant, JOINs naturales | NULLs en datos legacy |
| Join table `chunk_topics` | Sin NULLs, separación clara | Tabla extra, JOIN adicional |

**Justificación**: 
- Un chunk pertenece a exactamente 0 o 1 topic/concept (relación many-to-one).
- Nullable FK es el patrón estándar para optional relationships.
- Los NULLs se resuelven con re-ingestion (`--rebuild-hierarchy`).
- Join table sería para many-to-many, que no aplica aquí.

### D2: chunk_count denormalizado en topics/concepts

**Elección**: Almacenar `chunk_count` como columna, no computar.

**Justificación**:
- `topics list` y `topics show` son comunes → COUNT(*) frecuente es costoso.
- El count se actualiza al crear/eliminar chunks (trigger o aplicación).
- Riesgo de desincronización es bajo y detectable con `SELECT COUNT(*) vs chunk_count`.

### D3: embedding BLOB en topics/concepts

**Elección**: Almacenar embedding a nivel de topic y concept, no solo en chunks.

**Justificación**:
- Permite búsqueda semántica a nivel topic/concept en Phase 11.
- El vector se puede computar como promedio de embeddings de chunks hijos.
- Es denormalización justificada: evita computar el promedio en cada query.

**Fase de generación**: No se genera en Phase 10. Columna nullable, se llena en Phase 11.

### D4: semantic_links con surrogate PK

**Elección**: `link_id` como PK surrogate + UNIQUE(source, target).

**Justificación**:
- Consistente con el patrón de todas las demás tablas (PK surrogate TEXT).
- Facilita referencias futuras (ej: `trace_citations` podría referenciar links).
- UNIQUE constraint previene duplicados.

### D5: Graph crate ownership

**Elección**: Graph crate dueño de tipos + hierarchy builder. Ingest llama a graph.

```
ingest crate:
  1. Lee archivo
  2. Llama a graph::extract_headings(markdown) → Vec<HeadingSpan>
  3. Llama a graph::build_hierarchy(chunks, headings) → Vec<Topic>
  4. Persiste a storage

graph crate:
  - extract_headings() → parser de markdown headings
  - build_hierarchy() → lógica de agrupación
  - Tipos: Topic, Concept, SemanticLink

storage crate:
  - Migration 006
  - CRUD para topics, concepts, semantic_links
  - Queries jerárquicas
```

---

## 4. Index Strategy

```sql
-- Hierarchy traversal
CREATE INDEX idx_chunks_topic ON chunks(topic_id);
CREATE INDEX idx_chunks_concept ON chunks(concept_id);
CREATE INDEX idx_topics_document ON topics(document_id);
CREATE INDEX idx_concepts_topic ON concepts(topic_id);

-- Semantic links (forward and reverse lookup)
CREATE INDEX idx_semantic_links_source ON semantic_links(source_chunk_id);
CREATE INDEX idx_semantic_links_target ON semantic_links(target_chunk_id);

-- Composite for "all chunks in a topic" query
CREATE INDEX idx_chunks_topic_concept ON chunks(topic_id, concept_id);
```

**Queries optimizadas por estos indexes**:

| Query | Index usado |
|-------|-------------|
| "All chunks in topic X" | `idx_chunks_topic` |
| "All concepts in topic X" | `idx_concepts_topic` |
| "All topics in document X" | `idx_topics_document` |
| "Chunks in concept X ordered by index" | `idx_chunks_topic_concept` |
| "Links from chunk X" | `idx_semantic_links_source` |
| "Links to chunk X" | `idx_semantic_links_target` |

---

## 5. Migration Strategy

### Migration 006 — Safe Additive Changes

```sql
-- 1. Create new tables (no data affected)
CREATE TABLE IF NOT EXISTS topics (...);
CREATE TABLE IF NOT EXISTS concepts (...);
CREATE TABLE IF NOT EXISTS semantic_links (...);

-- 2. Add nullable columns to existing table (no data affected)
ALTER TABLE chunks ADD COLUMN concept_id TEXT REFERENCES concepts(concept_id);
ALTER TABLE chunks ADD COLUMN topic_id TEXT REFERENCES topics(topic_id);

-- 3. Create indexes
CREATE INDEX IF NOT EXISTS idx_chunks_topic ON chunks(topic_id);
CREATE INDEX IF NOT EXISTS idx_chunks_concept ON chunks(concept_id);
CREATE INDEX IF NOT EXISTS idx_topics_document ON topics(document_id);
CREATE INDEX IF NOT EXISTS idx_concepts_topic ON concepts(topic_id);
CREATE INDEX IF NOT EXISTS idx_semantic_links_source ON semantic_links(source_chunk_id);
CREATE INDEX IF NOT EXISTS idx_semantic_links_target ON semantic_links(target_chunk_id);
```

**Propiedades de seguridad**:
- Solo `CREATE TABLE` y `ALTER TABLE ADD COLUMN` — no destructivo.
- FKs nullable — no viola constraints en datos existentes.
- Indexes con `IF NOT EXISTS` — idempotente.
- Transactional (execute_batch) — todo o nada.

**Backward compatibility**:
- Datos existentes: `topic_id = NULL`, `concept_id = NULL`.
- Flat retrieval funciona sin cambios.
- `build_hierarchy = false` (default) ignora las nuevas tablas.

---

## 6. Module Boundaries

| Crate | Archivos nuevos | Responsabilidad |
|-------|----------------|-----------------|
| `storage` | `migrations/006_hierarchy.sql`, `src/topics.rs`, `src/concepts.rs`, `src/semantic_links.rs` | CRUD + queries jerárquicas |
| `graph` | `src/types.rs`, `src/heading_parser.rs`, `src/hierarchy.rs` | Domain types + build logic |
| `ingest` | `src/sentence_chunker.rs`, modificación en pipeline | Chunker + wiring |
| `config` | modificación en config struct | Nuevos campos |
| `common` | (posible) tipos compartidos | Si es necesario para evitar circular deps |

---

## 7. Checklist de Normalización

| Tabla | 1NF | 2NF | 3NF | BCNF | Issues |
|-------|-----|-----|-----|------|--------|
| topics | ✅ | ✅ | ✅ | ✅ | chunk_count denormalizado (justificado) |
| concepts | ✅ | ✅ | ✅ | ✅ | chunk_count denormalizado (justificado) |
| chunks (mod) | ✅ | ✅ | ✅ | ✅ | FK nullable, integrity constraint de dominio |
| semantic_links | ✅ | ✅ | ✅ | ✅ | Surrogate PK + UNIQUE natural |
| embeddings | ✅ | ✅ | ✅ | ⚠️ | PK asume 1 embedding/chunk (out of scope) |

**Veredicto**: El schema propuesto cumple 3NF en todas las tablas. BCNF se cumple en todas excepto `embeddings` (limitación existente fuera de scope). Las denormalizaciones (`chunk_count`, `embedding` en topics/concepts) están justificadas y documentadas.
