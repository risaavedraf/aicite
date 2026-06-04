# Revisión Cross-Crate — Análisis transversal de errores

**Revisor:** el Gentleman (análisis inline)
**Fecha:** 2026-06-02
**Alcance:** 51 errores encontrados en 9 crates (cli, common, config, engine, storage, ingest, providers, retrieval, graph)
**Metodología:** Síntesis de todos los `errores.md` individuales + lectura de código para validar conexiones cross-crate

---

## 1. Patrones sistémicos

### 1.1 — 🟥 BUG SISTÉMICO: Confusión bytes vs caracteres UTF-8

**El hallazgo más importante de toda la revisión.** Afecta 4 de 6 archivos del crate `ingest` y el crate `graph` completo. Todos los tests usan ASCII puro, así que ningún test lo detecta.

| Crate | Archivo | Línea(s) | Variable afectada | Usado por |
|-------|---------|----------|-------------------|-----------|
| `graph` | `heading_parser.rs` | 17, 35 | `char_offset` | `ingest/lib.rs:161` (hierarchy assignment) |
| `ingest` | `extractor.rs` | 37 | `total_chars` | Metadata (bajo impacto hoy) |
| `ingest` | `sentence_chunker.rs` | 42 | `min_chars` comparison | Chunk merging logic |
| `ingest` | `sentence_chunker.rs` | 47-48, 58-59 | `offset_end` | `ChunkInput.offset_start/end` |
| `ingest` | `validator.rs` | 97-98 | display name truncation | **PANIC en runtime** |
| `storage` | `util.rs`, `embeddings.rs`, etc. | múltiple | `i64 as u32` casts | Data corruption silenciosa |

**Impacto cross-crate:**
```
graph/heading_parser (byte offsets)
    → ingest/lib.rs (compare con char offsets del chunker)
    → chunks se asignan al topic/concept INCORRECTO
    → retrieval devuelve chunks con metadata de hierarchy equivocada
```

**Patrón del bug:** `str::len()` retorna bytes UTF-8, no caracteres. En Rust, la confusión es fácil porque `len()` existe en `str` y `String` y retorna bytes. El correcto para offsets de texto es `.chars().count()`.

**Fix global recomendado:** Crear un lint o helper `fn char_len(s: &str) -> usize { s.chars().count() }` en `common` y banear `str::len()` en contextos de offset/character counting con un clippy custom lint o grep en CI.

---

### 1.2 — 🟥 Patrón: Config que existe pero no se consume

| Config field | Definido en | Ignorado por | Efecto |
|-------------|-------------|--------------|--------|
| `embedding_timeout_secs` | `config/lib.rs:101` | `providers/gemini.rs:31`, `openai.rs:34` | Timeout hardcoded a 30s |
| `production_mode` flag | `cli/ingest.rs:64` | `engine/ingest.rs:41-57` | Production no bloquea ingest |
| `max_chunk_chars` | `config/lib.rs` | `ingest/sentence_chunker.rs` | Chunks sin límite de tamaño |
| Rate limit composite key | FR-109 spec | `engine/retrieve.rs:273-275` | Solo `provider_id()` |

**Patrón arquitectónico:** Hay una brecha entre la capa de configuración (que define fields bien pensados) y la capa de consumo (que no los lee). Esto sugiere que la config se diseñó con la intención correcta pero la implementación no la alcanzó. Es un patrón de "design ahead of implementation" que es común en proyectos en desarrollo, pero peligroso en producción porque el usuario cree que la config funciona.

**Recomendación:** Auditar TODOS los fields de `AppConfig` y verificar cuáles se consumen realmente. Los que no se consumen deberían tener un `// TODO: not yet consumed` o eliminarse.

---

### 1.3 — 🟥 Patrón: Validación defensiva ausente en boundaries de crates

| Boundary | De | A | Qué falta |
|----------|----|---|-----------|
| API key vacía | CLI | Providers | `unwrap_or_default()` → `""` → HTTP 401 críptico |
| Provider None | CLI | Engine | `unwrap()` en 3 comandos, `match` en 1 |
| Empty model string | Config | Providers | `""` genera endpoint malformado |
| Empty endpoint | Config | OpenAI provider | `""` pasa validación HTTPS |

**Patrón:** Los crates asumen que los datos que reciben de sus callers son válidos. No hay validación en los constructores ni en los boundaries. Esto es un anti-pattern porque viola el principio de "fail fast" — el error se manifiesta lejos de su causa real (HTTP 401 en vez de "no API key").

