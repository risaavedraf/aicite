# Análisis de Funcionamiento — aiharness

**Fecha:** 2026-06-02
**Alcance:** Cómo funciona el sistema end-to-end: qué hace cada crate, cómo se conectan, qué datos fluyen, y qué diseño sostiene el todo.
**Fuentes:** 9 `review.md` por crate, `analisis-final.md`, `README.md`, `compliance/review.md`
**Foco:** Funcionamiento (qué hace el código y cómo), no bugs.

---

## 1. Architecture Overview

### 1.1 Qué es el Sistema

aiharness es un **harness de desarrollo con IA** en Rust que produce un CLI llamado `cite`. Permite ingerir documentos en un corpus local, generar embeddings vectoriales, y hacer búsqueda semántica con citas y trazabilidad. Diseñado para uso local/privado con un camino definido hacia production.

**Stack:** Rust, SQLite (via rusqlite), reqwest (blocking HTTP), lopdf (PDF parsing), clap (CLI), serde/JSON.

### 1.2 Dependency Graph entre los 9 Crates

```
                          ┌───────────┐
                          │  common   │  ← nodo raíz (tipos, errores, exit codes)
                          └─────┬─────┘
                                │
           ┌────────────────────┼────────────────────┐
           │                    │                    │
     ┌─────▼──────┐      ┌──────▼──────┐      ┌─────▼──────┐
     │   config   │      │  providers  │      │   graph    │
     │(runtime,   │      │(gemini,     │      │(headings,  │
     │ retrieval, │      │ openai,     │      │ hierarchy, │
     │ ingest     │      │ eval)       │      │ topics,    │
     │ params)    │      └─────┬───────┘      │ concepts)  │
     └─────┬──────┘            │              └─────┬──────┘
           │                   │                    │
     ┌─────▼───────────────────┼────────────────────▼───────┐
     │                    ┌────▼─────┐                      │
     │                    │  ingest  │ (extractor, chunker, │
     │                    │          │ validator, sentence) │
     │                    └────┬─────┘                      │
     │                         │                            │
     │                   ┌─────▼──────┐                     │
     │                   │  storage   │ (SQLite: docs,      │
     │                   │            │ chunks, embeddings, │
     │                   │            │ topics, traces,     │
     │                   │            │ snapshots, locks,   │
     │                   │            │rate_limits, backlog)│
     │                   └─────┬──────┘                     │
     │                         │                            │
     │                   ┌─────▼──────┐  ┌─────────────┐    │
     │                   │  retrieval │  │             │    │
     │                   │(cosine sim,│  │             │    │
     │                   │ ranking)   │  │             │    │
     │                   └─────┬──────┘  │             │    │
     │                         │         │             │    │
     └─────────────────────────┼─────────┘             │
                               │                       │
                         ┌─────▼───────┐               │
                         │   engine    │ ◄─────────────┘
                         │(context,    │  (orquesta todo)
                         │ retrieve,   │
                         │ ingest,     │
                         │ refresh,    │
                         │ evaluate,   │
                         │ recovery)   │
                         └─────┬───────┘
                               │
                         ┌─────▼───────┐
                         │     cli     │  ← punto de entrada (binario `cite`)
                         │(14 subcmds, │
                         │ output,     │
                         │ main)       │
                         └─────────────┘
```

**Regla fundamental:** `engine` es el único crate que orquesta todos los demás. `cli` es el compositor raíz que instancia dependencias concretas y las inyecta a `engine`. Ningún crate downstream conoce `cli`.

### 1.3 Main Entry Point

El binario `cite` se construye desde `crates/cli/src/main.rs`. El flujo de arranque:

```
main()
  ├── dotenvy::dotenv()                    // carga .env si existe
  ├── Cli::parse()                         // clap parsea flags globales + subcomando
  ├── Config::load_from(config_path)       // defaults → config file → env overrides
  ├── apply_cli_overrides(data_dir, mode)  // flags CLI sobre-escriben config
  ├── [si retrieval command] show_provider_disclosure()  // banner a stderr
  ├── [si no es health/setup] run_startup_recovery()
  │     ├── create_dir_all(data_dir)
  │     ├── Database::open()
  │     └── recover_interrupted_processing()
  └── match command → dispatch a commands/*::execute()
```

