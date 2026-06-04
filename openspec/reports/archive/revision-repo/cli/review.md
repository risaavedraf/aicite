# Revisión: `crates/cli` — Arquitectura, flujo y hallazgos del binario `cite`

**Revisor:** el Gentleman subagent de revisión
**Fecha:** 2026-06-02
**Alcance:** 16 archivos en `crates/cli/src/` (main.rs, output.rs, commands/mod.rs, 13 subcomandos)
**Metodología:** Inspección manual de todos los archivos fuente, verificación de claims de revisión previa, búsqueda de código muerto con grep, análisis de dependencias cross-crate

---

## Resumen del Crate

El crate `cli` es el punto de entrada del proyecto. Produce el binario `cite` con 14 subcomandos organizados en 4 categorías funcionales:

| Categoría | Comandos | Acceso a DB | Acceso a Provider |
|-----------|----------|-------------|-------------------|
| Setup | `health`, `setup` | health sí (directo), setup no | health sí, setup sí (propio) |
| Ingesta | `ingest`, `list`, `get`, `retry` | Todos | Solo `ingest` |
| Búsqueda | `search`, `retrieve`, `context`, `read` | Todos | Solo search, retrieve, context |
| Gestión | `trace`, `refresh`, `evaluate` | trace, refresh sí; evaluate usa DB en memoria | Solo evaluate (EvalProvider propio) |

El crate no contiene lógica de negocio — es una capa de presentación que parsea argumentos, construye contextos de ejecución (DB + provider), delega al engine, y formatea output en JSON o texto legible.

### Estructura

```
src/
├── main.rs              # Entrypoint: parseo CLI, flags globales, dispatch
├── output.rs            # Helpers de serialización JSON y tipos compactos
└── commands/
    ├── mod.rs           # CommandContext, resolve_data_dir, create_provider, resolve_api_key
    ├── health.rs        # Diagnósticos de salud (config, provider, DB, data dir)
    ├── setup.rs         # Configuración interactiva/non-interactiva de API keys
    ├── ingest.rs        # Ingesta directa, encolada, y procesamiento de cola
    ├── list.rs          # Listado de documentos
    ├── get.rs           # Metadata de un documento
    ├── retry.rs         # Reintento de documento fallido
    ├── search.rs        # Búsqueda vectorial (scores + previews)
    ├── retrieve.rs      # Retrieval rankeado (texto completo)
    ├── context.rs       # Context pack con citas para agentes
    ├── read.rs          # Lectura de cita o chunk por ID
    ├── trace.rs         # Metadata de trazabilidad
    ├── refresh.rs       # Refresco de corpus con atomic snapshot swap
    └── evaluate.rs      # Evaluación golden dataset con corpus en memoria
```

---

## Flujo Principal

### 1. Arranque (`main.rs`)

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
  └── match command → dispatch a commands::*::execute()
```

### 2. CommandContext (`commands/mod.rs`)

Dos constructores:
- `CommandContext::open(config, json)` — abre DB + crea provider embedding. Usado por: `ingest`, `search`, `retrieve`, `context`.
- `CommandContext::open_db_only(config, json)` — solo abre DB. Usado por: `list`, `get`, `retry`, `read`, `trace`, `refresh`.

Ambos retornan `Result<Self, i32>` donde el error es el exit code (ya impreso).

### 3. Resolución de API Key (`commands/mod.rs:81-89`)

Precedencia: `CITE_EMBEDDING_API_KEY` → `GEMINI_API_KEY` → `OPENAI_API_KEY` → config file `api_key`. Retorna `Option<String>`.

### 4. Patrón de Output

Cada comando sigue el mismo patrón:
```
match operation_result {
    Ok(data) => {
        if json { print_json(&data) }
        else { println formato legible }
        ExitCode::Success
    }
    Err(e) => {
        if json { print_json(&e.to_json_response()) }
        else { eprintln!("Error: {e}") }
        e.exit_code()
    }
}
```

### 5. Modos de Output JSON

Tres comandos (`context`, `search`, `retrieve`) soportan formato compacto por defecto cuando `--json` se usa, con flag `--full` para el JSON completo. Los tipos compactos viven en `output.rs` y reducen ~60-70% del tamaño del JSON eliminando metadata y truncando snippets a 200 chars.

---

## Módulos/Archivos Clave

### `main.rs` (240 líneas)

- **`Cli` struct**: 5 flags globales (`--json`, `--config`, `--data-dir`, `--runtime-mode`, `--no-banner`) + subcomando.
- **`Commands` enum**: 14 variantes, cada una con sus Args dedicados excepto `Health`, `List`, `Refresh` (sin args).
- **`main()`**: Orquestación completa del lifecycle: env → parse → config → overrides → banner → recovery → dispatch.
- **`should_run_startup_recovery()`**: Excluye Health y Setup del recovery. Nota: esto significa que `list`, `get`, `read`, `trace` (comandos de solo lectura) también ejecutan recovery, agregando latencia innecesaria.
- **`run_startup_recovery()`**: Abre DB, ejecuta `recover_interrupted_processing`, cierra DB. La DB se abre de nuevo en el command context subsecuente.
- **`is_retrieval_command()`**: Define qué comandos muestran el banner de divulgación del provider. Excluye ingest, list, get, retry, refresh, evaluate.
- **`show_provider_disclosure()`**: Banner informativo a stderr cuando el provider es "real" (no eval/mock).
- **Tests**: 3 tests para `parse_runtime_mode` y `apply_cli_overrides`. Bien cubierto.

### `output.rs` (340 líneas)

- **`print_json()`**: Serialización pretty-printed con fallback a stderr en caso de error.
- **Tipos compactos**: `CompactContextResponse`, `CompactSearchOutput`, `CompactRetrieveOutput` — versiones reducidas de los responses del engine.
- **`to_compact_*()`**: Transformaciones borrowing (clonan strings). Usadas por los comandos.
- **`into_compact_*()`**: Transformaciones consuming (zero-clone). Marcadas `#[allow(dead_code)]` — nunca usadas.
- **`truncate_to()`**: Truncación segura por chars (no bytes), con ellipsis. Correcta para UTF-8.
- **Tests**: 7 tests cubriendo compact context, search, retrieve, y truncación.

