# Revisión: `crates/config` — Sistema de Configuración

## Resumen del Crate

**Propósito:** Provee la estructura `Config` central que configura todo el runtime de aiharness: modo de ejecución, paths, embedding provider, retrieval, rate limiting e ingest pipeline.

**Estructura:** Un único archivo `src/lib.rs` (~420 líneas) que contiene todos los tipos, lógica de carga, y tests.

**Dependencias:**
- `common` — usa `CiteError` como tipo de error (aunque nunca lo retorna en la práctica)
- `serde` — derive Serialize/Deserialize para todos los tipos
- `toml` — parseo del archivo de configuración TOML
- `dirs` — resolución del path de config usando XDG (`dirs::config_dir()`)

**Consumidores:**
- `cli` — carga `Config`, aplica overrides CLI, usa todos los sub-structs
- `engine` — usa `RateLimitConfig`, `RetrievalConfig`, `IngestConfig`, `RuntimeMode`
- `ingest` — usa `IngestConfig`

---

## Estructura de Config

### `Config` (top-level)
| Campo | Tipo | Default |
|-------|------|---------|
| `runtime` | `RuntimeConfig` | `LocalPrivateDemo` |
| `paths` | `PathsConfig` | `data_dir: None, cache_dir: None` |
| `embedding` | `EmbeddingConfig` | provider: `"openai-compatible"`, model: `"text-embedding-3-small"` |
| `retrieval` | `RetrievalConfig` | top_k: 5, evidence_floor: 0.50, confidence_threshold: 0.70, use_hierarchy: true |
| `rate_limit` | `RateLimitConfig` | max_requests: 20, window_seconds: 60 |
| `ingest` | `IngestConfig` | ver tabla abajo |

### `RuntimeConfig`
| Campo | Tipo | Default |
|-------|------|---------|
| `mode` | `RuntimeMode` | `LocalPrivateDemo` |

### `PathsConfig`
| Campo | Tipo | Default |
|-------|------|---------|
| `data_dir` | `Option<PathBuf>` | `None` |
| `cache_dir` | `Option<PathBuf>` | `None` |

### `EmbeddingConfig`
| Campo | Tipo | Default | Nota |
|-------|------|---------|------|
| `provider` | `String` | `"openai-compatible"` | |
| `model` | `String` | `"text-embedding-3-small"` | |
| `api_key` | `Option<String>` | `None` | `#[serde(default)]` |

### `RetrievalConfig`
| Campo | Tipo | Default | Nota |
|-------|------|---------|------|
| `top_k` | `u32` | `5` | |
| `evidence_floor` | `f64` | `0.50` | |
| `confidence_threshold` | `f64` | `0.70` | |
| `use_hierarchy` | `bool` | `true` | `#[serde(default = "default_use_hierarchy")]` |

### `RateLimitConfig`
| Campo | Tipo | Default |
|-------|------|---------|
| `max_requests` | `u32` | `20` |
| `window_seconds` | `u32` | `60` |

### `IngestConfig`
| Campo | Tipo | Default | Nota |
|-------|------|---------|------|
| `max_file_size_bytes` | `u64` | `52_428_800` (50MB) | |
| `chunk_size_chars` | `usize` | `1000` | Tamaño objetivo del chunk |
| `chunk_overlap_chars` | `usize` | `200` | |
| `min_chunk_size_chars` | `usize` | `100` | **Sin `#[serde(default)]`** |
| `max_retry_count` | `u32` | `3` | |
| `embedding_timeout_secs` | `u64` | `30` | |
| `embedding_endpoint` | `Option<String>` | `None` | |
| `sentence_chunking` | `bool` | `false` | `#[serde(default)]` |
| `min_chunk_chars` | `usize` | `30` | `#[serde(default)]` con fn |
| `max_chunk_chars` | `usize` | `200` | `#[serde(default)]` con fn |
| `build_hierarchy` | `bool` | `false` | `#[serde(default)]` |

**Implementa `Default`:** Solo `IngestConfig` tiene `impl Default`. Los demás sub-structs no; sus defaults están hardcodeados en `Config::defaults()`.

---

## Carga de Configuración

### Punto de entrada

```rust
Config::load() -> Result<Self, CiteError>
// Lee CITE_CONFIG env var como path, delega a load_from()
```

### `load_from(config_path: Option<&Path>)`

```
defaults() → file_config → env_overrides → merge → Config
```

### Precedencia (documentada en código: `flags > env > file > defaults`)

1. **CLI flags** — aplicados por `cli::apply_cli_overrides()` **después** de `load_from()`
2. **Variables de entorno** (`CITE_*`) — segunda prioridad dentro de `load_from()`
3. **Archivo TOML** — tercera prioridad
4. **Defaults hardcodeados** — base

### Archivo TOML

Path por defecto: `dirs::config_dir() / "cite" / "config.toml"` (override con `CITE_CONFIG` env var).

Estructura TOML reconocida:
```toml
[provider]
type = "..."
api_key = "..."
model = "..."

[retrieval]
top_k = 5
evidence_floor = 0.50
confidence_threshold = 0.70

[data]
dir = "/path/to/data"
```

### Variables de entorno

