# Revisión: `crates/storage` — Capa de persistencia SQLite para el harness

**Revisor:** el Gentleman subagent de revisión
**Fecha:** 2026-06-02
**Alcance:** 14 archivos Rust + 7 archivos SQL en crates/storage/src/
**Metodología:** Inspección manual de todos los archivos fuente, schema SQL, y dependencias cross-crate

---

## Resumen del Crate

El crate `storage` es la capa de persistencia central de aiharness. Proporciona un único handle `Database` que envuelve una conexión `rusqlite::Connection` a SQLite y expone toda la lógica de lectura/escritura como métodos sobre esa struct.

**Propósito:** Almacenar y recuperar el estado completo del pipeline de ingest, retrieval y observability — documentos, chunks, embeddings, topics, concepts, semantic links, traces con citations, snapshots corpus, rate limiting, locks durables, y backlog de ingest.

**Dependencias externas:** Solo `rusqlite`, `chrono`, y el crate `common` (tipos compartidos).

**Estructura:** 13 módulos públicos + 1 módulo privado (`util`). Las migraciones SQL están embebidas via `include_str!`. Todas las operaciones son métodos sobre `Database`, no funciones libres — el patrón es "repository method" sobre un handle compartido.

## Flujo Principal

### 1. Ingest Pipeline (Document → Chunks → Embeddings → Snapshot)

```
Source file
  → backlog.ingest_backlog (upsert with idempotency key)
  → documents.insert_document (status=pending)
  → documents.update_document_status (status=processing)
  → chunks.insert_chunks (bulk, transactional)
  → embeddings.insert_embeddings (bulk, transactional)
  → topics.insert_concepts / concepts.insert_concepts (hierarchy)
  → chunks.set_chunk_hierarchy (topic/concept assignment)
  → documents.update_document_status (status=ready)
  → snapshots.begin_snapshot_build → attach_document_to_snapshot → activate_snapshot
```

### 2. Retrieval Pipeline

```
Query arrives
  → rate_limits.check_and_increment_rate_limit (gate)
  → embeddings.list_chunk_embeddings_hierarchical (or list_ready_chunk_embeddings)
  → [external cosine similarity computation]
  → traces.persist_trace_with_citations (observability)
```

### 3. Recovery Pipeline

```
Startup
  → locks.is_lock_held("ingest_pipeline")
  → if not held: documents.recover_processing_documents_if_lock_free
  → backlog.claim_next_ingest_backlog (FIFO)
```

### 4. Snapshot Atomic Swap

```
Build phase:
  → snapshots.begin_snapshot_build (state=building)
  → snapshots.attach_document_to_snapshot (N documents)
  → snapshots.activate_snapshot (transactional):
      1. Verify building state
      2. Read current active pointer
      3. Supersede previous snapshot
      4. Set new snapshot active
      5. Upsert active pointer
  → Result: zero-downtime corpus swap
```

## Módulos/Archivos Clave

| Archivo | Responsabilidad | Tipos clave |
|---------|----------------|-------------|
| `lib.rs` | Handle `Database`, open/health/WAL config, migrations dispatch | `Database` |
| `util.rs` | Helpers internos: error conversion, datetime formatting, row→Chunk | `storage_err()`, `format_dt()`, `parse_dt()`, `row_to_chunk()` |
| `documents.rs` | CRUD documentos, status transitions, retry management, crash recovery | `insert_document`, `update_document_status`, `recover_processing_documents_if_lock_free` |
| `chunks.rs` | Bulk insert/delete chunks, hierarchy assignment | `insert_chunks`, `set_chunk_hierarchy`, `delete_chunks_for_document` |
| `embeddings.rs` | Bulk insert embeddings (BLOB f32 LE), hierarchical/flat queries | `ChunkEmbeddingRecord`, `HierarchicalChunkEmbedding`, `list_chunk_embeddings_hierarchical` |
| `concepts.rs` | CRUD concepts, chunk_count recalculation | `ConceptRow`, `insert_concept`, `update_concept_chunk_count` |
| `topics.rs` | CRUD topics, chunk_count recalculation | `TopicRow`, `insert_topic`, `update_topic_chunk_count` |
| `semantic_links.rs` | Cross-chunk semantic relationships | `SemanticLinkRow`, `insert_semantic_link`, `get_links_from/to` |
| `traces.rs` | Persist trace headers + citations (transactional), trace envelope retrieval | `persist_trace_with_citations`, `get_trace_envelope`, `get_ready_chunk_by_document` |
| `snapshots.rs` | Atomic snapshot lifecycle: build → activate → supersede | `ActivateSnapshotResult`, `activate_snapshot`, `get_active_snapshot_member_ids` |
| `locks.rs` | Named durable locks for distributed coordination | `try_acquire_lock`, `release_lock`, `is_lock_held` |
| `rate_limits.rs` | Fixed-window rate limiting with composite key | `RateLimitDecision`, `check_and_increment_rate_limit` |
| `backlog.rs` | Durable ingest queue with idempotency, FIFO claim | `IngestBacklogItem`, `upsert_ingest_backlog`, `claim_next_ingest_backlog` |
| `migrations/mod.rs` | Sequential integer-versioned migration runner (7 migrations) | `run()`, `_migrations` tracking table |