### `commands/mod.rs` (100 líneas)

- **`CommandContext`**: Struct con `db: Database` y `provider: Option<Box<dyn EmbeddingProvider>>`. Dos constructores: `open` (full) y `open_db_only`.
- **`handle_command_error()`**: Helper que imprime error en JSON o texto. Usado solo dentro de `CommandContext` construction, no en los cuerpos de comandos.
- **`resolve_data_dir()`**: Prioridad: config → `dirs::data_dir()/cite` → `.` (fallback).
- **`resolve_api_key()`**: Cadena de env vars con fallback a config.
- **`create_provider()`**: Match sobre `config.embedding.provider`: `"gemini"` → `GeminiProvider`, cualquier otro → `OpenAICompatibleProvider` con endpoint configurable.

### `commands/health.rs` (230 líneas)

- El comando más detallado. Verifica: versión, schema, runtime mode, config path, API key (enmascarada), provider (test real de embedding), data dir (writable), DB (existe, counts).
- **`check_provider()`**: Hace un `provider.embed("test")` real — llamada de red a la API externa. Esto va más allá de "chequeo de salud local" como dice el docstring del comando.
- **`mask_key()`**: Enmascara API key mostrando solo últimos 4 chars. Correcto.
- **`check_dir_writable()`**: Crea archivo temporal `.cite_write_test`, verifica escritura, lo elimina. Limpio.
- **`count_rows()`**: SQL directo con `query_row`. Usa `unwrap_or(0)` como fallback — aceptable para health check.

### `commands/setup.rs` (260 líneas)

- **Modo interactivo**: Usa `dialoguer` para selección de provider, input de API key con confirmación, test de conexión, guardado.
- **Modo non-interactivo**: Requiere `--provider` y `--api-key`. Test + guardado.
- **`--check`**: Alias para `cite health`.
- **`test_provider_connection()`**: Crea provider con modelos hardcodeados (`text-embedding-004` para Gemini, `text-embedding-3-small` para OpenAI), ignora la config del usuario.
- **`save_config()`**: Escribe TOML a `dirs::config_dir()/cite/config.toml`. Solo guarda `type` y `api_key`. En Unix setea permisos 0o600. En Windows no hace nada especial.
- **`unwrap_or_default()` en password input (línea 153)**: Si `dialoguer::Password::interact()` falla (non-TTY), produce string vacío silenciosamente. El check posterior de empty lo captura pero el error message es misleading.

### `commands/ingest.rs` (195 líneas)

- **Tres modos** (mutuamente excluyentes via `ArgGroup`): `--path` (ingest directo), `--queued` (encolar), `--next` (procesar siguiente de la cola).
- Crea `CommandContext::open()` (con provider) incluso para `--queued` que solo necesita DB.
- Pasa `production_mode: bool` a las funciones de engine — este bool solo afecta la derivación del display name, NO bloquea ingest.
- Tests: Ninguno a nivel CLI.

### `commands/search.rs` (170 líneas), `commands/retrieve.rs` (160 líneas), `commands/context.rs` (115 líneas)