**Recomendación:** Adoptar "parse, don't validate" en los constructores públicos de cada crate. Si un tipo no puede representar un estado inválido (ej: `ApiKey` newtype con validación en el constructor), los bugs de este tipo desaparecen compile-time o en el boundary.

---

### 1.4 — 🟧 Patrón: Errores silenciados con `.ok()`, `unwrap_or_default()`, `continue`

| Crate | Archivo | Patrón | Efecto |
|-------|---------|--------|--------|
| `storage` | `snapshots.rs:68-73` | `.ok()` en query | DB errors → `None` |
| `storage` | `embeddings.rs:122,155` | `continue` on `None` | BLOBs corruptos invisibles |
| `cli` | `commands/mod.rs:94` | `unwrap_or_default()` | API key vacía silenciosa |
| `cli` | `setup.rs` | `unwrap_or_default()` | TTY errors silenciados |
| `ingest` | `sentence_chunker.rs` | offset drift por trimming | Offsets aproximados |

**Patrón:** Errors se convierten a `None` o se ignoran en vez de propagarse. En Rust, el idioma correcto es `.optional()` (de `rusqlite`) para `QueryReturnedNoRows`, y `.map_err(...)?` para errores genuinos.

---

### 1.5 — 🟧 Patrón: DRY violations en error handling

- **CLI:** El bloque `if json { print_json } else { eprintln } + return exit_code` se repite 14+ veces.
- **Providers:** HTTP send + error parsing + JSON deserialization duplicado entre Gemini y OpenAI (~30 líneas cada uno).
- **Storage:** `list_chunk_embeddings_hierarchical` vs `list_ready_chunk_embeddings` duplican 60 líneas de row mapping.

**Recomendación:** Extract helpers en cada crate. El CLI necesita `print_error()`, providers necesita `send_and_parse()`, storage necesita el wrapper para `list_ready_chunk_embeddings`.

---

## 2. Bugs cross-crate (cadenas de causalidad)

### 2.1 — 🔴 Cadena: Hierarchy assignment incorrecta para texto no-ASCII

```
graph/heading_parser.rs:17,35  →  char_offset en bytes
    ↓
ingest/lib.rs:161              →  compara byte offsets con char offsets
    ↓
chunks asignados al topic/concept incorrecto
    ↓
storage persiste hierarchy equivocada
    ↓
retrieval devuelve chunks con metadata de hierarchy incorrecta
```

**Fix:** `graph/heading_parser.rs` es la causa raíz. Fixear ahí resuelve toda la cadena.

### 2.2 — 🔴 Cadena: Production mode no bloquea ingest

```
cli/commands/ingest.rs:64      →  calcula production_mode pero no llama al guard
    ↓
engine/ingest.rs:41-57         →  recibe production_mode pero solo para display_name
    ↓
runtime_guard::check_ingest_allowed  →  NUNCA SE LLAMA (dead code)
    ↓
storage persiste documento en production mode sin compliance check
```

**Fix:** Agregar guard call en CLI ingest Y en engine ingest (defensa en profundidad).

### 2.3 — 🔴 Cadena: API key vacía → error críptico

```
config resolve_api_key         →  None (no key configured)
    ↓
cli/commands/mod.rs:94         →  unwrap_or_default() → ""
    ↓
providers/gemini.rs:24         →  acepta "" sin error
    ↓
providers/gemini.rs embed()    →  HTTP 401 "Unauthorized" (no dice por qué)
```

**Fix:** Validar en CLI (fail fast) + validar en providers (defensa en profundidad).

### 2.4 — 🟧 Cadena: Rate limiting incompleto

```
FR-109 spec                    →  clave compuesta (4 campos)
    ↓
engine/retrieve.rs:273-275     →  solo usa provider_id()
    ↓
storage/rate_limits.rs         →  API genérica, no valida composición
    ↓
rate limit es por provider, no por la tupla completa
```

**Fix:** El bug es del caller (engine). Storage debería proveer helper o documentar el requisito.

---

## 3. Incoherencias arquitectónicas

### 3.1 — 🟧 Integridad referencial declarada pero no enforced

El schema SQL de storage define 6+ foreign key constraints (`REFERENCES`), pero `PRAGMA foreign_keys=ON` nunca se ejecuta. Esto significa:
- La DB acepta datos inconsistentes (chunks sin documento, embeddings sin chunk)
- La integridad depende 100% de la lógica de aplicación
- Los tests de storage no detectan inconsistencias porque usan `Database::open_memory()` que tampoco habilita FK

**Impacto:** Si un bug en engine o ingest inserta datos en orden incorrecto o deja datos huérfanos, la DB no lo detecta.

