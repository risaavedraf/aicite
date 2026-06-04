# Errores Config — Pendientes de Fix

## 🔴 CRITICAL

*(ninguno)*

## 🟠 HIGH

### E01 — Campos confusos/duplicados en `IngestConfig`: `min_chunk_size_chars` vs `min_chunk_chars`

**Archivos:** `crates/config/src/lib.rs` líneas 94-103, 126-128

**Problema:**
`IngestConfig` tiene dos campos con nombres casi idénticos y defaults diferentes:
- `min_chunk_size_chars` (línea 94): default `100` — "Minimum chunk size in characters"
- `min_chunk_chars` (línea 126): default `30` — "Minimum chunk length in chars before merge"

Además, `chunk_size_chars` (default `1000`, "Target chunk size") y `max_chunk_chars` (default `200`, "Maximum chunk length") están invertidos: el "máximo" (200) es menor que el "objetivo" (1000).

Esto crea confusión extrema para cualquier consumidor del config: ¿cuál es realmente el tamaño mínimo y máximo de un chunk? ¿Cuándo se usa cada uno?

**Código actual:**
```rust
pub struct IngestConfig {
    pub chunk_size_chars: usize,       // target: 1000
    pub min_chunk_size_chars: usize,   // minimum: 100
    // ...
    #[serde(default = "default_min_chunk_chars")]
    pub min_chunk_chars: usize,        // minimum: 30
    #[serde(default = "default_max_chunk_chars")]
    pub max_chunk_chars: usize,        // maximum: 200 (!) < target (!)
}
```

**Fix sugerido:** Consolidar en un modelo claro con 3 campos:
```rust
pub target_chunk_chars: usize,  // default: 1000
pub min_chunk_chars: usize,     // default: 100
pub max_chunk_chars: usize,     // default: 1500 (debe ser > target)
```
Eliminar `min_chunk_size_chars` (o renombrar `min_chunk_chars` a `min_chunk_size_chars` para consistencia). Asegurar que `min < target < max`.

**Impacto:** Cualquier consumidor de `IngestConfig` puede estar usando el campo incorrecto. Potencial bug silencioso en chunking.

---

### E02 — Archivo TOML no puede configurar la mayoría de los campos

**Archivos:** `crates/config/src/lib.rs` líneas 356-411 (`FileConfig`, `TomlRoot`)

**Problema:**
El TOML solo reconoce 3 secciones: `[provider]`, `[retrieval]`, `[data]`. Los siguientes campos son **inaccesibles** desde el archivo de configuración:
- `runtime.mode`
- `rate_limit.max_requests`, `rate_limit.window_seconds`
- `ingest.*` (todos los 11 campos)
- `paths.cache_dir`
- `retrieval.use_hierarchy`

Esto crea una asimetría significativa: las env vars pueden configurar ~20 campos, pero el TOML solo ~7. El usuario que prefiere archivos de configuración (el path más común) tiene acceso a una fracción de la funcionalidad.

**Código actual:**
```rust
struct TomlRoot {
    provider: Option<TomlProvider>,    // 3 campos
    retrieval: Option<TomlRetrieval>,  // 3 campos
    data: Option<TomlData>,            // 1 campo
    // Faltan: runtime, rate_limit, ingest, paths.cache_dir
}
```

**Fix sugerido:** Expandir `TomlRoot` para incluir todas las secciones:
```rust
struct TomlRoot {
    runtime: Option<TomlRuntime>,
    provider: Option<TomlProvider>,
    retrieval: Option<TomlRetrieval>,
    data: Option<TomlData>,
    rate_limit: Option<TomlRateLimit>,
    ingest: Option<TomlIngest>,
}
```
Y actualizar el merge en `Config::merge()` para aplicar los campos nuevos del archivo.

**Impacto:** Usuarios que configuran por archivo están limitados a la mitad de las opciones disponibles.

---

## 🟡 MEDIUM

### E03 — `load_from` retorna `Result<Self, CiteError>` pero nunca falla

**Archivos:** `crates/config/src/lib.rs` líneas 167-173

**Problema:**
La firma promete que puede fallar, pero la implementación siempre retorna `Ok(...)`. Los errores de parseo TOML se imprimen a stderr y se ignoran (`FileConfig::load` retorna `None`). Esto:
1. Engaña al caller que maneja `Err` con mensajes que nunca se darán.
2. Pierde la información del error (el usuario ve un warning en stderr pero el programa continúa con defaults silenciosamente).

**Código actual:**
```rust
pub fn load_from(config_path: Option<&std::path::Path>) -> Result<Self, CiteError> {
    let defaults = Self::defaults();
    let file = FileConfig::load(config_path);  // TOML errors → None, warning a stderr
    let env = EnvOverrides::load();
    Ok(Self::merge(defaults, file, env))  // Siempre Ok
}
```

**Opciones de fix:**
1. **Propagar error de TOML:** cambiar `FileConfig::load` para retornar `Result`, y usar `?` en `load_from`. El caller decide si TOML malformado es fatal o warning.
2. **Cambiar la firma a `Self` (sin Result):** si el diseño actual es intencional (graceful degradation), no simular posibilidad de error.