- Los tres comparten la misma estructura: validación de flags (`--flat` incompatible con `--topic`/`--concept`), creación de `CommandContext::open()`, match sobre resultado, formato compacto/full JSON.
- La validación de flags está duplicada en los tres archivos (DRY violation).
- `search.rs` es el único que maneja `None` del provider con `match`; los otros dos usan `.unwrap()`.
- Todos soportan `--k` para limitar resultados, `--topic`, `--concept`, `--flat`, `--full`.

### `commands/get.rs` (85 líneas), `commands/list.rs` (75 líneas), `commands/retry.rs` (65 líneas), `commands/refresh.rs` (55 líneas)

- Comandos simples de una operación. Usan `open_db_only` (sin provider). Patrón de error/output estándar.

### `commands/read.rs` (125 líneas)

- Soporta dos selectores: `--citation-id --trace-id` o `--chunk-id --document-id`.
- Validación manual en `build_selector()` — podría usar `ArgGroup` de clap como hace `IngestArgs`.
- Usa `open_db_only` — correcto, no necesita provider.

### `commands/trace.rs` (125 líneas + test)

- Busca metadata de trazabilidad por trace ID.
- Test de integración notable: crea DB temporal, persiste trace, ejecuta comando, verifica resultado. Es el único comando (además de evaluate) con test de integración real.
- El test usa nanosecond timestamps para nombres únicos de dirs temporales — posible colisión en ejecución paralela, aunque improbable.

### `commands/evaluate.rs` (490 líneas)

- El archivo más grande del crate. Contiene ~200 líneas de datos de test inline (3 documentos, 12 chunks) y 10 golden fixtures.
- Usa `EvalProvider` (determinístico, sin red) y `Database::open_memory()`.
- Ignora completamente la config del usuario (`_config: &Config`) — by design para evaluación reproducible.
- La función `execute` ignora `_args: &EvaluateArgs` — el campo `json` del struct nunca se lee.
- 4 tests: seed corpus, fixture count, provider determinism, full evaluation pass.

---

## Decisiones de Diseño

### 1. Separación CommandContext

Buen patrón. Centraliza la apertura de DB y creación de provider, evitando duplicación. Los dos niveles (`open` vs `open_db_only`) reflejan correctamente las necesidades diferentes de cada comando.

### 2. Output JSON compacto vs completo

Decisión inteligente. Los responses completos del engine son pesados (600-1500 tokens). Los compactos (~200-250 tokens) son más adecuados para consumo por agentes. El flag `--full` permite acceso al JSON completo cuando se necesita.

### 3. Error handling como exit codes

El patrón de retornar `i32` exit codes en lugar de `Result` tiene el trade-off de simplicidad vs composabilidad. Es correcto para un CLI donde cada comando es un fire-and-forget, pero dificulta testing y reuso.

### 4. Recovery automático

Ejecutar `recover_interrupted_processing` antes de cada comando (excepto health/setup) es defensivamente bueno pero tiene costo de latencia. Para comandos de solo lectura como `list` o `get`, el recovery es innecesario.

### 5. Evaluación con corpus propio

La decisión de usar un corpus determinístico en memoria para evaluate es correcta — garantiza resultados reproducibles sin dependencia de estado externo. El trade-off es que no evalúa el pipeline real de ingesta.

---

## Conexiones con Otros Crates

| Crate | Uso en CLI | Archivos que lo importan |
|-------|-----------|--------------------------|
| `common` | `CiteError`, `ExitCode`, `ReadSelector`, tipos de response | main.rs, mod.rs, context.rs, read.rs, evaluate.rs, todos los commands |
| `config` | `Config`, `RuntimeMode`, `RetrievalConfig`, `RateLimitConfig` | main.rs, mod.rs, todos los commands, evaluate.rs |
| `engine` | `ingest::*`, `retrieve::*`, `context::*`, `recovery::*`, `refresh::*`, `evaluate::*`, `runtime_guard::*` | mod.rs, ingest.rs, search.rs, retrieve.rs, context.rs, read.rs, trace.rs, refresh.rs, evaluate.rs, health.rs |
| `storage` | `Database` | mod.rs, evaluate.rs, trace.rs (tests) |
| `providers` | `GeminiProvider`, `OpenAICompatibleProvider`, `EvalProvider`, `EmbeddingProvider` trait | mod.rs, setup.rs, evaluate.rs |

### Flujo de dependencia

```
cli → engine (lógica de negocio)
cli → storage (acceso a DB, solo via CommandContext o evaluate directo)
cli → providers (creación de providers, solo via mod.rs y setup.rs)
cli → config (carga y manejo de configuración)
cli → common (tipos compartidos, errores, exit codes)
```

El crate `cli` no expone APIs — es un consumer final. No hay crates que dependan de `cli`.

---

*Documento generado: 2026-06-02*
*Revisado por: el Gentleman (subagent de revisión)*
*Estado: COMPLETADO*