### 1.4 Execution Pipeline (Resumen)

El sistema opera en 4 pipelines principales, todos orquestados por `engine`:

| Pipeline | Trigger | Flujo Simplificado |
|----------|---------|-------------------|
| **Ingest** | `cite ingest file.pdf` | validate → extract → chunk → embed → store |
| **Search/Retrieve** | `cite search "query"` | embed query → fetch candidates → cosine rank → return hits |
| **Context Pack** | `cite context "query"` | search pipeline → classify result kind → build citations → persist trace |
| **Refresh** | `cite refresh` | collect ready docs → build snapshot → atomic swap |

---

## 2. Crate-by-Crate Functioning

### 2.1 `common` — Tipos Compartidos

**Propósito:** Crate raíz del workspace. Centraliza tipos de dominio, el sistema de errores unificado, y códigos de salida CLI. Todos los demás crates dependen de él.

**Public API:**
- `CiteError` — enum de 18 variantes con `code()`, `exit_code()`, `to_json_response()`, `message()`
- `ExitCode` — 8 variantes `#[repr(i32)]` para códigos de salida CLI
- Tipos de dominio: `Document` (13 campos), `Chunk` (10 campos), `Citation` (12 campos), `ContextResponse`, `ContextMetadata`, `TraceResponse`, `ReadSelector`, `ReadResponse`
- Newtypes: `DocumentId(String)`, `ChunkId(String)`, `TraceId(String)` — definidos pero no usados consistentemente
- Enums: `DocumentStatus` (Pending/Processing/Ready/Failed), `FileType` (Pdf/Txt/Md), `ResultKind` (Context/NoResults/InsufficientContext)

**Módulos:**
- `error.rs` — `CiteError` con thiserror, mapeo a exit codes y JSON responses
- `exit.rs` — `ExitCode` con representación i32
- `types.rs` — ~640 líneas de tipos de dominio compartidos

**Conexiones:** Importado por los 8 crates restantes. `CiteError` es el tipo más usado del workspace.

### 2.2 `config` — Sistema de Configuración

**Propósito:** Carga y merge de configuración desde múltiples fuentes con precedencia definida.

**Public API:**
- `Config::load()` / `Config::load_from(path)` — punto de entrada
- Sub-structs: `RuntimeConfig`, `PathsConfig`, `EmbeddingConfig`, `RetrievalConfig`, `RateLimitConfig`, `IngestConfig`
- `RuntimeMode` — enum con 3 variantes: `LocalPrivateDemo`, `PublicPackagedDemo`, `Production`

**Módulos:** Un único `lib.rs` (~420 líneas) con toda la lógica.

**Precedencia de configuración:** `CLI flags > env vars (CITE_*) > TOML file > defaults`

**Conexiones:** Consumido por `cli` (carga), `engine` (runtime mode, retrieval params), `ingest` (chunk params).

### 2.3 `providers` — Proveedores de Embedding

**Propósito:** Abstraer la generación de embeddings vectoriales detrás de un trait, con 3 implementaciones.

**Public API:**
- `trait EmbeddingProvider` — `embed(&self, text: &str) -> Result<Vec<f32>, CiteError>` + `model_id()` + `provider_id()`
- `GeminiProvider` — HTTP blocking a Google Gemini API
- `OpenAICompatibleProvider` — HTTP blocking a cualquier API compatible con OpenAI embeddings
- `EvalProvider` — Mock determinista basado en keywords (8 dimensiones temáticas)

**Módulos:**
- `lib.rs` (11 líneas) — Trait definition
- `gemini.rs` — Implementación Gemini
- `openai.rs` — Implementación OpenAI-compatible (valida HTTPS)
- `eval.rs` — Provider offline para evaluación

