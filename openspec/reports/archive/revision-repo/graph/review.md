# Revisión: `crates/graph` — Jerarquía de tópicos/conceptos y parsing de headings

**Revisor:** el Gentleman subagent de revisión
**Fecha:** 2026-06-02
**Alcance:** 4 archivos en crates/graph/src/
**Metodología:** Inspección manual de todos los archivos fuente, trazado de flujo de datos con el crate `ingest`

---

## Resumen del Crate

El crate `graph` provee la representación de jerarquía semántica de documentos: tópicos (H2), conceptos (H3) y sus vínculos con chunks de texto. Es un crate de **dominio puro** — no tiene dependencias de base de datos, I/O ni networking. Sus dependencias son `chrono`, `serde` y `common` (tipos compartidos).

El crate expone:
- **Parsing de headings markdown** (`heading_parser`): extrae headings con su offset posicional.
- **Construcción de jerarquía** (`hierarchy`): asigna chunks a tópicos/conceptos según sus offsets.
- **Tipos de dominio** (`types`): `Topic`, `Concept`, `HeadingSpan`, `SemanticLink`.
- **Contenedor `Graph`**: unit struct placeholder sin funcionalidad.

El único consumidor directo es `ingest/src/lib.rs`, que orquesta la extracción de headings → construcción de jerarquía → persistencia en SQLite.

---

## Flujo Principal

El flujo completo de datos a través del crate (y su integración con `ingest`) es:

```
1. Markdown raw text
       │
       ▼
2. heading_parser::extract_headings(text)
   → Vec<HeadingSpan> { level, title, char_offset }
   (ignora headings dentro de bloques de código ```)
       │
       ▼
3. ingest recibe chunk_offsets desde el chunker
   (char-based offsets del fixed-size chunker o sentence chunker)
       │
       ▼
4. hierarchy::build_hierarchy(document_id, headings, chunk_offsets)
   → HierarchyResult { topics: Vec<TopicWithConcepts> }
   • H2 → Topic
   • H3 → Concept (anidado en Topic actual)
   • Chunks asignados por comparación de offsets contra boundaries
       │
       ▼
5. ingest persiste topics, concepts y asigna chunk_hierarchy en SQLite
   (incluye segunda pasada para chunks sin concepto)
```

Paso clave: la función `build_hierarchy` construye una lista ordenada de `boundaries` (offset, topic_idx, Option<concept_idx>), ordena por offset, y hace una pasada única sobre los chunk_offsets asignando cada chunk al topic/concept vigente según el cursor de boundaries.

---

## Módulos/Archivos Clave

### `types.rs` (40 líneas)

Define los 4 tipos de dominio del crate:

| Tipo | Derive | Uso |
|------|--------|-----|
| `Topic` | Debug, Clone, Serialize, Deserialize | Nodo H2 en la jerarquía. Campos: topic_id, document_id, name, summary?, embedding?, chunk_count, created_at |
| `Concept` | Debug, Clone, Serialize, Deserialize | Nodo H3, hijo de Topic. Mismos campos + topic_id FK |
| `SemanticLink` | Debug, Clone, Serialize, Deserialize | Enlace semántico entre chunks (source, target, similarity_score, link_type). **No utilizado** en ninguna función del crate |
| `HeadingSpan` | Debug, Clone | Heading extraído: level, title, char_offset. **Sin Serialize/Deserialize** |

Observaciones:
- `created_at` es `String` (formateado con `"%Y-%m-%d %H:%M:%S"`), no `DateTime<Utc>`. Inconsistente con `common::types::Chunk` que usa `DateTime<Utc>`.
- `SemanticLink` es código muerto — exportado pero nunca instanciado.

### `heading_parser.rs` (110 líneas)

Función principal: `extract_headings(markdown: &str) -> Vec<HeadingSpan>`

Lógica:
- Itera líneas con `markdown.lines()`
- Toggle de bloque de código con `trimmed.starts_with("```")`
- Para cada línea no en bloque de código que empiece con `#`: cuenta nivel, extrae título, registra offset
- `char_offset` se acumula con `line.len() + 1` (bytes + newline)

