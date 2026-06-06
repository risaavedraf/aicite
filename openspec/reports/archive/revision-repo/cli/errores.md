# Errores CLI — Pendientes de Fix

> Errores encontrados en la revisión del crate `cli`.
> Este archivo NO se sube a GitHub.
> Cuando se fixee un error, marcarlo como ✅ y moverlo a completados.

---

## 🔴 CRITICAL

### 1. ✅ RESUELTO: `check_ingest_allowed` ahora se invoca desde el CLI ingest path

**Verificado en CR-2 (2026-06-04):** `crates/cli/src/commands/ingest.rs` llama `engine::runtime_guard::check_ingest_allowed(&config.runtime.mode)` dentro de `execute()` antes de procesar `queued`, `next` o ingest inmediato. En JSON emite `e.to_json_response()`; en salida humana imprime el error y retorna `e.exit_code()`.

**Estado actual:** production/public-demo ingest queda bloqueado para el entrypoint CLI. El riesgo restante está en el límite interno del engine: `engine::ingest::ingest`, `ingest_next` e `ingest_internal` no llaman `check_ingest_allowed` por sí mismos, así que callers que salteen el CLI deben aplicar el guard o el proyecto debe documentar/agregar ese boundary check.

**Referencias actuales:**
- `crates/cli/src/commands/ingest.rs` — `execute()` invoca `check_ingest_allowed`.
- `crates/engine/src/runtime_guard.rs` — define y testea `check_ingest_allowed`.
- `crates/engine/src/ingest.rs` — `ingest_next` / `ingest_internal` todavía no revalidan runtime mode.

---

## 🟠 HIGH

### 2. Flag `--json` duplicado en `EvaluateArgs` — `cite evaluate --json` no produce JSON

**Archivos**:
- `crates/cli/src/commands/evaluate.rs:14-18` (define `pub json: bool`)
- `crates/cli/src/commands/evaluate.rs:359` (execute ignora `_args`)
- `crates/cli/src/main.rs:123` (pasa `cli.json` como parámetro `json`)

**Problema**: `EvaluateArgs` define su propio flag `--json` (línea 17), pero `execute()` ignora `_args: &EvaluateArgs` y usa el parámetro `json` que proviene del flag global `cli.json`. Esto crea un caso roto:

| Invocación | `args.json` | `cli.json` | JSON output? |
|------------|-------------|------------|--------------|
| `cite --json evaluate` | `false` | `true` | ✅ Sí |
| `cite evaluate --json` | `true` | `false` | ❌ No |
| `cite evaluate` | `false` | `false` | ❌ No |

`cite evaluate --json` no produce output JSON, lo que rompe la expectativa del usuario.

**Fix**: Eliminar `pub json: bool` de `EvaluateArgs` (línea 17). El flag global `--json` ya maneja esto.

**Severidad**: HIGH — Bug de UX que afecta a cualquier usuario que intente `cite evaluate --json`.

---

### 3. ✅ RESUELTO: `create_provider` rechaza API key ausente

**Archivo**: `crates/cli/src/commands/mod.rs`
**Verificado en CR-2 (2026-06-04)**

**Estado actual:** La afirmación original era histórica. `create_provider` ya no usa `resolve_api_key(config).unwrap_or_default()`: ahora llama `resolve_api_key(config).ok_or_else(...)` y retorna un `CiteError::ConfigError` claro cuando no hay API key configurada.

```rust
let api_key = resolve_api_key(config).ok_or_else(|| CiteError::ConfigError {
    message: "No API key configured. Set CITE_EMBEDDING_API_KEY, GEMINI_API_KEY, \
              OPENAI_API_KEY, or run `cite setup`.".into(),
})?;
```

**Defensa adicional:** Los constructores de providers (`GeminiProvider::new` y `OpenAICompatibleProvider::new`) también rechazan `api_key.is_empty()`, así que el CLI y los providers validan la credencial en capas.

