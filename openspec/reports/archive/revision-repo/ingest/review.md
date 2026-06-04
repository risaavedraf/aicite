# Revisión: `crates/ingest` — Pipeline de Extracción, Chunking y Jerarquía

**Revisor:** el Gentleman subagent de revisión
**Fecha:** 2026-06-02
**Alcance:** 6 archivos en crates/ingest/src/ + tests/
**Metodología:** Inspección manual de todos los archivos fuente, ejecución de tests (57/57 passing), trazado de flujo de datos end-to-end

---

## Resumen del Crate

El crate `ingest` es el pipeline central de ingestión de documentos de aiharness. Responsabilidades:

1. **Validación** (`validator.rs`): seguridad de paths, existencia, tipo de archivo, tamaño máximo.
2. **Extracción** (`extractor.rs`): lectura de texto desde TXT, MD y PDF (vía `lopdf`).
3. **Chunking** (`chunker.rs`): división de texto en fragmentos con overlap y tracking de página/offset.
4. **Chunking por oraciones** (`sentence_chunker.rs`): alternativa al chunking fijo, split en boundaries de oración con merge de chunks cortos.
5. **Orquestación** (`lib.rs`): función `ingest_document` que coordina todo el flujo y opcionalmente construye jerarquía topic/concept desde headings markdown.
6. **Jerarquía** (dependencia externa `graph`): parsing de headings H2/H3 y asignación de chunks a topics/concepts.

El crate expone 4 módulos públicos y un punto de entrada principal (`ingest_document`). Depende de `common`, `config`, `graph`, `storage`, `lopdf`, `uuid` y `chrono`.

---

## Flujo Principal

```
Archivo en disco
    │
    ▼
validator::validate_file(path, max_size)
    │  → path safety (traversal, UNC, device)
    │  → canonicalize + re-check
    │  → extension → FileType
    │  → size check
    ▼
extractor::extract_text(path, file_type)
    │  → TXT/MD: read_to_string → single PageText
    │  → PDF: lopdf → Vec<PageText> per page
    ▼
[if sentence_chunking]
sentence_chunker::chunk_by_sentence(text, min_chars)
    │  → split on . ! ? boundaries
    │  → merge short sentences < min_chars
    ▼
[else]
chunker::chunk_text(pages, size, overlap, min_size)
    │  → combine pages → char-offset mapping
    │  → chunk loop with sentence boundary seeking
    │  → overlap stepping
    ▼
Vec<ChunkInput>
    │
    ▼
lib.rs: convert to Chunk (UUID, timestamps)
    │  → db.insert_chunks()
    ▼
[if build_hierarchy && file_type == "md"/"markdown"]
graph::extract_headings(text)
graph::build_hierarchy(document_id, headings, chunk_offsets)
    │  → H2 = Topic, H3 = Concept
    │  → assign chunks by offset boundaries
    ▼
db.insert_topic / db.insert_concept / db.set_chunk_hierarchy
    │
    ▼
Vec<String> (chunk IDs)
```

---

## Módulos/Archivos Clave

### `src/validator.rs` (126 LOC)

- **`validate_file(path, max_size)`** → `Result<(FileType, u64), CiteError>`
  - Security-hardened: pre-canonicalization check (traversal, UNC, device), then canonicalize, then re-check resolved path.
  - Extension-based file type detection via `FileType::from_extension`.

- **`derive_display_name(path, override_name, production_mode)`** → `String`
  - Override > production generic > path filename.
  - Sanitizes override names.

- **`sanitize_display_name(name)`** → `String`
  - Filters control chars, path separators, null bytes.
  - Strips leading dots, trims, truncates to 255 chars.

- **`is_path_safe(path)`**, **`reject_network_path(path)`**, **`reject_device_path(path)`**
  - Multi-layer path security checks.

**Tests:** 11 unit tests covering valid files, unsupported types, missing files, traversal, size limits, display name derivation, sanitization, path safety.

### `src/extractor.rs` (96 LOC)

- **`extract_text(path, file_type)`** → `Result<ExtractionResult, CiteError>`
  - Dispatches to `extract_plain_text` (TXT/MD) or `extract_pdf_text` (PDF).

- **`extract_plain_text(path)`**
  - Returns single PageText (page=1) with full file content.
  - Uses `content.len()` for `total_chars` (byte count, not char count).

- **`extract_pdf_text(path)`**
  - Uses `lopdf` to extract text per page.
  - Handles extraction failures gracefully (empty string + warning).
  - Sorts pages by page number.

**Tests:** 7 unit tests covering TXT, MD, empty files, invalid UTF-8, whitespace-only files.

### `src/chunker.rs` (226 LOC)

- **`chunk_text(pages, chunk_size, overlap, min_size)`** → `Result<Vec<ChunkInput>, CiteError>`
  - Validates parameters (chunk_size > 0, overlap < chunk_size).
  - `build_combined_text`: concatenates pages with `\n` separators, builds char-offset → page mapping.
  - Chunking loop: character-based indexing (`chars().skip().take()`), sentence boundary seeking near chunk end, overlap stepping.
  - Filters chunks below `min_size` (except last).

- **`find_sentence_boundary(text, search_start, target_end)`**
  - Scans for `. `, `! `, `? `, `\n\n` in range.
  - Returns offset after boundary (first char of next sentence).

- **`resolve_page(char_page_map, start, end)`**
  - Maps char offset to page number via lookup table.

**Tests:** 9 unit tests covering basic chunking, overlap, small text, empty input, page tracking, sentence boundaries, min_size filtering, UTF-8, invalid params.

### `src/sentence_chunker.rs` (152 LOC)