**Conexiones:** CLI instancia providers concretos → los boxing como `Box<dyn EmbeddingProvider>` → inyecta a engine. El engine nunca conoce implementaciones concretas.

### 2.4 `graph` — Jerarquía de Tópicos/Conceptos

**Propósito:** Extraer estructura jerárquica de documentos markdown (H2→Topic, H3→Concept) y asignar chunks a esa jerarquía por posición.

**Public API:**
- `extract_headings(markdown: &str) -> Vec<HeadingSpan>` — parsing de headings con offsets
- `build_hierarchy(document_id, headings, chunk_offsets) -> HierarchyResult` — asignación de chunks a topics/concepts

**Módulos:**
- `heading_parser.rs` (~110 líneas) — Parsing de markdown headings
- `hierarchy.rs` (~280 líneas) — Construcción de jerarquía y asignación de chunks
- `types.rs` (~40 líneas) — `Topic`, `Concept`, `HeadingSpan`, `SemanticLink` (este último es dead code)

**Conexiones:** Crate hoja — consumido solo por `ingest`. Produce datos que `ingest` persiste en `storage`.

### 2.5 `ingest` — Pipeline de Ingesta

**Propósito:** Pipeline completo de ingesta: validación de archivos, extracción de texto, chunking, y orquestación de jerarquía.

**Public API:**
- `validate_file(path, max_size)` → `(FileType, u64)` — validación con seguridad de paths
- `extract_text(path, file_type)` → `Vec<PageText>` — extracción por tipo de archivo
- `chunk_text(pages, config)` → `Vec<ChunkInput>` — chunking fijo con overlap
- `chunk_by_sentence(text, min_chars)` → `Vec<SentenceChunk>` — chunking por oraciones
- `ingest_document(db, document_id, text, file_type, config)` → `Vec<String>` — orquestación completa

**Módulos:**
- `validator.rs` (126 LOC) — Path safety multi-layer, file type detection, display name
- `extractor.rs` (96 LOC) — TXT/MD via `read_to_string`, PDF via `lopdf`
- `chunker.rs` (226 LOC) — Fixed-size chunking con sentence boundary seeking y overlap
- `sentence_chunker.rs` (152 LOC) — Sentence-level chunking con merge de chunks cortos
- `lib.rs` (180 LOC) — Orquestación: chunking → storage → hierarchy

**Conexiones:** Depende de `common` (tipos), `config` (IngestConfig), `graph` (headings/hierarchy), `storage` (persistencia), `lopdf` (PDF).

### 2.6 `storage` — Persistencia SQLite

**Propósito:** Toda la persistencia del sistema a través de un handle `Database` que envuelve SQLite.

**Public API:**
- `Database::open(path)` / `Database::open_memory()` — constructor
- ~50 métodos públicos organizados por dominio

**Módulos (14 archivos):**

| Módulo | Responsabilidad |
|--------|-----------------|
| `lib.rs` | Handle `Database`, WAL config, migrations |
| `documents.rs` | CRUD documentos, status transitions, retry, crash recovery |
| `chunks.rs` | Bulk insert/delete chunks, hierarchy assignment |
| `embeddings.rs` | Bulk insert embeddings (BLOB f32 LE), hierarchical/flat queries |
| `topics.rs` | CRUD topics, chunk_count recalculation |
| `concepts.rs` | CRUD concepts, chunk_count recalculation |
| `semantic_links.rs` | Cross-chunk semantic relationships |
| `traces.rs` | Trace headers + citations, envelope retrieval |
| `snapshots.rs` | Atomic snapshot lifecycle: build → activate → supersede |
| `locks.rs` | Named durable locks for coordination |
| `rate_limits.rs` | Fixed-window rate limiting |
| `backlog.rs` | Durable ingest queue with idempotency |
| `migrations/` | 7 sequential migrations, embebidas via `include_str!` |
| `util.rs` | Error conversion, datetime helpers, row mapping |

