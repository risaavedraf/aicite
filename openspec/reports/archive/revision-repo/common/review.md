# Revisión: `crates/common` — Tipos Compartidos

## Resumen del Crate

**Propósito:** Centralizar tipos de dominio, errores y códigos de salida compartidos por todo el ecosistema `aiharness`. Es el crate base que todos los demás (`cli`, `engine`, `storage`, `ingest`, `providers`, `config`) importan para tipos, errores y constantes comunes.

**Estructura:**
```
crates/common/
├── Cargo.toml
└── src/
    ├── lib.rs      — re-exports públicos
    ├── error.rs    — CiteError (enum de errores con thiserror)
    ├── exit.rs     — ExitCode (códigos de salida CLI)
    └── types.rs    — tipos de dominio (Document, Chunk, Citation, Trace, newtypes, enums)
```

**Dependencias:**
- `serde` + `serde_json`: serialización para respuestas JSON y persistencia
- `thiserror`: derivación de `Display` y `Error` para `CiteError`
- `chrono`: timestamps en tipos de dominio (`DateTime<Utc>`)

**No tiene dependencias internas** — es el nodo raíz del grafo de dependencias del workspace.

## Módulos y Archivos

### `src/lib.rs`
Punto de entrada del crate. Exporta públicamente los tres submódulos (`error`, `exit`, `types`) y re-exporta en el namespace raíz los tipos más usados:
- `CiteError` de `error`
- `ExitCode` de `exit`
- 16 tipos de `types` (Document, Chunk, Citation, etc.)

**Observación:** No re-exporta `DocumentId`, `ChunkId`, `TraceId`, `ErrorInfo`, `OffsetRange`, `FixtureResult`, ni `EvalReport`. Los consumidores acceden a ellos vía `common::types::*` directamente, lo cual funciona pero es inconsistente con los tipos que sí están re-exportados.

### `src/error.rs` — `CiteError`

Enum de 18 variantes que cubre todos los dominios de error del sistema:

| Dominio | Variantes |
|---------|-----------|
| Archivos | `UnsupportedFileType`, `FileTooLarge`, `FileNotFound` |
| Entidades | `DocumentNotFound`, `DocumentNotReady`, `TraceNotFound`, `CitationNotFound`, `ChunkNotFound` |
| Configuración | `ConfigError` |
| Storage | `StorageError` |
| Interno | `InternalError` |
| Validación | `QueryTooLong`, `InvalidParameter`, `PathRejected` |
| Runtime | `RuntimeModeForbidden` |
| Concurrencia | `RateLimitExceeded`, `OperationInProgress` |
| Providers | `EmbeddingProviderError`, `RetrievalTimeout` |

**Funcionalidad clave:**
- `code()` → string machine-readable (`"document_not_found"`)
- `exit_code()` → `ExitCode` enum para CLI
- `to_json_response()` → `ErrorResponse` serializable para respuestas HTTP/JSON
- `message()` → wrapper sobre `Display` (thiserror genera el impl)

**Test:** Un único test que verifica que `OperationInProgress` serializa correctamente sus campos `retry_after_seconds` y `lock_name` en el JSON response.

### `src/exit.rs` — `ExitCode`

Enum con 8 variantes `#[repr(i32)]` que mapean códigos de salida CLI:

| Código | Valor | Uso |
|--------|-------|-----|
| `Success` | 0 | OK (incluye context, no_results, insufficient_context) |
| `Validation` | 1 | Error de validación/config/contrato |
| `NotFound` | 2 | Entidad no encontrada o no lista |
| `Provider` | 3 | Fallo de dependencia externa |
| `RuntimeForbidden` | 4 | Modo runtime prohibido |
| `Internal` | 5 | Error interno |
| `OperationInProgress` | 6 | Lock conflict |
| `RateLimitExceeded` | 7 | Rate limit |

Deriva `Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize`. Correcto y completo para su uso.

### `src/types.rs` — Tipos de Dominio

Archivo principal (~640 líneas). Contiene:

#### Newtypes (identificadores fuertemente tipados)
- `DocumentId(String)`, `ChunkId(String)`, `TraceId(String)`
- Cada uno deriva `Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize`
- Implementan `Display`, `From<String>`, `AsRef<str>`
- Documentación con ejemplos de uso

#### Enums
- `DocumentStatus`: `Pending | Processing | Ready | Failed` — con `serde(rename_all = "snake_case")` y `Display`
- `FileType`: `Pdf | Txt | Md` — con `from_extension()` que reconoce "pdf", "txt", "md", "markdown"
- `ResultKind`: `Context | NoResults | InsufficientContext` — clasificación de resultados de retrieval

#### Structs de dominio
- `ErrorInfo`: código + mensaje para documentos fallidos
- `Document`: metadatos completos de un documento ingestado (13 campos, incluyendo retry logic)
- `Chunk`: texto extraído de un documento (10 campos, con offsets y página)
- `Citation`: referencia citada en resultados de retrieval (12 campos, con score y breadcrumb)
- `OffsetRange`: rango de caracteres `[start, end)`
- `ContextMetadata`: metadatos del context pack (15 campos)
- `ContextResponse`: respuesta top-level del context pack
- `ReadSelector`: enum para comandos `read` (por citation o por chunk)
- `ReadResponse`: payload de respuesta del comando `read`
- `TraceResponse`: respuesta del comando `trace`
- `TraceHeaderInput` / `TraceHeaderRecord`: input y output de persistencia de trace headers
- `TraceCitationRecord`: cita denormalizada para el trace store
- `ContextMetadataScaffold`: scaffold mínimo de metadatos de contexto
- `TraceEnvelope`: envoltura completa de trace (header + citations + context metadata)
- `FixtureResult` / `EvalReport`: tipos para el harness de evaluación golden

