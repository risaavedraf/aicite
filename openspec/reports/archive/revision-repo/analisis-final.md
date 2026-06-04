# Análisis Final Consolidado — aiharness

**Fecha:** 2026-06-02
**Alcance:** Análisis cross-crate completo del proyecto aiharness (9 crates, ~110 archivos Rust + SQL)
**Fuentes:** 9 reportes de revisión por crate, 9 archivos de errores, 1 reporte cross-crate preliminar, 1 reporte de compliance, 1 archivo de items pendientes de code review anterior
**Metodología:** Lectura, deduplicación, y síntesis de todos los hallazgos individuales; identificación de cadenas de causalidad cross-crate; evaluación de coherencia arquitectónica

---

## Sección 1: Cómo Funciona el Sistema (End-to-End)

### 1.1 Visión General

aiharness es un **harness de desarrollo con IA** que permite a un usuario ingerir documentos en un corpus local, generar embeddings vectoriales, y luego hacer búsqueda semántica con citas y trazabilidad. El CLI se llama `cite` y tiene 14 subcomandos.

El flujo central del sistema es:

```
Documento en disco → Ingest (extracción + chunking + embeddings) → Storage (SQLite) → Retrieval (cosine similarity) → Context Pack (citas + metadata)
```

### 1.2 Los 14 Comandos del CLI

| Categoría | Comando | Qué hace | Crates involucrados |
|-----------|---------|----------|---------------------|
| **Setup** | `health` | Verifica salud del sistema local | cli, storage, config |
| | `setup` | Configura API keys y settings del provider | cli, providers |
| **Ingesta** | `ingest` | Ingiere un documento al corpus | cli → engine → ingest → graph → storage → providers |
| | `list` | Lista documentos en el corpus | cli → storage |
| | `get` | Obtiene metadata de un documento | cli → storage |
| | `retry` | Reintenta un documento fallido | cli → engine → ingest → storage |
| **Búsqueda** | `search` | Búsqueda vectorial (top-k previews) | cli → engine → providers, retrieval, storage |
| | `retrieve` | Búsqueda vectorial (texto completo) | cli → engine → providers, retrieval, storage |
| | `context` | Construye context pack con citas | cli → engine → providers, retrieval, storage |
| | `read` | Lee una cita o chunk por ID | cli → engine → storage |
| **Gestión** | `trace` | Busca metadata de trazabilidad | cli → storage |
| | `refresh` | Refresca corpus con atomic snapshot swap | cli → engine → storage |
| | `evaluate` | Ejecuta evaluación golden dataset | cli → engine → providers, retrieval, storage |

### 1.3 Flujo de Datos Completo: Ingest

```
1. Usuario ejecuta: cite ingest documento.pdf --data-dir ./mi_corpus

2. CLI (cli/main.rs):
   ├── Carga config (config) → aplica overrides (--data-dir, --runtime-mode)
   ├── Ejecuta recover_interrupted_processing (engine/recovery)
   ├── Crea CommandContext (storage::Database + provider)
   └── Llama a engine::ingest::ingest()

3. Engine (engine/ingest.rs):
   ├── validate_file() → tipo, tamaño, seguridad de path (ingest/validator)
   ├── Intenta adquirir lock "ingest_pipeline" (storage/locks)
   │   └── Si lock ocupado → encola en backlog (storage/backlog)
   ├── insert_document(Pending) → storage/documents
   ├── update_document_status(Processing)
   └── run_pipeline():
       ├── extract_text(path, file_type) → Vec<PageText> (ingest/extractor)
       │   └── PDF: lopdf por página. TXT/MD: read_to_string.
       ├── chunk_text(pages, config) → Vec<ChunkInput> (ingest/chunker o sentence_chunker)
       │   └── Split por tamaño + overlap + sentence boundary seeking
       ├── insert_chunks() → storage/chunks (bulk, transaccional)
       ├── Para cada chunk: provider.embed(text) → Vec<f32> (providers)
       ├── insert_embeddings() → storage/embeddings (bulk BLOB f32 LE)
       └── [Si build_hierarchy && markdown]:
           ├── extract_headings(text) → Vec<HeadingSpan> (graph/heading_parser)
           ├── build_hierarchy(doc_id, headings, offsets) → topics/concepts (graph/hierarchy)
           ├── insert_topic, insert_concept → storage/topics, concepts
           └── set_chunk_hierarchy → storage/chunks

4. Resultado:
   ├── update_document_status(Ready) + update_chunk_count
   ├── Release lock
   └── CLI imprime: "✓ Ingested 'documento.pdf' (15 chunks, 3 topics)"
```

### 1.4 Flujo de Datos Completo: Context Pack (Búsqueda con Citas)