**Conexiones:** Consumido por `engine` (principal), `cli` (directo para health/trace/evaluate), `ingest` (para pipeline de ingest).

### 2.7 `retrieval` — Motor de Similitud

**Propósito:** Crate puro sin I/O. Compara vectores de embedding y rankea por similitud coseno.

**Public API:**
- `cosine_similarity(a, b) -> Option<f32>` — cálculo puro
- `rank_by_similarity(query, candidates, k) -> Vec<ScoredChunk>` — pipeline completo
- `ScoredChunk` — struct con 14 campos (9 del chunk + score + 4 jerarquía)

**Módulos:** Un único `lib.rs` (~260 líneas).

**Conexiones:** Importa `ChunkEmbeddingRecord` de `storage`. Llamado por `engine` para ranking.

### 2.8 `engine` — Lógica de Negocio

**Propósito:** Capa de orquestación central. Coordina ingest, retrieval, context assembly, refresh, recovery, evaluación, y runtime guards. **No instancia dependencias** — todo le llega inyectado.

**Public API:**
- `ingest::ingest()`, `enqueue_ingest()`, `ingest_next()`, `retry_document()`
- `retrieve::search()`, `retrieve()`
- `context::build_context()`, `read_context()`, `get_trace()`
- `refresh::refresh_corpus()`
- `recovery::recover_interrupted_processing()`
- `evaluate::run_evaluation()`
- `runtime_guard::check_ingest_allowed()`, `is_real_provider()`

**Módulos:**

| Módulo | Líneas | Responsabilidad |
|--------|--------|-----------------|
| `context.rs` | ~940 | Context pack assembly: retrieval → classify → citations → trace |
| `retrieve.rs` | ~930 | Búsqueda vectorial con jerarquía, rate limiting, validación |
| `ingest.rs` | ~510 | Pipeline de ingest con lock, backlog queue, retry |
| `refresh.rs` | ~175 | Snapshot swap atómico del corpus |
| `evaluate.rs` | ~220 | Evaluación contra golden dataset |
| `recovery.rs` | ~95 | Recovery de documentos interrumpidos |
| `runtime_guard.rs` | ~28 | Guards de modo de runtime |
| `golden_provider.rs` | ~35 | Provider mock determinista |
| `lib.rs` | ~10 | Re-exports + struct `Engine` vacía (dead code) |

**Conexiones:** Depende de `common`, `storage`, `ingest`, `providers`, `config`, `retrieval`. Consumido exclusivamente por `cli`.

### 2.9 `cli` — Punto de Entrada

**Propósito:** Binario `cite` con 14 subcomandos. Parsea argumentos, construye contextos de ejecución, delega al engine, y formatea output.

**Public API:** No expone APIs — es consumer final.

**Módulos:**

| Módulo | Responsabilidad |
|--------|-----------------|
| `main.rs` (240 LOC) | Entry point: parse → config → recovery → dispatch |
| `output.rs` (340 LOC) | JSON/human output, tipos compactos (~60-70% reducción) |
| `commands/mod.rs` (100 LOC) | `CommandContext`, `resolve_api_key`, `create_provider` |
| 13 archivos de comandos | Uno por subcomando |

**14 subcomandos organizados en 4 categorías:**

| Categoría | Comandos | Necesita DB | Necesita Provider |
|-----------|----------|-------------|-------------------|
| Setup | `health`, `setup` | health sí | health sí, setup sí (propio) |
| Ingesta | `ingest`, `list`, `get`, `retry` | Todos | Solo `ingest` |
| Búsqueda | `search`, `retrieve`, `context`, `read` | Todos | search, retrieve, context |
| Gestión | `trace`, `refresh`, `evaluate` | trace, refresh | Solo evaluate (EvalProvider) |

---

## 3. Data Flow

### 3.1 Pipeline de Ingest (end-to-end)