Tests: 5 tests cubren caso básico, sin headings, headings en code blocks, headings vacíos, y offsets.

### `hierarchy.rs` (280 líneas)

Función principal: `build_hierarchy(document_id, headings, chunk_offsets) -> HierarchyResult`

Lógica en 3 fases:

1. **Construcción de nodos**: Recorre headings. H2 crea Topic, H3 crea Concept dentro del último Topic. H1, H4+ ignorados.
2. **Fallbacks**: Sin headings → "Untitled" topic con "Default" concept. Sin H2 → topic con nombre del primer heading, sin conceptos.
3. **Asignación de chunks**: Construye boundaries (offset → topic/concept), ordena, hace cursor lineal sobre chunk_offsets.

Tipos auxiliares:
- `TopicWithConcepts` — Topic + Vec<ConceptWithChunks>
- `ConceptWithChunks` — Concept + Vec<usize> (chunk indices)
- `HierarchyResult` — Vec<TopicWithConcepts>

Tests: 6 tests cubren sin headings, H2→topics, H3→concepts, asignación por offset, vacíos, unicidad de IDs.

### `lib.rs` (30 líneas)

Module declarations, re-exports públicos, y `Graph` unit struct placeholder.

---

## Decisiones de Diseño

### 1. Jerarquía estrictamente de 2 niveles (H2/H3)

Solo H2 y H3 participan en la jerarquía. H1 se ignora (probablemente es el título del documento), H4+ se ignora. Esto simplifica el modelo pero limita la granularidad para documentos con estructura profunda.

**Tradeoff**: Simplicidad vs. expresividad. Decisión razonable para la mayoría de documentos técnicos.

### 2. Asignación de chunks por cursor de offsets lineal

El algoritmo de asignación usa un patrón de **two-pointer**: boundaries ordenados + chunk_offsets ordenados, cursor lineal. Complejidad O(n + m).

**Ventaja**: Eficiente y simple.
**Riesgo**: Si los chunk_offsets no están estrictamente ordenados, el cursor avanza incorrectamente. No hay validación de orden.

### 3. Matching de boundaries por título (string equality)

Los boundaries se construyen buscando headings por `h.title == topic.name`. Esto es **frágil**: si dos headings tienen el mismo título, `find()` siempre devuelve el primero, causando asignaciones incorrectas en el segundo.

### 4. Offset tracking como `usize` genérico

`HeadingSpan.char_offset` y `chunk_offsets` son `usize` sin metadata de si representan bytes o caracteres. Esta ambigüedad es la raíz del bug CRITICAL #1.

### 5. `created_at` como String

Se formatea con `Utc::now().format(...)` en cada creación. Esto impide comparaciones temporales tipadas y difiere del patrón `DateTime<Utc>` usado en `common::types::Chunk`.

### 6. `SemanticLink` como forward declaration

Definido pero sin uso. Sugiere planificación para una feature futura de linking semántico (posiblemente basada en embeddings), pero actualmente es deuda técnica.

---

## Conexiones con Otros Crates

| Crate | Relación |
|-------|----------|
| `ingest` | **Consumidor directo**. Llama a `extract_headings()` y `build_hierarchy()`. Pasa `chunk_offsets` desde el chunker. Persiste resultados en storage. |
| `common` | **Dependencia de tipos**. `graph` importa `common` (probablemente para tipos compartidos, no usado directamente en los 4 archivos revisados más allá de la dependencia en Cargo.toml). |
| `config` | **Indirecta**. `config.ingest.build_hierarchy` controla si el flujo de `graph` se ejecuta. `graph` no depende de `config`. |
| `storage` | **Indirecta**. `graph` produce datos; `ingest` los persiste via `storage`. `graph` no conoce `storage`. |
| `engine` | **Sin conexión directa**. El engine consulta la jerarquía a través de storage, no a través del crate `graph`. |

Diagrama de dependencias:

```
config ──► ingest ──► graph
                │        ▲
                ▼        │
             storage    common
                │
                ▼
             engine
```

El crate `graph` es un nodo hoja en el grafo de dependencias — no tiene dependientes downstream excepto `ingest`.