```
1. Usuario ejecuta: cite context "¿Cómo funciona la autenticación?" --data-dir ./mi_corpus

2. Engine (engine/context.rs → engine/retrieve.rs):
   ├── validate_query() → no vacía, no solo puntuación, max 4000 chars
   ├── enforce_rate_limit() → storage/rate_limits (20 req/min por provider)
   ├── provider.embed(query) → query_vector: Vec<f32> (providers)
   ├── fetch_candidates(db, config):
   │   ├── [Si use_hierarchy && hay datos jerárquicos]:
   │   │   └── list_chunk_embeddings_hierarchical() → Vec<HierarchicalChunkEmbedding>
   │   └── [Else]:
   │       └── list_ready_chunk_embeddings() → Vec<ChunkEmbeddingRecord>
   ├── retrieval::rank_by_similarity(query_vector, candidates, top_k) → Vec<ScoredChunk>
   │   └── cosine_similarity para cada candidato → sort desc → truncate k
   ├── enrich_with_hierarchy() → agrega topic_name, concept_name, breadcrumb
   ├── compute_result_kind(scores, config):
   │   ├── top_score < evidence_floor → NoResults
   │   ├── top_score < confidence_threshold → InsufficientContext
   │   └── (else) → Context
   ├── build_citations_from_ranked() → Vec<Citation> con confidence labels
   ├── persist_trace() → storage/traces (header + citations para auditoría)
   └── Retorna ContextResponse { citations, metadata, result_kind, instructions }

3. CLI (cli/commands/context.rs):
   └── Imprime citations con scores, breadcrumbs, o JSON si --json
```

### 1.5 Flujo de Refresh (Atomic Snapshot Swap)

```
cite refresh → engine::refresh::refresh_corpus()
  ├── begin_snapshot_build(snapshot_id) → crea snapshot "building"
  ├── list_documents_by_status(Ready)
  ├── attach_document_to_snapshot() × N
  └── activate_snapshot() → swap atómico:
      ├── Supersede snapshot anterior
      ├── Activar nuevo
      └── Upsert active pointer
```

### 1.6 Flujo de Recovery (Startup)

```
cite <cualquier-comando> (excepto health, setup)
  └── recover_interrupted_processing()
      ├── Verifica lock "ingest_pipeline" no activo
      └── Documentos en Processing → Failed (con error "interrupted_processing_recovered")
```

---

## Sección 2: Mapa de Arquitectura

### 2.1 Grafo de Dependencias entre Crates

```
                          ┌───────────┐
                          │  common   │  ← nodo raíz (tipos, errores, exit codes)
                          └─────┬─────┘
                                │
           ┌────────────────────┼────────────────────┐
           │                    │                    │
     ┌─────▼─────┐      ┌──────▼──────┐      ┌─────▼──────┐
     │   config   │      │  providers  │      │   graph    │
     │(runtime,   │      │(gemini,     │      │(headings,  │
     │ retrieval, │      │ openai,     │      │ hierarchy, │
     │ ingest     │      │ eval)       │      │ topics,    │
     │ params)    │      └──────┬──────┘      │ concepts)  │
     └─────┬─────┘             │              └─────┬──────┘
           │                   │                    │
     ┌─────▼───────────────────┼────────────────────▼──────┐
     │                    ┌────▼─────┐                      │
     │                    │  ingest  │ (extractor, chunker, │
     │                    │          │  validator, sentence) │
     │                    └────┬─────┘                      │
     │                         │                            │
     │                   ┌─────▼──────┐                     │
     │                   │  storage   │ (SQLite: docs,      │
     │                   │            │  chunks, embeddings, │
     │                   │            │  topics, traces,     │
     │                   │            │  snapshots, locks,   │
     │                   │            │  rate_limits, backlog│
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

### 2.2 Flujo de Tipos Principales

| Tipo | Definido en | Usado por | Fluye hacia |
|------|------------|-----------|-------------|
| `CiteError` | common/error.rs | TODOS los crates | Error unificado del sistema |
| `ExitCode` | common/exit.rs | cli | Códigos de salida CLI |
| `Document` | common/types.rs | storage, engine, ingest, cli | Persistencia + orquestación |
| `Chunk` | common/types.rs | storage, engine, ingest | Persistencia + chunking |
| `Citation` | common/types.rs | engine, cli | Context pack response |
| `ContextResponse` | common/types.rs | engine, cli | Respuesta top-level de context |
| `EmbeddingProvider` (trait) | providers/lib.rs | engine | Inyección de dependencia |
| `Vec<f32>` (Embedding) | providers/lib.rs | engine, retrieval, storage | Vectores de embedding |
| `ChunkEmbeddingRecord` | storage/embeddings.rs | retrieval, engine | Candidatos para ranking |
| `ScoredChunk` | retrieval/lib.rs | engine | Resultados rankeados |
| `Config` | config/lib.rs | cli, engine | Configuración del runtime |
| `RuntimeMode` | config/lib.rs | cli, engine | Modo de ejecución |
| `IngestConfig` | config/lib.rs | engine, ingest | Parámetros de chunking |
| `HeadingSpan` | graph/types.rs | graph → ingest | Jerarquía de headings |
| `HierarchyResult` | graph/hierarchy.rs | ingest | Topics + concepts |
| `Database` | storage/lib.rs | engine, cli, ingest | Handle de persistencia |

### 2.3 Patrón de Inyección de Dependencia

```
CLI (create_provider)
    ├── resolve_api_key(config) → String
    ├── GeminiProvider::new(model, key) o OpenAICompatibleProvider::new(endpoint, model, key)
    └── Box<dyn EmbeddingProvider> → CommandContext.provider