```
cite ingest documento.pdf --data-dir ./mi_corpus
│
├── 1. CLI: Config::load_from() → apply_cli_overrides()
├── 2. CLI: recover_interrupted_processing() (startup safety)
├── 3. CLI: CommandContext::open() → Database + Box<dyn EmbeddingProvider>
│
└── 4. engine::ingest::ingest()
    ├── validate_file() → FileType, size check, path safety [ingest::validator]
    ├── try_acquire_lock("ingest_pipeline") [storage::locks]
    │   └── Si ocupado + queue_on_lock_conflict → upsert backlog [storage::backlog]
    ├── insert_document(Pending) [storage::documents]
    ├── update_document_status(Processing) [storage::documents]
    │
    ├── extract_text(path, file_type) → Vec<PageText> [ingest::extractor]
    │   └── PDF: lopdf por página. TXT/MD: read_to_string.
    │
    ├── chunk_text(pages, config) → Vec<ChunkInput> [ingest::chunker]
    │   └── Split por tamaño + overlap + sentence boundary seeking
    │   └── (alternativa: chunk_by_sentence si sentence_chunking=true) [ingest::sentence_chunker]
    │
    ├── insert_chunks() → storage [storage::chunks] (bulk, transaccional)
    │
    ├── Para cada chunk: provider.embed(text) → Vec<f32> [providers]
    │
    ├── insert_embeddings() → storage [storage::embeddings] (bulk BLOB f32 LE)
    │
    ├── [Si build_hierarchy && markdown]:
    │   ├── extract_headings(text) → Vec<HeadingSpan> [graph::heading_parser]
    │   ├── build_hierarchy(doc_id, headings, offsets) → topics/concepts [graph::hierarchy]
    │   ├── insert_topic, insert_concept → storage [storage::topics, concepts]
    │   └── set_chunk_hierarchy → storage [storage::chunks]
    │
    ├── update_document_status(Ready) + update_chunk_count [storage::documents]
    └── release_lock() [storage::locks]

Resultado: "✓ Ingested 'documento.pdf' (15 chunks, 3 topics)"
```

### 3.2 Pipeline de Context Pack (búsqueda con citas)

```
cite context "¿Cómo funciona la autenticación?" --k 5
│
├── 1. CLI: Config + CommandContext::open() (DB + provider)
│
└── 2. engine::context::build_context()
    ├── validate_corpus_ready() → al menos 1 doc Ready [storage]
    ├── validate_query() → no vacía, no solo puntuación, max 4000 chars
    ├── enforce_rate_limit() → storage::rate_limits (20 req/min por provider)
    ├── provider.embed(query) → query_vector: Vec<f32> [providers]
    │
    ├── fetch_candidates(db, config):
    │   ├── [Si use_hierarchy && hay datos jerárquicos]:
    │   │   └── list_chunk_embeddings_hierarchical() → Vec<HierarchicalChunkEmbedding>
    │   └── [Else]:
    │       └── list_ready_chunk_embeddings() → Vec<ChunkEmbeddingRecord>
    │
    ├── retrieval::rank_by_similarity(query_vector, candidates, top_k) → Vec<ScoredChunk>
    │   └── cosine_similarity para cada candidato → sort desc → truncate k
    │
    ├── enrich_with_hierarchy() → agrega topic_name, concept_name, breadcrumb
    │
    ├── compute_result_kind(scores, config):
    │   ├── top_score < evidence_floor (0.50) → NoResults
    │   ├── top_score < confidence_threshold (0.70) → InsufficientContext
    │   └── (else) → Context
    │
    ├── build_citations_from_ranked() → Vec<Citation> con confidence labels
    ├── persist_trace() → storage::traces (header + citations para auditoría)
    └── Retorna ContextResponse { citations, metadata, result_kind, instructions }

CLI imprime citations con scores, breadcrumbs, o JSON si --json
```

### 3.3 Pipeline de Refresh (atomic snapshot swap)