## Flujo de Datos

```
                    ┌─────────────┐
                    │   common    │  (tipos, errores, exit codes)
                    └──────┬──────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
   ┌────▼────┐      ┌─────▼─────┐     ┌──────▼──────┐
   │  cli    │      │  engine   │     │  providers  │
   │(output, │      │(context,  │     │(embeddings) │
   │ exit    │      │ retrieve, │     └──────┬──────┘
   │ codes)  │      │ ingest,   │            │
   └─────────┘      │ evaluate) │            │
                    └─────┬─────┘            │
                          │                  │
                    ┌─────▼─────┐      ┌─────▼─────┐
                    │  storage  │      │  ingest   │
                    │(SQLite    │      │(chunker,  │
                    │ persist)  │      │ extractor)│
                    └───────────┘      └───────────┘
```

**Flujo típico de un documento:**
1. `cli` recibe comando → crea `Document` con `DocumentStatus::Pending`
2. `ingest` parsea archivo → valida con `FileType`, crea `Vec<Chunk>`
3. `engine` orquesta → pasa `Document` + chunks a `storage` para persistir
4. `providers` genera embeddings → devuelve `Vec<f32>` (usa `CiteError` para errores)
5. `engine` hace retrieval → construye `Citation`, `ContextResponse`, `TraceResponse`
6. `cli` serializa output → usa `ExitCode` para el código de retorno

## Decisiones de Diseño

### ✅ Aciertos

1. **Newtypes para IDs fuertemente tipados**: `DocumentId`, `ChunkId`, `TraceId` previenen mezclar IDs accidentalmente. Patrón idiomático correcto con `Display`, `From<String>`, `AsRef<str>`, `Hash`.

2. **`thiserror` para CiteError**: Evita boilerplate masivo en la implementación de `Display` y `Error`. Los `#[error("...")]` son legibles y maintainbles.

3. **Mapeo error→exit code centralizado**: `code_and_exit()` garantiza consistencia entre el código machine-readable y el exit code CLI. Un solo punto de verdad.

4. **`to_json_response()` en el propio error**: Los consumidores no necesitan lógica de serialización de errores — llaman al método y listo. Los campos `details` extensibles con `serde_json::Value` son pragmáticos.

5. **`serde(rename_all = "snake_case")` en enums**: Garantiza que la serialización JSON sea consistente y predecible (e.g., `"pending"`, no `"Pending"`).

6. **`ContextMetadataScaffold` separado de `ContextMetadata`**: Permite que el trace store persista solo los campos que necesita sin cargar el metadata completo.

7. **`ReadSelector` como enum**: Modela correctamente dos modos mutuamente excluyentes de lectura. No derive Serialize/Deserialize porque es un tipo de dominio puro, no de persistencia.

### ⚠️ Tradeoffs y Observaciones

1. **`Document` es flat (13 campos)**: El doc comment explica que anidar structs como `DocumentIdentity` rompería todos los sitios de construcción. Es un tradeoff pragmático, pero a medida que el proyecto madure, la extracción de sub-structs será necesaria.

2. **`ErrorInfo` no deriva `PartialEq`**: `Document` tampoco. Esto impide asserts de igualdad directas en tests (`assert_eq!(doc1, doc2)` no compila). `ErrorInfo` se usa dentro de `Document`, así que el problema se propaga.

3. **IDs como `String` dentro de structs de dominio**: `Document`, `Chunk`, `Citation` usan `String` para IDs en vez de los newtypes `DocumentId`, `ChunkId`, `TraceId`. Esto anula parcialmente el beneficio de los newtypes — solo funcionan si se usan consistentemente.

4. **Sin `#[non_exhaustive]` en enums públicos**: `FileType`, `DocumentStatus`, `ResultKind`, `ExitCode` no tienen `#[non_exhaustive]`. Agregar una variante rompe compilación en crates dependientes. Para un workspace monorepo es aceptable, pero limita la extensibilidad futura.

5. **`ExitCode` no implementa `Into<i32>` o `process::exit()` helper**: Los consumidores necesitan hacer `process::exit(code as i32)` manualmente. Un método `as_i32()` o `fn exit(self) -> !` sería más ergonómico.

## Conexiones con Otros Crates

| Crate | Qué importa de `common` |
|-------|--------------------------|
| **cli** | `CiteError`, `ExitCode`, `ReadSelector`, `ResultKind`, `ContextResponse`, `TraceHeaderInput` |
| **engine** | `CiteError`, `Document`, `Chunk`, `Citation`, `ContextResponse`, `ReadResponse`, `ReadSelector`, `ResultKind`, `OffsetRange`, `ErrorInfo`, `TraceResponse` |
| **storage** | `CiteError`, `Document`, `DocumentStatus`, `ErrorInfo`, `FileType`, `Chunk`, `TraceHeaderInput`, `TraceHeaderRecord`, `TraceCitationRecord`, `TraceEnvelope`, `ContextMetadataScaffold` |
| **ingest** | `CiteError`, `FileType`, `Chunk`, `Document`, `DocumentStatus` |
| **providers** | `CiteError` (exclusivamente — trait `EmbeddingProvider` define `fn embed() -> Result<Vec<f32>, CiteError>`) |
| **config** | `CiteError` |

**`CiteError` es el tipo más usado del crate** — importado por 6 de 6 crates dependientes.