**Riesgo restante:** ninguno para el caso de API key vacía en `create_provider`; mantener tests/contrato de error claro si se refactoriza `CommandContext::open()`.

---

### 4. `setup` hardcodea modelos en test de conexión de provider

**Archivo**: `crates/cli/src/commands/setup.rs:208-232`

```rust
fn test_provider_connection(
    _config: &Config,  // ← ignorado
    provider: &str,
    api_key: &str,
) -> Result<u64, String> {
    let result = match provider {
        "gemini" => {
            let p = GeminiProvider::new("text-embedding-004", api_key) // hardcodeado
```

**Problema**: `test_provider_connection` recibe `_config: &Config` pero lo ignora completamente. Hardcodea:
- `"text-embedding-004"` para Gemini
- `"text-embedding-3-small"` para OpenAI

Si el usuario tiene configurado un modelo diferente (ej: `text-embedding-3-large`), el test de setup ejercita un modelo distinto al que se usará en producción. El test puede pasar pero el modelo real puede fallar (o ser más costoso).

**Fix**: Aceptar el modelo como parámetro o leerlo de config:
```rust
fn test_provider_connection(
    config: &Config,
    provider: &str,
    api_key: &str,
) -> Result<u64, String> {
    let model = &config.embedding.model;
    // ...
```

**Severidad**: HIGH — El test de setup da falsa confianza. El usuario cree que todo funciona pero testeó con el modelo equivocado.

---

### 5. Unwrap inconsistente del provider entre comandos

**Archivos**:
- `commands/context.rs:50` — `ctx.provider.as_ref().unwrap()`
- `commands/ingest.rs:66` — `ctx.provider.as_ref().unwrap()`
- `commands/retrieve.rs:80` — `ctx.provider.as_ref().unwrap()`
- `commands/search.rs:49-54` — `match ctx.provider.as_ref()` con `None → error`

**Problema**: `search` maneja `None` correctamente con un `match` y mensaje de error claro. Los otros tres comandos usan `.unwrap()`, que producirá panic si `CommandContext` se refactoriza para hacer el provider opcional.

Actualmente es seguro porque `CommandContext::open()` siempre crea `Some(provider)`, pero:
1. Es inconsistente dentro del mismo crate.
2. Es frágil ante refactorizaciones futuras.
3. Un panic en un CLI es mala UX (stack trace en vez de mensaje de error).

**Fix**: Agregar método helper en `CommandContext`:
```rust
impl CommandContext {
    pub fn provider(&self) -> Result<&dyn EmbeddingProvider, CiteError> {
        self.provider.as_deref().ok_or_else(|| CiteError::ConfigError {
            message: "Embedding provider not configured".into(),
        })
    }
}
```

Y reemplazar `ctx.provider.as_ref().unwrap()` por `ctx.provider()?` en todos los comandos.

**Severidad**: HIGH — Inconsistencia que puede causar panics en producción ante refactorizaciones.

---

### 6. `ingest --queued` crea provider innecesariamente

**Archivo**: `crates/cli/src/commands/ingest.rs:60-67`

```rust
let ctx = match CommandContext::open(config, json) {  // crea DB + provider
    Ok(ctx) => ctx,
    Err(code) => return code,
};
```

**Problema**: `CommandContext::open()` se llama al inicio de `execute()`, antes de determinar qué modo de ingest se usa. El modo `--queued` solo necesita la DB (`enqueue_ingest` no usa el provider). Si el provider no está configurado o el endpoint es inaccesible, `cite ingest --queued file.pdf` falla innecesariamente.

**Fix**: Reorganizar el flujo para crear el provider solo cuando se necesita:
```rust
// Para --queued, usar open_db_only
// Para --next y path directo, usar open (necesita provider)
```

**Severidad**: HIGH — Comando que debería funcionar offline (encolar) falla por dependencia innecesaria al provider.

---

## 🟡 MEDIUM

### 7. Violación DRY: patrón de error display repetido 14+ veces

