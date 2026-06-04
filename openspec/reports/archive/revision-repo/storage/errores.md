# Errores Storage — Pendientes de Fix

> Errores encontrados en la revisión del crate `storage`.
> Este archivo NO se sube a GitHub.

---

## 🔴 CRITICAL

### 1. Foreign keys deshabilitadas — integridad referencial no garantizada

**Archivo:** `src/lib.rs:34-40`
**Problema:** `Database::open()` configura `journal_mode=WAL` y `busy_timeout=5000` pero NO ejecuta `PRAGMA foreign_keys=ON`. En SQLite, las FK constraints están deshabilitadas por defecto. Esto significa que todas las `REFERENCES` en el schema (chunks→documents, embeddings→chunks, concepts→topics, trace_citations→traces, semantic_links→chunks, snapshot_members→snapshots) son decorativas y NO se enforcement.

**Impacto:** Un caller podría insertar chunks para documentos inexistentes, embeddings para chunks fantasma, o citations para traces que no existen. La integridad referencial depende 100% de la lógica de aplicación.

**Fix sugerido:** Agregar `conn.pragma_update(None, "foreign_keys", true)` después de habilitar WAL mode en `Database::open()`.

**Severidad:** CRITICAL — la base de datos no protege su propia consistencia.

### 2. Rate limit key no cumple FR-109

**Archivo:** `engine/src/retrieve.rs:273-275` (caller), `crate/rate_limits.rs` (API genérica)
**Problema:** FR-109 define clave compuesta `(runtime_mode + corpus_id + provider_id + retrieval_scope)` para rate limiting. El caller construye la clave usando solo `provider_id()`. La API de storage es genérica (acepta cualquier `key` string) así que el problema es del caller, pero storage no provee helper ni validación para la clave compuesta.

**Impacto:** Rate limiting es por provider solamente. Dos requests con diferente `corpus_id` o `retrieval_scope` comparten el mismo contador. En un multi-corpus setup, un corpus saturaría el rate limit de otro.

**Fix sugerido:** Agregar un helper en `rate_limits.rs` que construya la clave compuesta, o al menos documentar el requisito en la API. El caller debe pasar `format!("{runtime_mode}:{corpus_id}:{provider_id}:{retrieval_scope}")`.

**Severidad:** CRITICAL — no cumple requisito funcional FR-109.

---

## 🟠 HIGH

### 3. `activate_snapshot` usa `.ok()` en vez de `.optional()` — errores de DB silenciados

**Archivo:** `src/snapshots.rs:68-73`
**Problema:**
```rust
let previous_snapshot_id: Option<String> = tx
    .query_row(
        "SELECT active_snapshot_id FROM snapshot_pointer WHERE id = 1",
        [],
        |row| row.get(0),
    )
    .ok();
```
`.ok()` convierte cualquier `Err` a `None`, incluyendo errores de base de datos genuinos (corruption, I/O errors, etc.). Solo debería convertir `QueryReturnedNoRows` a `None`.

**Impacto:** Si hay un error de DB real durante la lectura del snapshot pointer, se trataría como "no hay snapshot activo" y se saltaría el paso de supersede. El snapshot anterior quedaría en estado `active` indefinidamente.

**Fix sugerido:** Reemplazar `.ok()` con `.optional().map_err(storage_err)?` — igual que se hace en `get_active_snapshot_id()` y `rate_limits.rs`.

**Severidad:** HIGH — puede causar snapshots huérfanos en estado `active` ante errores de I/O.

### 4. Casts `i64 → u32` sin verificación de overflow

**Archivo:** Múltiple — `util.rs:37,42,47`, `embeddings.rs:144,148,153`, `traces.rs:124,128,150`, `documents.rs:72,73,74,75`
**Problema:** Los campos `chunk_index`, `page`, `offset_start`, `offset_end`, `file_size_bytes` se castean de `i64` a `u32` con `as u32` / `as u64`. Si la base de datos contiene valores negativos o > `u32::MAX` (~4.2B), el cast trunca silenciosamente.

**Ejemplo concreto:**
```rust
chunk_index: row.get::<_, i64>("chunk_index").map_err(storage_err)? as u32,
```

**Impacto:** Un `file_size_bytes` de 5GB se almacena como `i64` pero se lee como `u32` truncado. Un `chunk_index` corrupto negativo se convertiría a un valor enorme.

**Fix sugerido:** Usar `u32::try_from(i64_value).map_err(...)` o `i64::to_u32()` con manejo de error. Para `file_size_bytes`, considerar `u64::try_from(i64_value)`.

**Severidad:** HIGH — data corruption silenciosa en edge cases.

---

## 🟡 MEDIUM

### 5. Rate limit counters no tienen TTL — acumulación indefinida

**Archivo:** `src/rate_limits.rs` + `migrations/004_rate_limits.sql`
**Problema:** Los registros en `rate_limit_counters` se insertan con `ON CONFLICT DO UPDATE` pero nunca se eliminan. Cada ventana temporal crea un registro que persiste indefinidamente.

**Impacto:** En un sistema long-running, la tabla crece sin límite. Con 10 providers × 4 routes × 1440 ventanas/día = ~57,600 registros/día.

**Fix sugerido:** Agregar un método `cleanup_expired_rate_limits(window_seconds: u32)` que elimine registros donde `window_start_epoch < now - 2*window_seconds`. Ejecutar periódicamente o al inicio del pipeline.

**Severidad:** MEDIUM — storage leak gradual, no afecta corrección.

### 6. `list_chunk_embeddings_hierarchical` y `list_ready_chunk_embeddings` — duplicación significativa