```
cite refresh
│
└── engine::refresh::refresh_corpus()
    ├── begin_snapshot_build(snapshot_id) → snapshot "building" [storage::snapshots]
    ├── list_documents_by_status(Ready) [storage::documents]
    ├── attach_document_to_snapshot() × N [storage::snapshots]
    └── activate_snapshot() → swap atómico [storage::snapshots]:
        ├── Supersede snapshot anterior
        ├── Activar nuevo
        ├── Upsert active pointer
        └── Retorna RefreshResult { snapshot_id, document_count, previous_snapshot_id }
```

### 3.4 Pipeline de Recovery (startup)

```
cite <cualquier-comando> (excepto health, setup)
│
└── recover_interrupted_processing() [engine::recovery]
    ├── Verifica lock "ingest_pipeline" no activo [storage::locks]
    └── Documentos en Processing → Failed (con error "interrupted_processing_recovered")
```

### 3.5 Flujo de Evaluación

```
cite evaluate [--json]
│
└── engine::evaluate::run_evaluation()
    ├── Crea DB en memoria + EvalProvider (determinístico, sin red)
    ├── Seed corpus: 3 documentos, 12 chunks inline
    ├── Para cada golden fixture (10):
    │   ├── build_context() con el corpus en memoria
    │   └── Compara result_kind + min_citations contra expectativas
    └── Retorna EvalReport { hit_rate, results[] }
```

---

## 4. Key Design Decisions

### 4.1 Dependency Injection via Trait Objects

**Patrón:** CLI es el compositor raíz. Crea implementaciones concretas y las inyecta a engine como referencias trait-object.

```
CLI: GeminiProvider::new() → Box<dyn EmbeddingProvider> → CommandContext.provider
CLI: Database::open() → CommandContext.db
Engine recibe: &Database, &dyn EmbeddingProvider, &Config
```

**Resultado:** Engine no conoce implementaciones concretas. Testing con `EvalProvider` es trivial. El coupling es mínimo entre la capa de orquestación y las capas de infraestructura.

### 4.2 Unified Error Type (`CiteError`)

**Patrón:** Un único enum de 18 variantes con 3 representaciones:
- `code()` → string machine-readable (`"document_not_found"`)
- `exit_code()` → `ExitCode` enum para CLI
- `to_json_response()` → `ErrorResponse` serializable para agentes

**Resultado:** Consistencia total en error handling. Un solo punto de verdad para mapeo error→exit code. Los consumidores no necesitan lógica de serialización de errores.

### 4.3 Functional Decomposition (no OOP)

**Patrón:** No hay struct `Engine` con métodos. Toda la funcionalidad son funciones libres en módulos que reciben `&Database`, `&dyn EmbeddingProvider`, etc. como parámetros.

**Resultado:** Código directo, sin estado oculto, fácil de razonar. El trade-off es que `Database` es un "god object" con ~50 métodos, pero para storage puro esto es pragmático.

### 4.4 Lock-Based Concurrency para Ingest

**Patrón:** Un lock named `ingest_pipeline` serializa todos los ingests. Si el lock está ocupado, la operación se encola en backlog.

**Resultado:** Garantiza que solo un ingest corra a la vez. El backlog con idempotency key permite re-ingest sin duplicación. Adecuado para CLI/demo, pero sería bottleneck en producción concurrente.

### 4.5 Snapshot Staging + Atomic Swap

**Patrón:** `building → attach docs → activate` con transacción que supersedes el anterior.

**Resultado:** Zero-downtime corpus swap. Los reads siempre ven datos consistentes (el snapshot activo anterior mientras se construye el nuevo).

### 4.6 Feature Flags Opt-in

**Patrón:** `sentence_chunking: bool` y `build_hierarchy: bool` como campos en `IngestConfig`, ambos `false` por defecto.

**Resultado:** Comportamiento conservador por defecto. El chunking por oraciones y la jerarquía de topics son mejoras opt-in que no afectan el flujo básico.

### 4.7 Dual Output (JSON/Human)

**Patrón:** Cada comando tiene `if json { print_json } else { println formato legible }`. Los commands de búsqueda soportan además compact vs full JSON.