**Archivos**: Todas las funciones `execute()` en `commands/*.rs`

El bloque:
```rust
if json {
    print_json(&e.to_json_response());
} else {
    eprintln!("Error: {e}");
}
return e.exit_code() as i32;
```

aparece verbatim en: `get.rs:74-78`, `context.rs:103-107`, `ingest.rs:91-95`, `ingest.rs:141-145`, `ingest.rs:183-187`, `list.rs:64-68`, `read.rs:33-37`, `read.rs:76-80`, `refresh.rs:45-49`, `retrieve.rs:151-155`, `retry.rs:53-57`, `search.rs:160-164`, `trace.rs:60-64` (13 occurrences en bodies) + `mod.rs:62-68` (helper `handle_command_error` que no se reutiliza en los bodies).

**Nota irónica**: `handle_command_error` en `mod.rs` implementa exactamente este patrón pero solo se usa para errores de construcción de `CommandContext`, no para errores de los comandos.

**Fix**: Reutilizar o extender el helper existente:
```rust
// output.rs
pub fn handle_error(e: &CiteError, json: bool) -> i32 {
    if json {
        print_json(&e.to_json_response());
    } else {
        eprintln!("Error: {e}");
    }
    e.exit_code() as i32
}
```

**Severidad**: MEDIUM — No causa bugs pero dificulta mantenimiento. Un cambio en el formato de error requiere tocar 14+ archivos.

---

### 8. `CommandContext::open` devuelve `Result<Self, i32>` — pierde tipo de error

**Archivo**: `crates/cli/src/commands/mod.rs:31`

```rust
pub fn open(config: &Config, json: bool) -> Result<Self, i32> {
```

**Problema**: El tipo de error es `i32` crudo (exit code). El error ya fue impreso dentro de `open()` via `handle_command_error`, así que el caller solo puede retornar el código. Esto:
1. Mezcla error reporting con error construction.
2. Impide que los callers inspeccionen o transformen el error.
3. Dificulta testing (no se puede assert sobre el tipo de error).

**Fix**: Retornar `Result<Self, CiteError>` y dejar que el caller maneje el display. O al menos retornar un struct que contenga tanto el exit code como el error original.

**Severidad**: MEDIUM — Limita composabilidad y testabilidad.

---

### 9. Validación de flags duplicada en search/retrieve/context

**Archivos**:
- `crates/cli/src/commands/context.rs:34-40`
- `crates/cli/src/commands/search.rs:37-43`
- `crates/cli/src/commands/retrieve.rs:37-43`

**Problema**: La misma lógica de validación está copiada tres veces:
```rust
if args.flat && (args.topic.is_some() || args.concept.is_some()) {
    eprintln!("Error: --flat cannot be combined with --topic or --concept.");
    return common::ExitCode::Validation as i32;
}
if args.topic.is_some() && args.concept.is_some() {
    eprintln!("Error: --topic and --concept cannot be used together.");
    return common::ExitCode::Validation as i32;
}
```

**Fix**: Extraer a función compartida:
```rust
// commands/mod.rs
pub fn validate_retrieval_flags(flat: bool, topic: Option<&str>, concept: Option<&str>) -> Result<(), ExitCode> {
    // ...
}
```

**Severidad**: MEDIUM — DRY violation que puede causar inconsistencia si se fixea en un lugar pero no en otros.

---

### 10. `setup.rs` — `unwrap_or_default()` silencia errores de TTY

**Archivo**: `crates/cli/src/commands/setup.rs:153`

```rust
.interact()
.unwrap_or_default();
```

**Problema**: Si `dialoguer::Password::interact()` falla (ej: stdin no es un TTY, pipe, CI environment), `unwrap_or_default()` produce un string vacío silenciosamente. El check posterior (`k if !k.is_empty()`) lo captura, pero el error message es "API key cannot be empty" — misleading cuando el problema real es que no hay TTY interactivo disponible.