| Variable | Campo | Tipo parseado |
|----------|-------|---------------|
| `CITE_RUNTIME_MODE` | `runtime.mode` | `RuntimeMode` via `FromStr` |
| `CITE_DATA_DIR` | `paths.data_dir` | `PathBuf` |
| `CITE_CACHE_DIR` | `paths.cache_dir` | `PathBuf` |
| `CITE_EMBEDDING_PROVIDER` | `embedding.provider` | `String` |
| `CITE_EMBEDDING_API_KEY` / `CITE_API_KEY` | `embedding.api_key` | `String` (alias con deprecation warning) |
| `CITE_EMBEDDING_MODEL` | `embedding.model` | `String` |
| `CITE_TOP_K` | `retrieval.top_k` | `u32` |
| `CITE_MAX_FILE_SIZE` | `ingest.max_file_size_bytes` | `u64` |
| `CITE_CHUNK_SIZE` | `ingest.chunk_size_chars` | `usize` |
| `CITE_CHUNK_OVERLAP` | `ingest.chunk_overlap_chars` | `usize` |
| `CITE_EMBEDDING_TIMEOUT` | `ingest.embedding_timeout_secs` | `u64` |
| `CITE_EMBEDDING_ENDPOINT` | `ingest.embedding_endpoint` | `String` |
| `CITE_SENTENCE_CHUNKING` | `ingest.sentence_chunking` | `bool` (true/1/yes, false/0/no) |
| `CITE_MIN_CHUNK_CHARS` | `ingest.min_chunk_chars` | `usize` |
| `CITE_MAX_CHUNK_CHARS` | `ingest.max_chunk_chars` | `usize` |
| `CITE_BUILD_HIERARCHY` | `ingest.build_hierarchy` | `bool` (true/1/yes, false/0/no) |

---

## RuntimeMode

### Los 3 modos

| Modo | Serialización serde | Ingest permitido | Uso previsto |
|------|---------------------|-------------------|--------------|
| `PublicPackagedDemo` | `"public_packaged_demo"` | ❌ No | Demo read-only distribuible |
| `LocalPrivateDemo` | `"local_private_demo"` | ✅ Sí | Desarrollo y testing local |
| `Production` | `"production"` | ❌ No (ingest vía pipeline) | Deploy en producción |

### Parsing

- `FromStr`: match exacto sobre las 3 variantes en snake_case; retorna `Err(String)` para valores inválidos.
- `Display`: genera el snake_case correspondiente.
- `#[serde(rename_all = "snake_case")]`: serialización/deserialización serde consistente.
- Variables de entorno: parse silencioso (`.ok().and_then(|v| v.parse().ok())`) — valores inválidos se ignoran sin warning.
- CLI override: parse con error explícito que se convierte a `CiteError::ConfigError`.

### Reglas de RuntimeMode (en `engine::runtime_guard`)

- `check_ingest_allowed()`: solo `LocalPrivateDemo` permite ingest.
- `PublicPackagedDemo` y `Production` retornan `CiteError::RuntimeModeForbidden`.

---

## Decisiones de Diseño

### ✅ Aciertos

1. **Precedencia clara**: flags > env > file > defaults es una jerarquía estándar y bien pensada.
2. **`CITE_API_KEY` como alias**: buena UX para el caso común del embedding API key.
3. **Serde defaults en campos opcionales**: `#[serde(default)]` en campos como `api_key` permite archivos TOML parciales.
4. **RuntimeMode como enum**: previene errores de typo y habilita match exhaustivo.
5. **Graceful degradation**: archivo TOML ausente o con errores → usa defaults, no falla.
6. **`sentence_chunking` y `build_hierarchy` como `false` por defecto**: features opt-in, comportamiento conservador.

### ⚠️ Trade-offs observados

1. **`load_from` retorna `Result` pero nunca falla**: la firma sugiere que puede fallar, pero los errores de TOML se tragan con `eprintln!`. Esto engaña al caller.
2. **Archivo TOML incompleto**: el TOML solo cubre `provider`, `retrieval.top_k/evidence_floor/confidence_threshold`, y `data.dir`. No cubre `runtime`, `rate_limit`, `ingest`, `paths.cache_dir`, ni `retrieval.use_hierarchy`. Hay una brecha significativa entre lo que el TOML soporta y lo que las env vars soportan.
3. **`IngestConfig` es más un struct de "runtime" que de "ingest"**: contiene campos de embedding (`embedding_endpoint`, `embedding_timeout_secs`) que están más relacionados con el provider que con el pipeline de ingest.
4. **`FileConfig` intermedio es boilerplate**: la separación entre `TomlRoot` → `FileConfig` → merge manual es innecesaria. Podría simplificarse con serde flatten directo.
5. **Defaults hardcodeados duplicados**: `Config::defaults()` y `IngestConfig::default()` pueden desincronizarse. No hay un único source of truth para todos los defaults.

---

## Conexiones con Otros Crates

### `cli` (dependiente directo)
- Carga `Config` en `main.rs` con `Config::load_from(config_path)`
- Aplica CLI overrides (`--data-dir`, `--runtime-mode`) post-carga
- Distribuye sub-structs a comandos individuales (evaluate, retrieve, ingest, etc.)

### `engine` (dependiente directo)
- `runtime_guard.rs` — usa `RuntimeMode` para decidir qué operaciones están permitidas
- `evaluate.rs`, `context.rs`, `retrieve.rs` — usan `RateLimitConfig` y `RetrievalConfig`
- `ingest.rs` — usa `IngestConfig`

### `ingest` (dependiente directo)
- `lib.rs` — usa `IngestConfig` para configurar chunking, timeouts, y retries

### Patrón de uso típico
```
CLI main → Config::load_from() → apply_cli_overrides()
  → engine modules reciben &RateLimitConfig, &RetrievalConfig, etc.
  → ingest module recibe &IngestConfig
```

### No hay re-export del crate
- Los tipos se importan directamente: `use config::{Config, RuntimeMode, ...}`