**Resultado:** Los responses completos son ~600-1500 tokens. Los compactos ~200-250 tokens, más adecuados para consumo por agentes. El flag `--full` permite acceso al JSON completo.

### 4.8 Migrations Embebidas

**Patrón:** Archivos SQL incluidos via `include_str!` con versionado secuencial por entero.

**Resultado:** Las migraciones están compiladas en el binario. No hay problemas de path en runtime. 7 migraciones cubren el schema completo.

### 4.9 Error Handling como Exit Codes

**Patrón:** Los comandos CLI retornan `i32` exit codes en lugar de `Result`. 8 variantes de `ExitCode` mapean a códigos POSIX.

**Resultado:** Simple para un CLI fire-and-forget. El trade-off es dificultad para testing y reuso de la lógica CLI como librería.

---

## 5. Gaps and Concerns NOT Related to Bugs

### 5.1 Abstractions Definidas pero No Usadas

Los newtypes `DocumentId`, `ChunkId`, `TraceId` están definidos en `common::types` con `Display`, `From<String>`, `AsRef<str>`, `Hash` — pero todos los structs de dominio (`Document`, `Chunk`, `Citation`) usan `String` para IDs. Los newtypes son dead code que promete type-safety que no se entrega. Similarmente, `SemanticLink` en `graph::types` está definido sin uso.

**Impacto:** Los lectores del código esperan type-safety que no existe. La migración es posible pero requiere cambios coordinados en todos los crates.

### 5.2 Config Diseñada pero Parcialmente Consumida

`IngestConfig` define `embedding_timeout_secs` (default 30s), pero ambos providers hardcodean `Duration::from_secs(30)` sin leer el config. El campo existe, es configurable via env var `CITE_EMBEDDING_TIMEOUT`, pero nunca llega al provider.

El archivo TOML soporta ~7 campos (provider type/key/model, retrieval top_k/evidence_floor/confidence_threshold, data dir), mientras que las env vars soportan ~20 campos. Hay una asimetría significativa: un usuario que configura via archivo TOML tiene acceso a menos de la mitad de los campos disponibles.

`min_chunk_size_chars` y `min_chunk_chars` coexisten en `IngestConfig` con defaults diferentes (100 y 30 respectivamente), creando confusión sobre cuál controla qué.

### 5.3 `Engine` Struct Vacía

`engine/lib.rs` declara `pub struct Engine;` que no se usa en ningún lugar. Toda la funcionalidad del engine se accede a través de funciones libres en módulos. La struct es un placeholder sin propósito actual que confunde a los lectores.

### 5.4 Dead Code Visible

- `into_compact_*()` en `cli/output.rs` — marcadas `#[allow(dead_code)]`, nunca usadas (las versiones borrowing `to_compact_*()` sí se usan)
- `SemanticLink` en `graph/types.rs` — definido, exportado, nunca instanciado
- `check_ingest_allowed` en `engine/runtime_guard.rs` — definida con tests, nunca llamada en el path real de ingest
- `SemanticLinkRow` en `storage/semantic_links.rs` — tiene CRUD completo pero nadie lo usa

### 5.5 DRY Violations en CLI

- El patrón de error display (`if json { print_json(&e.to_json_response()) } else { eprintln!("Error: {e}") }`) se repite 14+ veces sin helper compartido
- La validación de flags (`--flat` incompatible con `--topic`/`--concept`) está duplicada en `search.rs`, `retrieve.rs`, y `context.rs`
- El unwrap inconsistente del provider: `search.rs` usa `match`, `context.rs` y `retrieve.rs` usan `.unwrap()`

### 5.6 Acoplamiento entre `ingest` y `graph`

La lógica de asignación de chunks a topics/concepts en `ingest/lib.rs` (~40 líneas) replica y complementa la lógica de `build_hierarchy` en `graph/hierarchy.rs`. Si `build_hierarchy` cambiara su estrategia de asignación, `lib.rs` necesitaría cambios paralelos. La responsabilidad de boundary matching está fragmentada entre ambos crates.