**Fix**:
```rust
.interact()
.map_err(|e| {
    eprintln!("Error reading input: {e}. Use --non-interactive for scripted setup.");
    ExitCode::Validation as i32
})?;
```

**Severidad**: MEDIUM — Error message confuso en ambientes non-TTY.

---

### 11. `save_config` no guarda modelo ni endpoint

**Archivo**: `crates/cli/src/commands/setup.rs:244-254`

```rust
let content = format!(
    "[provider]\ntype = \"{}\"\napi_key = \"{}\"\n",
    provider, api_key
);
```

**Problema**: El config guardado solo almacena tipo de provider y API key. No incluye:
- Modelo de embedding (que `test_provider_connection` hardcodea)
- Embedding endpoint (para providers openai-compatible custom)
- Runtime mode

Un usuario que corre `cite setup` y luego `cite ingest` usará los defaults del sistema, que pueden no coincidir con lo que testeó durante setup.

**Severidad**: MEDIUM — Inconsistencia entre lo que setup testea y lo que se guarda.

---

### 12. `health` hace llamada de red pese a ser "local state health check"

**Archivo**: `crates/cli/src/commands/health.rs:155-170`

```rust
fn check_provider(config: &Config) -> ProviderHealth {
    // ...
    match provider.embed("test") {  // ← llamada de red real
```

**Problema**: El docstring del comando dice "Check CLI runtime and local state health", pero `check_provider` ejecuta `provider.embed("test")` — una llamada HTTP real a la API externa. Esto:
1. Puede fallar por problemas de red sin que haya un problema local.
2. Puede tomar varios segundos (timeout de red).
3. Inesperado para un usuario que solo quiere verificar estado local.

**Fix**: Considerar hacer el test de provider opcional (ej: `--test-provider` flag) o documentar claramente que health hace una verificación de conectividad externa.

**Severidad**: MEDIUM — UX engañosa y posible latencia sorpresa.

---

## 🟢 LOW

### 13. Funciones `into_compact_*` son dead code

**Archivo**: `crates/cli/src/output.rs:97-113, 139-155, 177-193`

Tres funciones `into_compact_context`, `into_compact_search`, `into_compact_retrieve` están marcadas `#[allow(dead_code)]`. Son alternativas zero-clone a `to_compact_*` pero nunca se usan.

**Severidad**: LOW — Código muerto que ocupa espacio y puede confundir a futuros contributors.

---

### 14. Gaps significativos en cobertura de tests

**Comandos con tests**: `main.rs` (3 tests), `output.rs` (7 tests), `trace.rs` (1 test), `evaluate.rs` (4 tests)
**Comandos sin tests a nivel CLI**: health, setup, ingest, list, get, retry, search, retrieve, context, read, refresh

**Tests faltantes de mayor prioridad**:
- `ingest` — el comando más complejo con 3 modos (immediate, queued, next) y lógica de production mode
- `search` / `retrieve` / `context` — comandos core con validación de flags y formato compacto
- `read` — validación compleja de selectores mutuamente excluyentes

**Severidad**: LOW — Los comandos son wrappers delgado sobre engine, que sí tiene tests, pero la lógica de CLI (validación de flags, formateo de output) queda sin cobertura.

---

### 15. `run_startup_recovery` corre para comandos de solo lectura

**Archivo**: `crates/cli/src/main.rs:87-105`

`should_run_startup_recovery()` retorna `false` solo para Health y Setup. Esto significa que comandos de solo lectura como `list`, `get`, `read`, `trace` también ejecutan recovery, lo que:
1. Abre la DB dos veces (una en recovery, otra en el comando).
2. Agrega latencia innecesaria para operaciones que no modifican datos.

**Severidad**: LOW — Performance, no correctness.

---

### 16. `evaluate` ignora completamente la config del usuario

**Archivo**: `crates/cli/src/commands/evaluate.rs:359`