- **`chunk_by_sentence(text, min_chars)`** → `Vec<SentenceChunk>`
  - Splits on `.`, `!`, `?` boundaries with whitespace lookahead.
  - Merges sentences shorter than `min_chars` with the next sentence.
  - Uses `is_abbreviation` to avoid splitting on common abbreviations (Dr., Mr., etc.).

- **`split_sentences(text)`** → `Vec<SentenceInfo>`
  - Character-by-character scan with abbreviation detection.
  - Tracks `offset_start` per sentence.

**Tests:** 10 unit tests covering basic sentences, short sentence merging, UTF-8, empty text, abbreviations, offset tracking, boundary cases.

### `src/lib.rs` (180 LOC)

- **`ingest_document(db, document_id, text, file_type, config)`** → `Result<Vec<String>, CiteError>`
  - Main entry point. Orchestrates chunking, storage, and hierarchy building.
  - Chunking mode selected by `config.sentence_chunking`.
  - Hierarchy building selected by `config.build_hierarchy`.
  - For markdown: extracts headings, builds hierarchy, assigns chunks to topics/concepts via offset boundaries.
  - For non-markdown with hierarchy: creates single "Untitled" topic.
  - Two-pass chunk-to-topic assignment: (1) concept-level from hierarchy, (2) topic-level from heading boundaries.

**Tests:** 8 unit tests covering basic ingest, empty text, default config, sentence chunking, hierarchy for markdown/non-markdown, combined mode.

### `tests/ingest_e2e.rs` (105 LOC)

- End-to-end integration tests using real fixture files.
- Tests: TXT/MD file validation → extraction → chunking pipeline.
- Error cases: unsupported type, file too large, missing file.
- Display name derivation.
- Chunker overlap verification.

**Tests:** 7 e2e tests. All pass.

---

## Decisiones de Diseño

### 1. Doble modo de chunking

El crate ofrece dos estrategias de chunking seleccionables por config:
- **Fijo** (`chunker.rs`): chunk_size_chars + overlap. Más predecible para embeddings.
- **Por oraciones** (`sentence_chunker.rs`): split en boundaries de oración + merge de chunks cortos. Mejor preservación semántica.

**Tradeoff:** el chunking por oraciones no tiene `max_chunk_chars` como límite superior real — una oración muy larga no se divide. El campo `max_chunk_chars` de `IngestConfig` no se usa en `sentence_chunker.rs`.

### 2. Jerarquía opcional como feature flag

`build_hierarchy` es opt-in. Permite topic/concept mapping para markdown sin afectar el flujo básico.

**Tradeoff:** la asignación de chunks a topics usa una lógica compleja de dos pasos en `lib.rs` que replica parcialmente lo que hace `build_hierarchy` en `graph`. Esto crea duplicación de lógica de boundary matching.

### 3. Seguridad de paths en profundidad

El validador implementa defensa en capas: pre-canonicalización, canonicalización, post-canonicalización. Rechaza traversal, UNC, y device paths. Buena práctica para un sistema que lee archivos del filesystem.

### 4. Extracción PDF tolerante a fallos

`extract_pdf_text` no falla si una página individual no puede extraer texto (e.g. scanned images). Retorna string vacío + warning. Decisión razonable para robustez.

### 5. UUIDs v4 para chunk IDs

Cada chunk recibe un UUID v4 aleatorio. No hay determinismo — re-ingestar el mismo documento produce IDs diferentes. Esto es correcto para el caso de uso (no se necesita idempotencia a nivel chunk ID).

### 6. Interfaz string-typed para document_id/topic_id

Los IDs se pasan como `&str` y se almacenan como `String`. No hay newtype wrappers. Funcional pero propenso a errores de swap de parámetros.

---

## Conexiones con Otros Crates

| Crate | Relación | Uso en ingest |
|-------|----------|---------------|
| `common` | Error types (`CiteError`), domain types (`Chunk`, `FileType`) | Importa tipos fundamentales usados en todo el pipeline |
| `config` | `IngestConfig` | Parámetros de chunking, feature flags, límites de tamaño |
| `graph` | `extract_headings`, `build_hierarchy` | Parsing de headings markdown y construcción de jerarquía topic/concept |
| `storage` | `Database` | Inserción de chunks, topics, concepts; actualización de jerarquía en chunks |
| `lopdf` | PDF parsing | Extracción de texto de archivos PDF |
| `uuid` | UUID v4 generation | Generación de IDs únicos para chunks |
| `chrono` | Timestamps | `created_at` en cada chunk |

**Nota:** `ingest` no es dependido por otros crates directamente en el pipeline de request → response. Es un crate de "ingestión batch" que alimenta `storage`. Los crates downstream (embedding, retrieval) consumen los chunks desde storage, no desde ingest.

---

## Observaciones Adicionales

### Cobertura de tests

- **57 tests totales** (50 unit + 7 e2e). Todos pasan.
- Buena cobertura de casos felices y edge cases básicos.
- **Gap importante:** ningún test usa texto con caracteres multi-byte (emoji, CJK, acentos extensos) para probar offsets y truncación. Esto oculta bugs UTF-8 críticos (ver `errores.md`).

### Acoplamiento con `graph`

La lógica de asignación de chunks a topics en `lib.rs` (las ~40 líneas desde "Collect concept-level assignments" hasta el final) replica y complementa la lógica de `build_hierarchy`. Si `build_hierarchy` cambiara su estrategia de asignación, `lib.rs` necesitaría cambios paralelos. Considerar delegar toda la asignación a `graph`.

### Offsets como u32

`ChunkInput.offset_start` y `offset_end` son `u32`. El campo `Chunk.offset_start` y `offset_end` en `common::types::Chunk` son `Option<u32>`. Para documentos muy grandes (>4B caracteres = ~16GB con texto ASCII), esto haría overflow. Bajo en probabilidad pero es una limitación arquitectural.