### 5.7 `Database` como God Object

`Database` tiene ~50 métodos públicos cubriendo documents, chunks, embeddings, topics, concepts, semantic links, traces, snapshots, locks, rate limits, backlog, y health. No hay traits de dominio ni separación por aggregate. Esto es pragmático para un storage puro pero:
- Impide testear consumidores con mocks de storage
- Hace que el handle sea un punto de coupling masivo
- Cualquier cambio de schema requiere tocar el mismo struct

### 5.8 Inconsistencia de Tipos Temporales

`Document` y `Chunk` usan `DateTime<Utc>` para `created_at`. `Topic`, `Concept`, y `SemanticLink` usan `String` formateado con `"%Y-%m-%d %H:%M:%S"`. Esto impide comparaciones temporales tipadas entre entidades y requiere parsing manual en los módulos que necesitan calcular diferencias.

### 5.9 Offsets: Ambigüedad bytes vs caracteres

No hay metadata que indique si un offset representa bytes o caracteres. `HeadingSpan.char_offset` se acumula con `line.len()` (bytes). `ChunkInput.offset_start/end` son `u32` producidos por el chunker que usa `chars().skip().take()` (caracteres). La mezcla de estos dos sistemas es la causa raíz de bugs cross-crate para texto no-ASCII, pero incluso sin bugs, la falta de tipado diferenciado hace que el código sea difícil de razonar.

### 5.10 Recovery Ejecutado en Comandos de Solo Lectura

`run_startup_recovery()` se ejecuta antes de cada comando excepto `health` y `setup`. Esto incluye comandos de solo lectura como `list`, `get`, `read`, y `trace`, que no necesitan recovery. Agrega latencia innecesaria y abre una conexión DB que se cierra inmediatamente después.

### 5.11 Jerarquía Estrictamente 2 Niveles

Solo H2 y H3 participan en la jerarquía semántica. H1 se ignora (probablemente título del documento), H4+ se ignora. Para documentos con estructura profunda (H4 para subsecciones), la jerarquía no captura la granularidad disponible. Es una decisión de simplicidad deliberada pero que limita la expresividad.

### 5.12 Sin Retry ni Backoff en Providers

Los providers hacen un único intento HTTP. Para una CLI interactiva es aceptable (el usuario reintenta), pero para uso programático (batch ingest de múltiples archivos) un retry con backoff exponencial sería deseable. El config define `max_retry_count: 3` pero se aplica al pipeline de ingest, no a las llamadas HTTP individuales al provider.

### 5.13 ScoredChunk Replica Campos

`ScoredChunk` (en `retrieval`) replica 9 campos de `ChunkEmbeddingRecord` (en `storage`) en lugar de envolverlo. Si `ChunkEmbeddingRecord` agrega un campo, `ScoredChunk` y `rank_by_similarity` deben actualizarse manualmente. Es un tradeoff deliberado por simplicidad de acceso (evita `item.chunk.text` vs `item.text`), pero crea riesgo de desalineación.

### 5.14 Golden Fixtures en 4 Ubicaciones

Los fixtures de evaluación existen en 4 variantes: `tests/golden/fixtures.rs` (Rust), `tests/golden/fixtures.json` (JSON), `cli/src/commands/evaluate.rs` (inline), y `engine/src/evaluate.rs` (framework). Hay inconsistencias entre variantes (expectativas opuestas para algunos fixtures). No hay una fuente única de verdad.

### 5.15 Snapshot Refresh Sin Rollback Explícito

Si `activate_snapshot` falla después de los `attach_document_to_snapshot`, los attaches quedan como datos huérfanos en el snapshot "building". No hay cleanup explícito que marque el snapshot como failed o limpie los attaches parciales.

---

*Documento generado: 2026-06-02*
*Fuentes: 9 review.md por crate, analisis-final.md, README.md, compliance/review.md*
*Foco: Funcionamiento del sistema — qué hace el código y cómo*