```rust
pub fn execute(_args: &EvaluateArgs, _config: &Config, json: bool) -> i32 {
```

**Problema**: Tanto `_args` como `_config` se ignoran. El comando crea su propia DB en memoria, usa `EvalProvider` determinístico, y hardcodea `RetrievalConfig`. Esto es by design (evaluación reproducible) pero puede sorprender a usuarios que esperan que `cite evaluate` evalúe su corpus real.

**Severidad**: LOW — By design, pero la documentación del comando debería aclarar esto.

---

### 17. `read.rs` usa validación manual en vez de `ArgGroup`

**Archivo**: `crates/cli/src/commands/read.rs:89-120`

La función `build_selector()` valida manualmente que `--citation-id` y `--chunk-id` son mutuamente excluyentes, y que cada uno requiere su par complementario. `IngestArgs` en ingest.rs usa `ArgGroup` de clap para la misma lógica, que es más declarativa y produce mejores mensajes de error automáticamente.

**Severidad**: LOW — Consistencia de estilo, no funcional.

---

### 18. `trace` test usa nombres de directorio con nanosecond precision

**Archivo**: `crates/cli/src/commands/trace.rs:81-87`

```rust
fn unique_temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "cite-{name}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
```

**Problema**: En ejecución paralela de tests, dos tests podrían generar el mismo timestamp en nanosegundos, causando colisión de directorio. Aunque improbable, es una práctica fragil.

**Fix**: Usar `tempfile::tempdir()` o `Uuid::new_v4()` para nombres garantizados únicos.

**Severidad**: LOW — Flaky test potential, improbable en la práctica.

---

### 19. Permisos de archivo en Windows para config de API key

**Archivo**: `crates/cli/src/commands/setup.rs:250-254`

```rust
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(&config_path, std::fs::Permissions::from_mode(0o600));
}
```

En Windows, el archivo de config con API key tiene permisos por defecto (potencialmente legible por otros usuarios en la misma máquina). No hay equivalente Windows.

**Severidad**: LOW — Security hardening, no urgente para desarrollo.

---

### 20. Cross-crate: `sanitize_display_name` panic UTF-8 en truncación

**Archivo**: `crates/ingest/src/validator.rs:97-98` (NO es del crate `cli`, pero afecta su flujo de ingest)

```rust
if trimmed.len() > 255 {
    trimmed[..255].to_string()  // PANICA si byte 255 cae en multi-byte UTF-8
}
```

**Problema**: `len()` devuelve bytes, `trimmed[..255]` slicea por bytes. Si el byte 255 cae dentro de un carácter multi-byte (acentos, CJK, emoji), panic en runtime.

**Nota**: Este bug está en el crate `ingest`, no en `cli`. Se menciona aquí porque el previous review lo incluyó como hallazgo del CLI. Corregido: pertenece al review del crate `ingest`.

**Severidad**: LOW para este review (cross-crate), CRITICAL para el review del crate `ingest`.

---

## ✅ Completados

| # | Fix | Fecha | Notas |
|---|-----|-------|-------|
| - | - | - | - |

---

## Notas sobre la revisión previa

La revisión previa fue verificada en su mayoría correcta. Correcciones:

1. **C2 previo** (panic UTF-8 en `sanitize_display_name`): Correcto como hallazgo pero **ubicado mal** — el código está en `crates/ingest/src/validator.rs`, no en `crates/cli`. Se movió a nota cross-crate (#20).
2. **H5 previo** (unwrap inconsistente): Correcto pero le faltó precisión — `search.rs` es el que hace lo correcto, los otros tres son los inconsistentes.
3. **Nuevo**: Se agregaron H6 (ingest --queued crea provider innecesariamente), M5 (validación de flags duplicada), M6 (handle_command_error no reutilizado), M12 (health hace llamada de red), L16 (evaluate ignora config), L17 (read usa validación manual), L18 (trace test nombres colisionables).

---

*Última actualización: 2026-06-02*
