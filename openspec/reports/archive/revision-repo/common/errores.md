# Errores Common — Pendientes de Fix

## 🔴 CRITICAL

### Los newtypes `DocumentId`, `ChunkId`, `TraceId` están definidos pero nunca usados fuera de `common`

**Archivos:**
- `crates/common/src/types.rs:32` (`DocumentId`)
- `crates/common/src/types.rs:66` (`ChunkId`)
- `crates/common/src/types.rs:101` (`TraceId`)

**Problema:** Los tres newtypes están definidos con impls completos (`Display`, `From<String>`, `AsRef<str>`, `Hash`) pero:
1. **No están re-exportados** desde `lib.rs` — la lista `pub use types::{...}` los omite.
2. **Ningún otro crate los importa** — todos usan `String` directamente para IDs.

Ejemplo: `Document.document_id` es `String` (línea 247), no `DocumentId`. Lo mismo para `Chunk.chunk_id` (línea 303), `Citation.document_id` (línea 353), etc.

**Impacto:** Dead code real. Los newtypes no aportan nada si no se usan en las structs de dominio. El beneficio de type-safety en tiempo de compilación se pierde completamente.

**Fix sugerido (2 pasos):**

Paso 1 — Re-exportar desde `lib.rs`:
```rust
pub use types::{
    Chunk, ChunkId, Citation, ContextMetadata, ContextMetadataScaffold, ContextResponse,
    Document, DocumentId, DocumentStatus, FileType, ReadResponse, ReadSelector, ResultKind,
    TraceCitationRecord, TraceEnvelope, TraceHeaderInput, TraceHeaderRecord, TraceId,
    TraceResponse,
};
```

Paso 2 — Migrar campos de `String` a newtypes en las structs (requiere cambios en todos los crates):
```rust
pub struct Document {
    pub document_id: DocumentId,  // era String
    // ...
}

pub struct Chunk {
    pub chunk_id: ChunkId,        // era String
    pub document_id: DocumentId,  // era String
    // ...
}
```

**Nota:** El paso 2 es un cambio de alto impacto que toca todos los crates. Alternativa pragmática: eliminar los newtypes si no se planea migrar, para evitar confusiones.

---

## 🟠 HIGH

### `CiteError` no deriva `PartialEq`

**Archivo:** `crates/common/src/error.rs:7`

**Problema:** `CiteError` solo deriva `Debug` y `thiserror::Error`. Sin `PartialEq`, no se puede hacer:
```rust
assert_eq!(result.unwrap_err(), CiteError::DocumentNotFound { document_id: "x".into() });
```

Los tests en otros crates usan `assert!(matches!(...))` como workaround, que es menos expresivo y no verifica valores.

**Fix sugerido:**
```rust
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum CiteError {
    // ...
}
```

**Nota:** Todas las variantes de `CiteError` contienen tipos que ya implementan `PartialEq` (`String`, `PathBuf`, `u32`, `usize`, `Option<String>`), así que la derivación funciona sin cambios adicionales.

---

### `Document` y `ErrorInfo` no derivan `PartialEq`

**Archivos:**
- `crates/common/src/types.rs:213` (`ErrorInfo`)
- `crates/common/src/types.rs:239` (`Document`)

**Problema:** Imposibilidad de comparar documentos y errores de forma directa en tests. `ErrorInfo` se usa dentro de `Document.error`, amplificando el problema.

**Fix sugerido:**
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorInfo { /* ... */ }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document { /* ... */ }
```

**Nota:** `Document` contiene `DateTime<Utc>`, `PathBuf`, `FileType`, `DocumentStatus` — todos implementan `PartialEq`. `Option<ErrorInfo>` requiere que `ErrorInfo` también lo derive. Es una cadena que se resuelve derivando ambos.

---

## 🟡 MEDIUM

### Re-exports incompletos en `lib.rs` — tipos usados externamente no re-exportados

**Archivo:** `crates/common/src/lib.rs:8-17`

**Problema:** Los siguientes tipos se importan en otros crates vía `common::types::*` pero no están re-exportados en el namespace raíz:
- `ErrorInfo` — usado por `storage`, `engine`
- `OffsetRange` — usado por `engine`
- `FixtureResult`, `EvalReport` — usados por `engine`
- `DocumentId`, `ChunkId`, `TraceId` — definidos pero no usados (ver 🔴 CRITICAL)

Los consumidores pueden acceder vía `common::types::ErrorInfo` directamente, pero la convención del crate es re-exportar en raíz (`common::CiteError`, `common::ExitCode`, etc.). La inconsistencia genera confusión sobre qué es API pública estable.

**Fix sugerido:**
```rust
pub use types::{
    Chunk, ChunkId, Citation, ContextMetadata, ContextMetadataScaffold, ContextResponse,
    Document, DocumentId, DocumentStatus, ErrorInfo, EvalReport, FileType, FixtureResult,
    OffsetRange, ReadResponse, ReadSelector, ResultKind, TraceCitationRecord, TraceEnvelope,
    TraceHeaderInput, TraceHeaderRecord, TraceId, TraceResponse,
};
```

---

### `ExitCode` no tiene helper de conversión a `i32`

**Archivo:** `crates/common/src/exit.rs`

**Problema:** `ExitCode` usa `#[repr(i32)]` pero no expone un método para obtener el valor numérico. Los consumidores en `cli` hacen `process::exit(code as i32)` implícitamente. Un helper mejora la ergonomía y documenta la intención.

**Fix sugerido:**
```rust
impl ExitCode {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}
```

---

## 🟢 LOW

### Sin `#[non_exhaustive]` en enums públicos

**Archivos:**
- `crates/common/src/exit.rs:6` (`ExitCode`)
- `crates/common/src/types.rs` (`FileType`, `DocumentStatus`, `ResultKind`)

**Problema:** Agregar una variante a cualquier enum público rompe compilación en todos los crates dependientes. En un workspace monorepo esto es aceptable porque los cambios se propagan atómicamente, pero impide publicar `common` como crate independiente sin romper API.

**Fix sugerido (solo si se planea publicación externa):**
```rust
#[non_exhaustive]
pub enum FileType { Pdf, Txt, Md }
```

---

### Test insuficiente en `CiteError`

**Archivo:** `crates/common/src/error.rs:160-177`

**Problema:** Solo un test para una variante (`OperationInProgress`). Las otras 17 variantes no tienen coverage. Especialmente importante verificar:
- `RateLimitExceeded` serializa `retry_after_seconds`
- Los `code()` strings son correctos y estables (cambio accidental rompe API)
- Los `exit_code()` mapeos son correctos

**Fix sugerido:** Agregar test que verifique estabilidad de códigos:
```rust
#[test]
fn test_error_codes_are_stable() {
    // Snapshot test — si estos cambian, es breaking change
    assert_eq!(CiteError::DocumentNotFound { document_id: "x".into() }.code(), "document_not_found");
    assert_eq!(CiteError::StorageError { message: "x".into() }.code(), "storage_error");
    // ... etc
}
```

---

## ✅ Completados

| # | Archivo | Línea | Error | Fix | Estado |
|---|---------|-------|-------|-----|--------|
| — | — | — | — | — | — |