**Archivo:** `src/embeddings.rs:103-166` vs `src/embeddings.rs:170-210`
**Problema:** `list_ready_chunk_embeddings()` es un subconjunto de `list_chunk_embeddings_hierarchical(None, None)`. La lógica de row mapping está duplicada (~60 líneas). Cualquier cambio en la query o el mapping debe hacerse en dos lugares.

**Impacto:** Mantenibilidad. Si se agrega un campo al JOIN, hay que actualizar ambos métodos.

**Fix sugerido:** Reescribir `list_ready_chunk_embeddings` como wrapper:
```rust
pub fn list_ready_chunk_embeddings(&self) -> Result<Vec<ChunkEmbeddingRecord>, CiteError> {
    self.list_chunk_embeddings_hierarchical(None, None)
        .map(|rows| rows.into_iter().map(|h| h.chunk).collect())
}
```

**Severidad:** MEDIUM — code smell, no afecta funcionalidad.

### 7. `ConceptRow` y `TopicRow` almacenan `created_at` como String

**Archivo:** `src/concepts.rs:14`, `src/topics.rs:14`
**Problema:** `Document` y `Chunk` parsean `created_at` a `DateTime<Utc>` via `parse_dt()`, pero `ConceptRow` y `TopicRow` lo dejan como `String`. Los callers que necesitan comparar timestamps entre entidades tienen que hacer parsing manual.

**Impacto:** Inconsistencia de API. Los consumers de topics/concepts no pueden hacer operaciones de tiempo sin parsear manualmente.

**Fix sugerido:** Agregar campo `created_at: DateTime<Utc>` a ambos row types y parsear con `parse_dt()` en el row mapper, igual que hace `documents.rs`.

**Severidad:** MEDIUM — inconsistencia de diseño, bajo impacto práctico.

### 8. Error silenciado en `decode_vector_blob` — rows corruptos se saltan sin warning

**Archivo:** `src/embeddings.rs:31-37` (decoder), `src/embeddings.rs:122,155` (callers)
**Problema:**
```rust
let Some(vector) = decode_vector_blob(&blob) else {
    continue; // row silently skipped
};
```
Si un BLOB tiene longitud no múltiplo de 4 (corrupción), la row se salta silenciosamente. No hay log ni error.

**Impacto:** Embeddings corruptos se vuelven invisibles. El retrieval podría retornar menos resultados de los esperados sin indicar por qué.

**Fix sugerido:** Retornar `Err` si el BLOB es inválido, o al menos loggear un warning. Alternativamente, agregar un campo `skipped_count` al resultado.

**Severidad:** MEDIUM — debugging dificultado ante corrupción de datos.

### 9. Snapshot pointer sin campo `updated_at`

**Archivo:** `migrations/005_snapshots.sql:18-21`
**Problema:** La tabla `snapshot_pointer` solo tiene `id` y `active_snapshot_id`. No hay timestamp de cuándo se activó el snapshot actual.

**Impacto:** Imposible saber cuándo se hizo el último swap sin consultar `corpus_snapshots.activated_at` por separado.

**Fix sugerido:** Agregar columna `updated_at TEXT NOT NULL` a `snapshot_pointer` y actualizarla en `activate_snapshot`.

**Severidad:** MEDIUM — observabilidad reducida, no afecta corrección.

---

## 🟢 LOW

### 10. `update_document_status` no valida transiciones de estado

**Archivo:** `src/documents.rs:222-245`
**Problema:** Cualquier status puede transicionar a cualquier otro. No hay validación de que `failed → processing` sea inválido o que `ready → pending` no tenga sentido.

**Impacto:** Un bug en el caller podría poner un documento en un estado inválido sin que la DB lo detecte.

**Fix sugerido:** Opcional — agregar un `CHECK` constraint o una función `validate_transition(from, to)`. Es un trade-off entre defensa y flexibilidad.

**Severidad:** LOW — la responsabilidad de transiciones válidas está en el caller.

### 11. Superseded snapshots no se limpian nunca

**Archivo:** `src/snapshots.rs`
**Problema:** Los snapshots en estado `superseded` permanecen en la DB indefinidamente con todos sus `snapshot_members`.

**Impacto:** Acumulación gradual de datos históricos. En un pipeline que hace refresh frecuente, esto crece linealmente.

**Fix sugerido:** Agregar un método `cleanup_superseded_snapshots()` que elimine snapshots y members con estado `superseded` más antiguos que N días.

**Severidad:** LOW — storage leak gradual.

### 12. `semantic_links` UNIQUE constraint impide múltiples tipos de relación entre el mismo par

**Archivo:** `migrations/006_hierarchy.sql:22-23`
**Problema:** `UNIQUE(source_chunk_id, target_chunk_id)` impide tener dos links (ej: "semantic" + "citation") entre los mismos dos chunks.

**Impacto:** Limitación de diseño. Si el dominio necesita múltiples tipos de relación entre el mismo par, el schema no lo permite.

**Fix sugerido:** Si se necesita múltiples tipos, cambiar la UNIQUE a `(source_chunk_id, target_chunk_id, link_type)`. Por ahora es una limitación conocida aceptable.

**Severidad:** LOW — limitación de diseño, no bug.

### 13. `strip_windows_extended_prefix` solo maneja `\\?\`

**Archivo:** `src/backlog.rs:16-18`
**Problema:** Solo strip el prefijo `\\?\`. Los paths UNC (`\\server\share`) u otros prefijos extendidos no se manejan.

**Impacto:** Mínimo — el prefijo `\\?\` es el único que `std::fs::canonicalize()` agrega en Windows normalmente.

**Severidad:** LOW — edge case de plataforma.

---

## ✅ Completados

| # | Descripción | Archivo | Estado |
|---|------------|---------|--------|
| — | — | — | — |