### 3.2 — 🟧 Tipos de datos inconsistentes entre crates

| Tipo | `common::types` | `storage` | `graph` | `retrieval` |
|------|-----------------|-----------|---------|-------------|
| `created_at` | `DateTime<Utc>` | `DateTime<Utc>` (Document, Chunk), `String` (Topic, Concept) | `String` | N/A |
| Offsets | `u32` (ChunkInput) | `u32` (DB columns) | `usize` (char_offset) | N/A |
| Status enum | `DocumentStatus` | String en DB, sin CHECK | N/A | N/A |

**Incoherencia:** `created_at` se parsea a `DateTime<Utc>` en `Document` y `Chunk` pero queda como `String` en `TopicRow`, `ConceptRow`, y los tipos de `graph`. Esto impide comparaciones temporales tipadas entre entidades.

### 3.3 — 🟧 Los tests no reflejan la realidad del dominio

**Todos los tests usan ASCII puro.** Esto oculta el bug más importante del proyecto (byte vs char). Los tests de providers dependen de red. Los tests de storage no testean concurrencia real.

| Crate | Gap de testing |
|-------|---------------|
| `ingest` | 57 tests, 0 con texto multi-byte |
| `graph` | Tests de offsets solo con ASCII |
| `providers` | Tests dependen de red sin `#[ignore]` |
| `storage` | Sin tests de concurrencia real (dos threads) |
| `retrieval` | Sin tests para edge cases de cosine |

### 3.4 — 🟧 Dead code y forward declarations sin marcar

| Item | Crate | Estado |
|------|-------|--------|
| `check_ingest_allowed` | engine | Dead code (existe pero no se llama) |
| `SemanticLink` type | graph | Definido pero nunca usado |
| `Graph` unit struct | graph | Sin métodos ni estado |
| `into_compact_*` functions | CLI | `#[allow(dead_code)]` explícito |
| `tokio`, `tracing` deps | providers | En Cargo.toml pero no importados |

---

## 4. Priorización de fixes

### Tier 1 — Fixear antes de production (5 items)

| # | Bug | Crates afectados | Esfuerzo |
|---|-----|-----------------|----------|
| 1 | `PRAGMA foreign_keys=ON` | storage | 1 línea |
| 2 | heading_parser byte-vs-char offset | graph → ingest | 2 líneas + test |
| 3 | sentence_chunker/extractor byte-vs-char | ingest | ~6 líneas + tests |
| 4 | validator UTF-8 panic | ingest | 3 líneas + test |
| 5 | `check_ingest_allowed` dead code | cli + engine | ~10 líneas |

### Tier 2 — Fixear pronto (6 items)

| # | Bug | Crates afectados | Esfuerzo |
|---|-----|-----------------|----------|
| 6 | Empty API key validation | cli + providers | ~15 líneas |
| 7 | Rate limit composite key | engine → storage | ~10 líneas |
| 8 | `activate_snapshot` `.ok()` → `.optional()` | storage | 1 línea |
| 9 | Timeout hardcoded vs config | providers + cli | ~10 líneas |
| 10 | i64→u32 casts sin overflow check | storage | ~20 líneas |
| 11 | Duplicate heading title boundary | graph | ~15 líneas |

### Tier 3 — Deuda técnica (resto)

DRY violations, tests adicionales, dead code cleanup, deps no usadas, code block detection, etc.

---

## 5. Recomendaciones arquitectónicas

### 5.1 — Adoptar "parse, don't validate" en boundaries de crates

Crear newtypes con validación en el constructor:
```rust
pub struct ApiKey(String);
impl ApiKey {
    pub fn new(key: String) -> Result<Self, CiteError> {
        if key.is_empty() {
            return Err(CiteError::ConfigError { message: "API key cannot be empty".into() });
        }
        Ok(Self(key))
    }
}
```

### 5.2 — Crear lint CI para `str::len()` en contextos de offset

```bash
# grep en CI para detectar .len() usado como character count
grep -rn '\.len()' crates/*/src/ | grep -i 'offset\|char\|count\|position'
```

### 5.3 — Agregar tests con texto UTF-8 multi-byte como estándar

Todo test que verifique offsets, truncación, o conteo de caracteres DEBE incluir texto con acentos, emoji, o CJK.

### 5.4 — Auditar consumo de config

Verificar que cada field de `AppConfig` se consume en al menos un punto del código. Los no-consumidos deberían tener `// TODO` o eliminarse.

---

*Documento generado: 2026-06-02*
