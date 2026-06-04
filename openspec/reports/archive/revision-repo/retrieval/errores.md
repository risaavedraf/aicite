# Errores Retrieval — Pendientes de Fix

> Errores encontrados en la revisión del crate `retrieval`.
> Este archivo NO se sube a GitHub.

---

## 🔴 CRITICAL

_No se encontraron bugs críticos en el crate. El código es correcto y los 5 tests pasan._

---

## 🟠 HIGH

### 1. ScoredChunk duplica campos de ChunkEmbeddingRecord — riesgo de desalineación

**Archivo:** `crates/retrieval/src/lib.rs:40-64` + `crates/storage/src/embeddings.rs:8-20`
**Problema:** `ScoredChunk` replica 9 campos de `ChunkEmbeddingRecord` (`chunk_id`, `document_id`, `display_name`, `section_id`, `chunk_index`, `text`, `page`, `offset_start`, `offset_end`) en lugar de envolverlo. Si `ChunkEmbeddingRecord` agrega o modifica un campo, `ScoredChunk` y el mapeo en `rank_by_similarity` (líneas 161-172) deben actualizarse manualmente. Actualmente hay un caso de esta desalineación: `ChunkEmbeddingRecord` tiene `vector` que `ScoredChunk` no replica (correcto), pero no hay ningún mecanismo compile-time que garantice la alineación del resto.

**Impacto:** Mantenibilidad. No es un bug hoy, pero cada cambio en `ChunkEmbeddingRecord` requiere un update manual coordinado. El compilador no atrapa campos faltantes porque el mapeo es explícito.

**Fix sugerido:** Envolver `ChunkEmbeddingRecord` en `ScoredChunk`:
```rust
pub struct ScoredChunk {
    pub chunk: ChunkEmbeddingRecord,
    pub score: f32,
    pub topic_id: Option<String>,
    pub topic_name: Option<String>,
    pub concept_id: Option<String>,
    pub concept_name: Option<String>,
}
```
Esto requiere cambios en `engine/src/retrieve.rs` y `engine/src/context.rs` donde se accesan campos de `ScoredChunk`. Alternativa menos disruptiva: implementar `From<ChunkEmbeddingRecord>` para `ScoredChunk` con `score: 0.0` y usarlo como único punto de mapeo.

**Severidad:** 🟠 HIGH — deuda técnica activa, no bug hoy.

---

## 🟡 MEDIUM

### 2. Tests insuficientes para edge cases de cosine_similarity

**Archivo:** `crates/retrieval/src/lib.rs:197-256`
**Problema:** Los tests cubren: vectores idénticos (score=1.0), dimensión inválida, y norma cero. Faltan:
- Vectores opuestos → score ≈ -1.0
- Vectores ortogonales → score ≈ 0.0 (solo hay docstring example, no test)
- Vectores de dimensión 1
- Vectores con valores negativos mixtos

**Fix sugerido:** Agregar tests para los casos faltantes:
```rust
#[test]
fn test_cosine_similarity_opposite_vectors() {
    let a = vec![1.0, 0.0];
    let b = vec![-1.0, 0.0];
    let score = cosine_similarity(&a, &b).unwrap();
    assert!((score - (-1.0)).abs() < 1e-6);
}

#[test]
fn test_cosine_similarity_orthogonal_vectors() {
    let a = vec![1.0, 0.0];
    let b = vec![0.0, 1.0];
    let score = cosine_similarity(&a, &b).unwrap();
    assert!(score.abs() < 1e-6);
}
```

**Severidad:** 🟡 MEDIUM — tests existentes son correctos pero no cubren el rango completo de la función.

### 3. Tests insuficientes para edge cases de rank_by_similarity

**Archivo:** `crates/retrieval/src/lib.rs:230-256`
**Problema:** Faltan tests para:
- `k > candidates.len()` (debe devolver todos los candidatos válidos)
- `candidates` vacío (debe devolver `Vec` vacío)
- Todos los candidatos con dimensión inválida (debe devolver `Vec` vacío)
- `k = 0` (debe devolver `Vec` vacío, `truncate(0)` lo maneja)

**Fix sugerido:** Agregar:
```rust
#[test]
fn test_rank_k_larger_than_candidates() {
    let query = vec![1.0, 0.0];
    let candidates = vec![candidate("a", vec![1.0, 0.0], "only")];
    let ranked = rank_by_similarity(&query, &candidates, 10);
    assert_eq!(ranked.len(), 1);
}

#[test]
fn test_rank_empty_candidates() {
    let query = vec![1.0, 0.0];
    let ranked = rank_by_similarity(&query, &[], 5);
    assert!(ranked.is_empty());
}

#[test]
fn test_rank_all_invalid_candidates() {
    let query = vec![1.0, 0.0];
    let candidates = vec![
        candidate("bad1", vec![1.0, 0.0, 0.0], "dim mismatch"),
        candidate("bad2", vec![0.0, 0.0], "zero norm"),
    ];
    let ranked = rank_by_similarity(&query, &candidates, 10);
    assert!(ranked.is_empty());
}
```

**Severidad:** 🟡 MEDIUM — cobertura de tests incompleta para edge cases.

---

## 🟢 LOW

### 4. Salteo silencioso de candidatos inválidos puede ocultar problemas de datos

**Archivo:** `crates/retrieval/src/lib.rs:157-160`
**Problema:** `rank_by_similarity` usa `filter_map` + `?` sobre `cosine_similarity`, lo que descarta silenciosamente candidatos con dimensión incorrecta o norma cero. Si todos los embeddings de un corpus tienen dimensión incorrecta (ej: modelo de embedding cambió), la función devuelve un `Vec` vacío sin ninguna señal de que algo está mal.

**Impacto:** Debugging. El caller (engine) no tiene forma de distinguir "no hay resultados relevantes" de "todos los candidatos fueron descartados por datos corruptos".

**Fix sugerido (opcional):** Log `tracing::warn` cuando se descarta un candidato, o devolver metadata adicional. Alternativa: documentar el comportamiento esperado para que el caller haga su propio check de consistencia de dimensiones.

**Severidad:** 🟢 LOW — comportamiento documentado, pero mejorable.

### 5. Sin clamp explícito del rango de cosine_similarity

**Archivo:** `crates/retrieval/src/lib.rs:119`
**Problema:** El docstring promete `[-1.0, 1.0]` pero no hay clamp explícito. Con aritmética exacta el resultado siempre estaría en rango, pero el cast de `f64` a `f32` en la línea 119 podría producir valores como `1.0000001` en casos extremos (vectores de alta dimensión con valores muy pequeños).

**Impacto:** Mínimo. En la práctica con embeddings de 384-1536 dimensiones de modelos estándar, esto no ocurre. Los callers (engine) comparan contra thresholds que están lejos de los extremos.

**Fix sugerido (opcional):** `Some((dot / (norm_a.sqrt() * norm_b.sqrt())).clamp(-1.0, 1.0) as f32)`

**Severidad:** 🟢 LOW — edge case teórico, no observable en producción.

### 6. Cargo.toml no declara feature flags ni metadata de propósito

**Archivo:** `crates/retrieval/Cargo.toml`
**Problema:** El Cargo.toml es minimal — solo tiene nombre, versión, edición y 2 deps. No tiene `description`, `categories`, ni feature flags. Esto es consistente con otros crates del workspace, así que no es una anomalía del crate retrieval en particular.

**Severidad:** 🟢 LOW — consistente con el proyecto, no requiere acción individual.

---

## ✅ Completados

| # | Archivo | Fix | Fecha |
|---|---------|-----|-------|
| — | — | — | — |