Engine recibe:
    &Database         ← storage (inyectado por CLI)
    &dyn EmbeddingProvider ← providers (inyectado por CLI)
    &Config/*         ← config (inyectado por CLI)
```

El CLI es el compositor raíz. `engine` nunca instancia dependencias concretas — todo le llega inyectado. Esto es un patrón limpio.

---

## Sección 3: Consolidación de Errores

### 3.1 Resumen Cuantitativo

| Crate | CRITICAL | HIGH | MEDIUM | LOW | Total |
|-------|----------|------|--------|-----|-------|
| common | 1 | 2 | 2 | 2 | 7 |
| config | 0 | 2 | 4 | 3 | 9 |
| engine | 2 | 3 | 4 | 3 | 12 |
| cli | 2 | 4 | 5 | 6 | 17 |
| storage | 2 | 2 | 5 | 4 | 13 |
| ingest | 3 | 3 | 3 | 3 | 12 |
| providers | 1 | 3 | 2 | 4 | 10 |
| retrieval | 0 | 1 | 2 | 3 | 6 |
| graph | 1 | 2 | 2 | 4 | 9 |
| **TOTAL (bruto)** | **12** | **22** | **29** | **32** | **95** |

Tras deduplicación y agrupación por causa raíz:

| Categoría | Cantidad |
|-----------|----------|
| Bugs cross-crate (cadenas de causalidad) | 4 |
| Bugs per-crate | 18 |
| Issues arquitectónicos | 8 |
| Gaps de testing | 7 |
| Deuda técnica / code smell | 14 |
| **Total deduplicado** | **51** |

### 3.2 Bugs Cross-Crate — Cadenas de Causalidad

#### BUG-X1: 🟥 Confusión bytes vs caracteres UTF-8 (EL BUG MÁS IMPORTANTE)

**El hallazgo más crítico de toda la revisión.** Afecta 5 archivos en 3 crates y el bug más importante es invisible para todos los tests existentes.

**Cadena causal completa:**

```
graph/heading_parser.rs:17,35     char_offset += line.len()  (bytes, no chars)
         │
         ▼
ingest/lib.rs:130-161             topic_boundaries usa heading.char_offset (bytes)
                                  chunk_offsets del chunker son char-based
         │
         ▼
chunks se asignan al topic/concept INCORRECTO para texto no-ASCII
         │
         ▼
storage persiste hierarchy equivocada en chunks
         │
         ▼
retrieval devuelve chunks con metadata de hierarchy incorrecta
```

**Archivos y líneas afectadas:**

| Crate | Archivo | Línea(s) | Variable | Tipo de bug |
|-------|---------|----------|----------|-------------|
| `graph` | `heading_parser.rs` | 17, 35 | `char_offset` | `line.len()` = bytes |
| `ingest` | `extractor.rs` | 37 | `total_chars` | `content.len()` = bytes |
| `ingest` | `sentence_chunker.rs` | 42 | `min_chars` comparison | `current_text.len()` = bytes |
| `ingest` | `sentence_chunker.rs` | 47-48, 58-59 | `offset_end` | `current_text.len()` = bytes |
| `ingest` | `validator.rs` | 97-98 | truncation | `trimmed[..255]` = byte slice → **PANIC** |

**Impacto:** Para cualquier documento con caracteres multi-byte (acentos, emoji, CJK, símbolos):
1. Los headings se asignan a offsets incorrectos → chunks caen en el topic/concept equivocado
2. Los offsets de sentence_chunker son incorrectos
3. `sanitize_display_name` con un emoji en el nombre crashea el sistema en runtime

**Por qué los tests no lo detectan:** Todos los tests usan ASCII puro donde `len() == chars().count()`.

**Fix raíz:** Cambiar `line.len()` por `line.chars().count()` en `graph/heading_parser.rs` (2 líneas). Este es el fix de mayor impacto/esfuerzo de todo el proyecto.

---

#### BUG-X2: 🟥 `check_ingest_allowed` es dead code — Production mode no bloquea ingest

**Cadena causal:**

```
engine/runtime_guard.rs:23-34     define check_ingest_allowed() con tests
         │                        (NUNCA LLAMADA)
         ▼
cli/commands/ingest.rs:64         calcula production_mode: bool
         │                        (solo para derive_display_name)
         ▼
engine/ingest.rs:41-57            recibe production_mode: bool
         │                        (solo cambia display name a genérico)
         ▼
INGEST PROCEDE SIN RESTRICCIÓN en Production/PublicPackagedDemo
```

**Impacto:** CRITICAL para compliance. Un usuario en modo `Production` puede ejecutar `cite ingest` y el documento se procesa sin restricción. El guard de seguridad es ilusorio.

**Archivo de referencia:** compliance/review.md identifica esto como incongruencia #1.

---

#### BUG-X3: 🟥 API key vacía → error críptico en runtime

**Cadena causal:**

```
config resolve_api_key              → None (no key configured)
         │
         ▼
cli/commands/mod.rs:94              unwrap_or_default() → ""
         │
         ▼
providers/gemini.rs:24 / openai.rs:29  acepta "" sin error en constructor
         │
         ▼
provider.embed("query")              → HTTP 401 "Unauthorized"
                                      (mensaje críptico, no indica causa real)
```

**Impacto:** Un usuario nuevo sin configurar API key obtiene un error HTTP que no indica la causa real. Experiencia de onboarding rota.

---

#### BUG-X4: 🟧 Rate limiting incompleto — no cumple FR-109

**Cadena causal:**

```
PRD FR-109                          → clave compuesta (4 campos)
         │
         ▼
engine/retrieve.rs:273-275          rate_limit_key() = solo provider_id()
         │
         ▼
storage/rate_limits.rs              API genérica, no valida composición
         │
         ▼
Rate limit es por provider, no por la tupla completa
(dos requests con diferente corpus_id comparten contador)
```

---

### 3.3 Bugs Per-Crate (deduplicados)

#### Storage

| # | Severidad | Bug | Archivo:Línea | Fix |
|---|-----------|-----|---------------|-----|
| S1 | CRITICAL | `PRAGMA foreign_keys=ON` nunca se ejecuta → FK constraints decorativas | `storage/lib.rs:34-40` | 1 línea |
| S2 | HIGH | `activate_snapshot` usa `.ok()` en vez de `.optional()` → errores DB silenciados | `storage/snapshots.rs:68-73` | 1 línea |
| S3 | HIGH | Casts `i64 → u32` sin verificación de overflow | `storage/util.rs:37,42,47`, `embeddings.rs:144,148,153`, `traces.rs:124,128,150` | `u32::try_from()` |
| S4 | MEDIUM | `decode_vector_blob` saltea rows corruptas sin warning | `storage/embeddings.rs:31-37, 122, 155` | Log warning o `Err` |
| S5 | MEDIUM | Rate limit counters sin TTL → acumulación indefinida | `storage/rate_limits.rs` | Cleanup periódico |
| S6 | MEDIUM | `ConceptRow`/`TopicRow` almacenan `created_at` como `String` | `storage/concepts.rs:14`, `topics.rs:14` | Parsear a `DateTime<Utc>` |

#### CLI

| # | Severidad | Bug | Archivo:Línea | Fix |
|---|-----------|-----|---------------|-----|
| C1 | HIGH | Flag `--json` duplicado en `EvaluateArgs` | `cli/evaluate.rs:17-20` | Eliminar campo |
| C2 | HIGH | `setup` hardcodea modelos en test de provider | `cli/setup.rs:215-232` | Leer de config |
| C3 | HIGH | Unwrap inconsistente del provider entre comandos | `cli/ingest.rs:66`, `context.rs:50`, `retrieve.rs:80` vs `search.rs:49` | Helper `provider()` |
| C4 | MEDIUM | Error display pattern repetido 14+ veces | Todas las `execute()` | Helper `print_error()` |

#### Engine

| # | Severidad | Bug | Archivo:Línea | Fix |
|---|-----------|-----|---------------|-----|
| E1 | HIGH | `required_facets_for_query` — falsos positivos con "e", "en" | `engine/context.rs:62-80` | Word boundary regex |
| E2 | HIGH | `cleanup_partial` ignora errores DB → datos huérfanos posibles | `engine/ingest.rs:215-220` | Propagar o loggear |
| E3 | HIGH | Snapshot refresh no es fully atomic — attaches huérfanos si activación falla | `engine/refresh.rs:34-54` | Cleanup on failure |
| E4 | MEDIUM | Golden fixtures duplicados en 4 ubicaciones con inconsistencias | `engine/tests/golden/`, `cli/evaluate.rs` | Consolidar en JSON |

#### Config

| # | Severidad | Bug | Archivo:Línea | Fix |
|---|-----------|-----|---------------|-----|
| CF1 | HIGH | `min_chunk_size_chars` vs `min_chunk_chars` confusos/semi-duplicados | `config/lib.rs:94, 126` | Consolidar a 3 campos |
| CF2 | HIGH | TOML no puede configurar la mayoría de los campos | `config/lib.rs:356-411` | Expandir `TomlRoot` |
| CF3 | MEDIUM | `load_from` retorna `Result` pero nunca falla | `config/lib.rs:167-173` | Propagar error o cambiar firma |
| CF4 | MEDIUM | Env vars con valores inválidos se ignoran silenciosamente | `config/lib.rs:298-325` | Log warning |

#### Providers

| # | Severidad | Bug | Archivo:Línea | Fix |
|---|-----------|-----|---------------|-----|
| P1 | HIGH | `embedding_timeout_secs` del config ignorado — timeout hardcoded 30s | `providers/gemini.rs:31`, `openai.rs:34` | Parámetro en constructor |
| P2 | HIGH | `tokio` y `tracing` en Cargo.toml — nunca usadas | `providers/Cargo.toml:8-9` | Eliminar |
| P3 | MEDIUM | Sin validación de `model` ni `endpoint` vacíos en constructores | `providers/gemini.rs:24`, `openai.rs:29` | Validar |

#### Graph

| # | Severidad | Bug | Archivo:Línea | Fix |
|---|-----------|-----|---------------|-----|
| G1 | HIGH | Boundary matching por título duplicado es frágil (dos headings con mismo nombre) | `graph/hierarchy.rs:128-148` | Cursor por posición |
| G2 | HIGH | Code block detection frágil (indentación, 4+ backticks) | `graph/heading_parser.rs:14-16` | Mejorar parser |
| G3 | MEDIUM | Topic sin H2 → chunks sin concepto asignado | `graph/hierarchy.rs:110-120` | Crear "Default" concept |

#### Retrieval

| # | Severidad | Bug | Archivo:Línea | Fix |
|---|-----------|-----|---------------|-----|
| R1 | HIGH | `ScoredChunk` replica campos de `ChunkEmbeddingRecord` — riesgo desalineación | `retrieval/lib.rs:40-64` | Envolver en vez de replicar |
| R2 | MEDIUM | Tests insuficientes para edge cases de cosine | `retrieval/lib.rs:197-256` | Agregar tests |

### 3.4 Issues Arquitectónicos

| # | Issue | Crates afectados | Impacto |
|---|-------|-----------------|---------|
| A1 | Config diseñada pero no consumida (4+ campos ignorados) | config, providers, engine | Usuario cree que config funciona pero no |
| A2 | Newtypes `DocumentId`/`ChunkId`/`TraceId` definidos pero nunca usados | common, todos | Type-safety perdida |
| A3 | `created_at` inconsistente: `DateTime<Utc>` en Document/Chunk, `String` en Topic/Concept/Graph | storage, graph, common | Impide comparaciones temporales |
| A4 | Tipos de datos inconsistentes para offsets: `u32` vs `usize` vs `i64` | common, ingest, graph, storage | Casts silenciosos, overflow potential |
| A5 | `Engine` struct vacía es dead code | engine/lib.rs | Confunde lectores |
| A6 | `SemanticLink` definido pero nunca usado | graph/types.rs | Dead code |
| A7 | Boolean `production_mode` tiene semántica engañosa | engine/ingest.rs | Confunde a desarrolladores |
| A8 | TOML soporta ~7 campos, env vars ~20 — asimetría significativa | config | UX rota para config-via-archivo |

### 3.5 Gaps de Testing

| Crate | Gap | Impacto |
|-------|-----|---------|
| `ingest` | 57 tests, 0 con texto multi-byte | Oculta el bug UTF-8 más crítico |
| `graph` | Tests de offsets solo con ASCII | No detecta byte-vs-char |
| `providers` | Tests dependen de red sin `#[ignore]` | Flaky en CI |
| `storage` | Sin tests de concurrencia real (dos threads) | Locks no verificados en escenario real |
| `retrieval` | Faltan tests para vectores opuestos, ortogonales, k=0, candidates vacíos | Cobertura incompleta |
| `config` | Sin tests para merge, env vars, ni archivo TOML | Lógica más compleja sin cobertura |
| `common` | 1 test para 18 variantes de CiteError | Códigos de error no verificados |

---

## Sección 4: Análisis de Coherencia Arquitectónica

### 4.1 Patrones Consistentes ✅

| Patrón | Dónde se aplica | Evaluación |
|--------|-----------------|------------|
| **Inyección de dependencia** | CLI → Engine (Database, Provider, Config) | Excelente. Engine no conoce implementaciones concretas. |
| **`CiteError` como tipo unificado** | Todos los crates | Muy bueno. 18 variantes cubren todos los dominios. `code()`, `exit_code()`, `to_json_response()` son consistentes. |
| **Transacciones en bulk inserts** | storage/chunks, storage/embeddings | Correcto. Rollback total si un elemento falla. |
| **Serialización JSON/humano dual** | CLI (todos los comandos) | Consistente. Patrón `if json { print_json } else { eprintln }`. |
| **Idempotencia en backlog** | storage/backlog | Bien diseñado. `ON CONFLICT(idempotency_key) DO UPDATE`. |
| **Snapshot atomic swap** | storage/snapshots, engine/refresh | Correcto. Transacción que supersede + activate. |
| **Defensa en profundidad en paths** | ingest/validator | Multi-layer: pre-canonical, canonical, post-canonical. |
| **serde(rename_all = "snake_case")** | Todos los enums públicos | Consistente y predecible. |

### 4.2 Patrones Inconsistentes ⚠️

| Patrón | Crates afectados | Problema |
|--------|-----------------|----------|
| **Manejo de tipos de tiempo** | common (`DateTime<Utc>`), graph/storage topics/concepts (`String`) | `created_at` parseado vs no parseado |
| **IDs como String vs newtypes** | common define newtypes, todos usan `String` | Newtypes inútiles sin migración |
| **Offets: bytes vs chars** | graph (bytes), chunker (chars), sentence_chunker (bytes) | Bug cross-crate #1 |
| **Error propagation** | storage (`.ok()`), engine (`let _ =`), cli (`unwrap_or_default()`) | Errors silenciados inconsistentemente |
| **Provider unwrap** | ingest, context, retrieve (`unwrap()`), search (`match`) | Inconsistencia dentro del mismo crate |
| **DRY violations** | cli (error pattern ×14), providers (HTTP ×2), storage (row mapping ×2) | Boilerplate repetido sin extract |
| **Timeout handling** | config define, providers hardcodean | Config desconectada de implementación |

### 4.3 Contradicciones entre Specs e Implementación

| # | Spec | Implementación | Contradicción |
|---|------|---------------|---------------|
| 1 | PRD: "runtime guard blocks ingest in Production" | `check_ingest_allowed` nunca se llama | Guard es ilusorio |
| 2 | PRD FR-109: rate limit key compuesta (4 campos) | Solo `provider_id()` | Granularidad insuficiente |
| 3 | PRD FR-015: atomic snapshot swap | Snapshots building quedan huérfanos si activate falla | Parcialmente implementado |
| 4 | Compliance: "data deletion workflow" required pre-production | `delete_document` es post-MVP | Contradicción directa |
| 5 | Compliance: "privacy notice" required | No existe template ni archivo | No implementado |
| 6 | Config: `embedding_timeout_secs` configurable | Providers hardcodean 30s | Config ignorada |
| 7 | Config: TOML como método de configuración | Solo ~7 de ~20 campos accesibles | Asimetría significativa |
| 8 | PRD: golden dataset con fixtures específicos | Fixtures duplicados con expectativas opuestas | Inconsistencia activa |

### 4.4 Patrones que se Repiten (Buenos y Malos)

**Buenos patrones que se repiten:**
- Trait-based abstraction (`EmbeddingProvider`, `Database` handle)
- `Result<T, CiteError>` como tipo de retorno universal
- Transacciones SQLite para operaciones batch
- Feature flags opt-in (`sentence_chunking`, `build_hierarchy`)

**Malos patrones que se repiten:**
- `.unwrap()` o `.unwrap_or_default()` en boundaries de crates (CLI, providers)
- `.ok()` o `let _ =` para silenciar errores genuinos (storage, engine)
- Tests solo con ASCII (ingest, graph, retrieval)
- Config definida pero no consumida (providers timeout, engine production_mode)
- Dead code no marcado como TODO (Engine struct, SemanticLink, check_ingest_allowed)

### 4.5 Filosofía de Error Handling

La filosofía **intencional** es buena: `CiteError` como tipo unificado, códigos de salida CLI mapeados, JSON response para agentes. Sin embargo, la **implementación** es inconsistente:

| Nivel | Filosofía | Realidad |
|-------|-----------|----------|
| Tipos de error | `CiteError` unificado | ✅ Consistente |
| Propagación | `?` operator | ✅ Mayoría del código |
| Error en boundaries | Validar explícitamente | ❌ `unwrap_or_default()`, `.ok()`, `let _ =` |
| Error display | Dual JSON/humano | ✅ Consistente (pero repetido 14 veces) |
| Testing de errores | Cubrir variantes | ❌ 1/18 variantes testeada |

---

## Sección 5: Priorización de Fixes

### Tier 1: Must Fix Antes de Production (7 items)

| # | Fix | Crates | Esfuerzo | Dependencias |
|---|-----|--------|----------|-------------|
| 1 | `PRAGMA foreign_keys=ON` | storage | 1 línea | Ninguna |
| 2 | heading_parser `char_offset` byte→char | graph | 2 líneas + test UTF-8 | Ninguna |
| 3 | sentence_chunker/extractor byte→char | ingest | ~6 líneas + tests | Ninguna |
| 4 | validator `sanitize_display_name` panic UTF-8 | ingest | 3 líneas + test | Ninguna |
| 5 | `check_ingest_allowed` → conectar al path de ingest | cli + engine | ~10 líneas | Ninguna |
| 6 | API key vacía → fail fast en CLI + defensa en providers | cli + providers | ~15 líneas | Ninguna |
| 7 | Duplicación golden fixtures → consolidar + resolver inconsistencias | engine + cli | ~50 líneas | Ninguna |

**Dependencias internas Tier 1:** Los fixes 2, 3, 4 son independientes entre sí. El fix 5 puede hacerse en CLI o engine (o ambos). El fix 6 requiere cambios coordinados en cli + providers.

### Tier 2: Should Fix Soon (10 items)

| # | Fix | Crates | Esfuerzo | Dependencias |
|---|-----|--------|----------|-------------|
| 8 | Rate limit key compuesta FR-109 | engine + storage | ~10 líneas | Ninguna |
| 9 | `activate_snapshot` `.ok()` → `.optional()` | storage | 1 línea | Ninguna |
| 10 | Timeout config → providers | providers + cli | ~10 líneas | Ninguna |
| 11 | Casts `i64→u32` sin overflow check | storage | ~20 líneas | Ninguna |
| 12 | Boundary matching por título → por posición | graph | ~15 líneas | Depende de #2 |
| 13 | `required_facets_for_query` falsos positivos | engine | ~10 líneas | Ninguna |
| 14 | `cleanup_partial` → propagar errores | engine | ~10 líneas | Ninguna |
| 15 | Snapshot refresh → cleanup on activation failure | engine | ~15 líneas | Ninguna |
| 16 | Config: consolidar `min_chunk_size_chars`/`min_chunk_chars` | config + ingest | ~20 líneas | Antes de Tier 1 #3 |
| 17 | Config: expandir TOML para soportar todos los campos | config | ~30 líneas | Ninguna |

**Dependencias internas Tier 2:** El fix 16 (config) debe resolverse ANTES o junto con el fix 3 (sentence_chunker) para no agravar la confusión de campos. El fix 12 (graph) depende de que el fix 2 ya esté aplicado.

### Tier 3: Tech Debt / Nice to Have (14 items)

| # | Fix | Crates | Categoría |
|---|-----|--------|-----------|
| 18 | DRY: `print_error` helper en CLI | cli | Code smell |
| 19 | DRY: `send_and_parse` helper en providers | providers | Code smell |
| 20 | DRY: wrapper para `list_ready_chunk_embeddings` | storage | Code smell |
| 21 | Migración newtypes `DocumentId`/`ChunkId`/`TraceId` | common + todos | Type-safety |
| 22 | `created_at` → `DateTime<Utc>` en Topic/Concept/Graph | storage + graph | Consistencia |
| 23 | Eliminar deps muertas: `tokio`, `tracing` en providers | providers | Build time |
| 24 | Eliminar dead code: `Engine` struct, `SemanticLink`, `into_compact_*` | engine, graph, cli | Limpieza |
| 25 | `CommandContext` → `Result<Self, CiteError>` en vez de `i32` | cli | Ergonomía |
| 26 | Agregar `#[non_exhaustive]` a enums públicos | common | API stability |
| 27 | Agregar `PartialEq` a `CiteError`, `Document`, `ErrorInfo` | common | Testing |
| 28 | Tests UTF-8 como estándar en ingest, graph, retrieval | ingest, graph, retrieval | Testing |
| 29 | Tests de concurrencia real en storage | storage | Testing |
| 30 | Tests de config merge/env vars/TOML | config | Testing |
| 31 | `EvalProvider` → keywords más específicas | providers | Test quality |

### Diagrama de Dependencias entre Fixes

```
Tier 1 (todos independientes excepto):
  #16 (config consolidar campos) ──debe ir antes de──→ #3 (sentence_chunker byte→char)
  #2 (heading_parser) ──debe ir antes de──→ #12 (boundary matching)

Tier 1 → Tier 2:
  #7 (golden fixtures) ──habilita──→ tests fiables para otros fixes

Secuencia recomendada:
  1. #1 (FK pragma) — 1 línea, riesgo cero
  2. #4 (validator panic) — 3 líneas, impacto visible inmediato
  3. #2 + #3 (byte→char UTF-8) — el cambio más importante del proyecto
  4. #5 (check_ingest_allowed) — compliance blocker
  5. #6 (API key validation) — UX blocker
  6. #7 (golden fixtures) — testing blocker
```

---

## Sección 6: Recomendaciones

### 6.1 Mejoras Arquitectónicas

#### R1: Adoptar "Parse, Don't Validate" en Boundaries de Crates

Crear newtypes con validación en el constructor para los tipos que cruzan boundaries:

```rust
// En common o en el crate que define el boundary
pub struct ApiKey(String);
impl ApiKey {
    pub fn new(key: String) -> Result<Self, CiteError> {
        if key.is_empty() {
            return Err(CiteError::ConfigError {
                message: "API key cannot be empty".into(),
            });
        }
        Ok(Self(key))
    }
}

pub struct NonEmptyString(String);
// Similar para model, endpoint, etc.
```

**Impacto:** Elimina toda la categoría de bugs de "string vacío que cruza boundaries sin validación".

#### R2: Helper de Offsets UTF-8 en Common

```rust
// common/src/utf8.rs
pub fn char_len(s: &str) -> usize { s.chars().count() }
pub fn char_truncate(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}
```

Y agregar un lint CI:
```bash
grep -rn '\.len()' crates/*/src/ | grep -i 'offset\|char\|count\|position'
```

**Impacto:** Prevención sistémica de la confusión bytes vs caracteres.

#### R3: Config como Única Fuente de Verdad para Providers

Los providers deberían recibir el timeout desde el constructor, no hardcodearlo. El pattern actual de "config define, provider ignora" debe eliminarse:

```rust
// CLI create_provider
let provider = match provider_type {
    "gemini" => GeminiProvider::new(model, &api_key, config.ingest.embedding_timeout_secs)?,
    _ => OpenAICompatibleProvider::new(endpoint, model, &api_key, config.ingest.embedding_timeout_secs)?,
};
```

#### R4: Consolidar Config de Chunking

Los 4+ campos de chunking confusos deben reducirse a 3 claros:

```rust
pub struct IngestConfig {
    pub target_chunk_chars: usize,   // default: 1000 (reemplaza chunk_size_chars)
    pub min_chunk_chars: usize,      // default: 100 (consolida min_chunk_size_chars + min_chunk_chars)
    pub max_chunk_chars: usize,      // default: 1500 (debe ser > target, no 200 como ahora)
    // ... resto de campos sin cambio
}
```

#### R5: Refrescar Snapshot con Rollback Explanícito

```rust
match db.activate_snapshot(&snapshot_id) {
    Ok(result) => Ok(RefreshResult { ... }),
    Err(e) => {
        let _ = db.mark_snapshot_failed(&snapshot_id);
        Err(e)
    }
}
```

### 6.2 Mejoras de Testing

#### T1: Estándar UTF-8 para Todos los Tests de Texto

Regla: todo test que verifique offsets, truncación, o conteo de caracteres DEBE incluir texto con:
- Acentos: `café`, `résumé`
- Emoji: `🎉🎊`
- CJK (opcional): `テスト`

Esto se puede enforcear con un grep en CI:
```bash
# Buscar tests de ingest/graph que NO usen UTF-8
grep -L 'emoji\|café\|🎉\|résumé' crates/ingest/src/*.rs crates/graph/src/*.rs
```

#### T2: Tests de Providers sin Red

Marcar con `#[ignore]` los tests que requieren acceso a red:
```rust
#[test]
#[ignore = "requires network access to Gemini API"]
fn test_embed_invalid_key_returns_error() { ... }
```

#### T3: Tests de Config Merge

Los tests de config deben cubrir la lógica de merge (la parte más compleja del crate):
- Defaults correctos
- TOML override defaults
- Env vars override TOML
- CLI flags override todo
- Valores inválidos en env vars

#### T4: Tests de Errores Estables

```rust
#[test]
fn test_error_codes_stable() {
    // Snapshot: si estos cambian, es breaking change
    assert_eq!(
        CiteError::DocumentNotFound { document_id: "x".into() }.code(),
        "document_not_found"
    );
    // ... para las 18 variantes
}
```

#### T5: Golden Fixtures como Fuente Única

Consolidar los fixtures en `fixtures.json` y que tanto CLI como tests carguen desde ahí. Resolver las inconsistencias de expectativas (amb-001 y pi-001 tienen expectativas opuestas).

### 6.3 Mejoras de Proceso

#### P1: Checklist Pre-Production

Antes de habilitar modo Production, verificar:

- [ ] `PRAGMA foreign_keys=ON` activado
- [ ] `check_ingest_allowed` conectado y testeado
- [ ] Todos los offsets usan chars, no bytes
- [ ] `sanitize_display_name` no panic con UTF-8
- [ ] API key vacía produce error claro
- [ ] Golden fixtures consistentes entre CLI y tests
- [ ] Rate limit key cumple FR-109
- [ ] Privacy notice existe (aunque sea placeholder)
- [ ] Data deletion workflow definido (reset manual o API)

#### P2: CI Lint para Confusión bytes/chars

Agregar un check en CI que detecte usos sospechosos de `.len()` en contextos de offsets:

```bash
# En CI script
echo "Checking for potential byte-vs-char confusion..."
grep -rn '\.len()' crates/*/src/ | grep -v 'test' | grep -v '//' | grep -iE 'offset|char|count|position|truncat' && \
  echo "⚠ Potential byte-vs-char confusion found. Review above lines." && exit 1