## Decisiones de Diseño

### ✅ Aciertos

1. **WAL + busy_timeout:** `journal_mode=WAL` y `busy_timeout=5000` son las pragmas correctas para concurrent reads con serialized writes en SQLite.

2. **Snapshot staging + atomic activate:** El patrón `building → activate` con transacción que supersedes el anterior y activa el nuevo es exactamente lo que requiere FR-106. No es un rebuild in-place — es un swap atómico sin mixed visibility.

3. **Idempotent backlog:** `upsert_ingest_backlog` con `ON CONFLICT(idempotency_key) DO UPDATE` garantiza que re-ingest del mismo archivo no duplica trabajo. La clave de idempotencia normaliza paths (canonical + strip `\\?\` + `/` normalization).

4. **Lock-aware recovery:** `recover_processing_documents_if_lock_free` usa un subquery `NOT EXISTS` para verificar el lock dentro del mismo statement SQL — atómico sin necesidad de transacción explícita.

5. **Bulk inserts transactionales:** `insert_chunks` e `insert_embeddings` envuelven el batch completo en una transacción — rollback total si un chunk falla.

6. **Vector BLOB encoding:** `f32::to_le_bytes()` es el encoding estándar y portable para embeddings. El decode con `chunks_exact(4)` es correcto y eficiente.

7. **Migrations embebidas:** `include_str!` evita problemas de path en runtime y garantiza que las migraciones están compiladas en el binario.

### ⚠️ Trade-offs observados

1. **Single struct pattern:** Todas las operaciones son `impl Database`. Esto es simple pero hace que `Database` sea un "god object" con ~50 métodos públicos. No hay traits de dominio ni separación por aggregate. Aceptable para un crate de storage puro, pero limita testeo con mocks.

2. **String-based enums en DB:** `DocumentStatus`, `FileType`, `RateLimitDecision` se serializan como strings. Esto es legible pero no tiene constraint CHECK en la tabla (excepto `corpus_snapshots.state`). Cualquier typo en el código insertaría un valor inválido.

3. **No `PRAGMA foreign_keys=ON`:** Las FK constraints en el schema SQL son decorativas sin esta pragma. Toda la integridad referencial depende de la lógica de aplicación. Ver sección de errores.

4. **`row_to_chunk` en `util.rs` vs row types en módulos:** Los módulos `topics.rs` y `concepts.rs` definen sus propios row types (`TopicRow`, `ConceptRow`) con `created_at: String`, mientras que `Document` y `Chunk` parsean `created_at` a `DateTime<Utc>`. Inconsistencia menor.

5. **Queries hardcoded con positional indexing:** Los queries usan índices numéricos (`row.get(0)`, `row.get(9)`, etc.) en lugar de nombres de columna. Frágil ante cambios de schema pero presente en todo el crate de forma consistente.

## Conexiones con Otros Crates

| Crate consumidor | Qué usa de storage |
|-----------------|-------------------|
| `engine` | `Database` directamente para ingest, retrieve, evaluate, context, recovery, refresh |
| `cli` | `Database` para comandos de usuario (health, trace, evaluate) |
| `ingest` | `Database` para el pipeline de ingest |
| `retrieval` | `ChunkEmbeddingRecord` para la búsqueda vectorial |
| `config` | Define `RateLimitConfig` que se pasa a `check_and_increment_rate_limit` |

**Patrón de uso:** Los crates consumidores reciben `&Database` como dependency injection. No hay traits abstractos — el coupling es directo con el tipo concreto. Esto es pragmático para un proyecto monolítico pero impediría cambiar el backend de storage sin tocar todos los consumidores.

**Flujo de tipos:** `common::types` → `storage` (persist) → `engine`/`retrieval` (consume). Los tipos de dominio viven en `common`, storage los serializa/deserializa, y los motores los operan.

## Observaciones sobre FR-109 (Rate Limiting)

La especificación FR-109 requiere clave compuesta `(runtime_mode + corpus_id + provider_id + retrieval_scope)` con 20 req/min. El módulo `rate_limits.rs` provee una API genérica `(route, key, max_requests, window_seconds)`. El caller en `engine/src/retrieve.rs` construye la clave usando solo `provider_id()`:

```rust
fn rate_limit_key(provider: &dyn EmbeddingProvider) -> String {
    provider.provider_id().to_string()
}
```

**Resultado:** La clave compuesta completa no se construye. El rate limit es por provider, no por la tupla completa. Los defaults (20 req/60s) coinciden con FR-109 pero la granularidad de la clave es menor a la requerida.

## Observaciones sobre Tests

El crate tiene cobertura de tests robusta: cada módulo tiene tests unitarios que usan `Database::open_memory()`. Los tests cubren:
- CRUD básico y edge cases (not found, duplicates)
- Transacciones y rollback (duplicate chunk_id, duplicate embedding)
- Idempotencia (backlog upsert)
- Atomicidad (snapshot activate)
- Lock semantics (owner verification)
- Rate limit window rollover
- Path normalization en backlog
- Hierarchical embedding queries con filtros

**Gap:** No hay tests de concurrencia real (dos threads/proceses accediendo simultáneamente). Los tests de locks y rate limits verifican lógica single-connection.