**Impacto:** Deuda técnica. Los callers manejan errores que no existen, y los errores reales de config se pierden.

---

### E04 — Variables de entorno con valores inválidos se ignoran silenciosamente

**Archivos:** `crates/config/src/lib.rs` líneas 298-325 (`EnvOverrides::load`)

**Problema:**
Todos los campos numéricos usan `.and_then(|v| v.parse().ok())`. Si un usuario setea `CITE_TOP_K=abc` o `CITE_CHUNK_SIZE=-5`, el valor inválido se ignora silenciosamente y se usa el default. No hay warning ni error.

**Código actual:**
```rust
top_k: std::env::var("CITE_TOP_K")
    .ok()
    .and_then(|v| v.parse().ok()),  // "abc" → None → default, sin warning
```

**Fix sugerido:** Log de warning para valores que fallan parseo:
```rust
top_k: std::env::var("CITE_TOP_K")
    .ok()
    .and_then(|v| match v.parse() {
        Ok(val) => Some(val),
        Err(_) => {
            eprintln!("⚠ Warning: Invalid value '{}' for CITE_TOP_K, using default", v);
            None
        }
    }),
```

**Impacto:** El usuario cree que configuró algo, pero en realidad se usa el default. Difícil de diagnosticar.

---

### E05 — Config no implementa `PartialEq` ni `Default`

**Archivos:** `crates/config/src/lib.rs` línea 40

**Problema:**
`Config` derive `Debug, Clone, Serialize, Deserialize` pero no `PartialEq` ni `Default`. Esto:
1. Impide assertions directas en tests: `assert_eq!(config1, config2)` no compila.
2. Impide `Config::default()` — los defaults están en `fn defaults()` privada.
3. Solo `RuntimeMode` tiene `PartialEq, Eq` (línea 6).

**Código actual:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]  // Falta PartialEq
pub struct Config { ... }
```

**Fix sugerido:**
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config { ... }
```
Derivar `PartialEq` (y `Eq` donde aplique) en todos los sub-structs. Implementar `Default` delegando a `defaults()`.

**Impacto:** Limita la capacidad de testing y comparación de configs.

---

### E06 — Tests insuficientes: no cubren merge, env vars, ni archivo TOML

**Archivos:** `crates/config/src/lib.rs` líneas 423-462

**Problema:**
Los tests solo cubren:
- `Config::defaults()` — valores hardcodeados
- `RuntimeMode::from_str` — parsing de strings

No hay tests para:
- Carga desde archivo TOML
- Override por variables de entorno
- Merge con precedencia (file override defaults, env override file)
- `CITE_API_KEY` como alias de `CITE_EMBEDDING_API_KEY`
- Valores inválidos en env vars
- Combinaciones de fuentes

**Impacto:** La lógica de merge (la parte más compleja del crate) no tiene cobertura de tests. Regresiones pueden pasar inadvertidas.

---

## 🟢 LOW

### E07 — Path de config por defecto usa "cite" en vez del nombre del proyecto

**Archivos:** `crates/config/src/lib.rs` línea 418

**Problema:**
```rust
fn default_config_path() -> Option<PathBuf> {
    let config_dir = dirs::config_dir()?;
    Some(config_dir.join("cite").join("config.toml"))
}
```
El directorio es `cite/` pero el proyecto es `aiharness`. En Linux esto sería `~/.config/cite/config.toml`. No está claro si "cite" es intencional (nombre legacy) o un error.

**Fix sugerido:** Verificar si el nombre debería ser `aiharness` o mantener `cite` como nombre de producto. Documentar la decisión.

---

### E08 — `FileConfig` intermedio es boilerplate innecesario

**Archivos:** `crates/config/src/lib.rs` líneas 356-411

**Problema:**
La cadena `TomlRoot → FileConfig → merge manual` es verbose y propensa a errores. `FileConfig` tiene exactamente los mismos campos que `TomlRoot` pero aplanados.

**Fix sugerido:** Usar serde flatten o un solo struct TOML que se mapee directamente al merge:
```rust
impl FileConfig {
    fn from_toml(root: TomlRoot) -> Self {
        // o directamente mergear desde TomlRoot
    }
}
```
O mejor aún, hacer el merge directamente desde `TomlRoot` y eliminar `FileConfig`.

---

### E09 — `CITE_API_KEY` alias emite warning de deprecation pero ambos funcionan

**Archivos:** `crates/config/src/lib.rs` líneas 277-285

**Problema:**
El warning dice "CITE_API_KEY will be ignored" pero el código toma `CITE_EMBEDDING_API_KEY` primero y cae a `CITE_API_KEY` como fallback. El warning es impreciso: `CITE_API_KEY` no es ignorado cuando es la única variable seteada, solo cuando ambas existen.

**Fix sugerido:** Cambiar el mensaje:
```
"CITE_API_KEY es redundante cuando CITE_EMBEDDING_API_KEY está seteada. Se usará CITE_EMBEDDING_API_KEY."
```

---

## ✅ Completados

| # | Severidad | Descripción | Estado |
|---|-----------|-------------|--------|
| — | — | — | (vacío — sin fixes aplicados aún) |