```

#### P3: Documentar Decisiones de Diseño

Las decisiones de diseño importantes deben documentarse en el código:
- ¿Por qué `ScoredChunk` replica campos en vez de envolver `ChunkEmbeddingRecord`?
- ¿Por qué `created_at` es `String` en Topic pero `DateTime<Utc>` en Document?
- ¿Por qué `check_ingest_allowed` no se llama? (o documentar que es un bug)

#### P4: Roadmap de Migración de Newtypes

Si se decide migrar `DocumentId`/`ChunkId`/`TraceId`:
1. Fase 1: `storage/documents.rs` → `DocumentId` en signatures
2. Fase 2: `engine/ingest.rs`, `engine/context.rs`
3. Fase 3: `cli/commands/*`
4. Fase 4: Resto de crates

Hacerlo incrementalmente, un crate a la vez, con compilación limpia entre fases.

---

## Anexo: Tabla Cross-Referencia de Errores Relacionados

| Error | Relacionado con | Tipo de relación |
|-------|----------------|-----------------|
| graph/heading_parser byte offset (G-CRITICAL) | ingest/sentence_chunker byte offset | Misma causa raíz (confusión len()) |
| ingest/validator panic UTF-8 | graph/heading_parser byte offset | Misma causa raíz |
| cli/API key vacía (CLI-H2) | providers/sin validación key vacía (P-CRITICAL) | Cross-crate: caller + callee |
| engine/check_ingest_allowed dead code (E-CRITICAL) | cli/check_ingest_allowed dead code (CLI-CRITICAL) | Misma observación en ambos crates |
| config/embedding_timeout_secs (P-HIGH) | config/no consumida (A1) | Patrón sistémico |
| config/min_chunk_size_chars vs min_chunk_chars (CF-H1) | ingest/sentence_chunker no usa max_chunk_chars | Config confusa → uso incorrecto |
| storage/activate_snapshot .ok() (S-HIGH) | storage/errores silenciados | Patrón sistémico |
| engine/cleanup_partial ignora errores (E-H2) | storage/errores silenciados | Patrón sistémico |
| engine/golden fixtures inconsistentes (E-M1) | engine/GoldenProvider duplicado (E-M2) | Misma feature, doble problema |
| common/newtypes no usados (C-CRITICAL) | common/re-exports incompletos (C-M1) | Newtypes inútiles sin re-export |

---

*Documento generado: 2026-06-02*
*Fuentes: 18 reportes individuales (9 review.md + 9 errores.md), 1 cross-crate-review.md, 1 compliance/review.md, 1 items-pendientes.md*
*Total de errores analizados: 95 (bruto) → 51 (deduplicados)*
*Estado: LISTO para priorizar fixes*
