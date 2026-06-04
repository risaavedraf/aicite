# Errores Providers — Pendientes de Fix

> Errores encontrados en la revisión del crate `providers`.
> Este archivo NO se sube a GitHub.
> Cuando se fixee un error, marcarlo como ✅ y moverlo a completados.

---

## 🔴 CRITICAL

### 1. `create_provider` pasa API key vacía silenciosamente — error críptico en runtime

**Archivos:**
- `crates/cli/src/commands/mod.rs:94` (`resolve_api_key(...).unwrap_or_default()`)
- `crates/providers/src/gemini.rs:24` (constructor recibe key sin validar)
- `crates/providers/src/openai.rs:29` (constructor recibe key sin validar)

**Problema:** Cuando no hay API key configurada, `resolve_api_key()` devuelve `None` y `unwrap_or_default()` convierte a `""`. Ambos providers aceptan el string vacío sin error. El fallo ocurre después, en el primer `embed()`, con un error HTTP 401 críptico ("Gemini API returned HTTP 401 Unauthorized") en vez de un mensaje claro que diga "no API key configured".

Este bug ya está documentado como CLI errores #2, pero el problema tiene una dimensión en `providers`: los constructores no son defensivos contra keys vacías.

**Fix sugerido (doble capa):**

Capa 1 — En CLI (`create_provider`): validar antes de construir:
```rust
let api_key = resolve_api_key(config).ok_or_else(|| CiteError::ConfigError {
    message: "No API key configured. Set CITE_EMBEDDING_API_KEY, GEMINI_API_KEY, \
              OPENAI_API_KEY, or run `cite setup`.".into(),
})?;
```

Capa 2 — En providers (defensa en profundidad): validar en constructores:
```rust
// GeminiProvider::new
if api_key.is_empty() {
    return Err(CiteError::ConfigError {
        message: "Gemini API key cannot be empty".into(),
    });
}

// OpenAICompatibleProvider::new
if api_key.is_empty() {
    return Err(CiteError::ConfigError {
        message: "API key cannot be empty".into(),
    });
}
```

**Rationale de severidad:** CRITICAL porque un usuario nuevo sin configurar API key obtiene un error HTTP críptico que no indica la causa real. Experiencia de onboarding rota.

---

## 🟠 HIGH

### 2. Gemini model field: posible formato incorrecto en request body

**Archivo:** `crates/providers/src/gemini.rs:91`

**Problema:** El campo `model` en el body JSON se construye como:
```rust
model: format!("models/{}", self.model),
```

El endpoint URL ya contiene el modelo: `/v1beta/models/{model}:embedContent`. El body envía `"models/gemini-embedding-001"` como valor del campo `model`.

Según la documentación de la API de Gemini para `embedContent`, el campo `model` en el body debería ser el **recurso completo** `models/{model}` cuando se usa la REST API directamente. Esto es consistente con el patrón de Google APIs. Sin embargo, otros endpoints de Gemini esperan solo el nombre del modelo.

**Riesgo:** Si la API de Gemini cambia de formato o si algún modelo no sigue el patrón `models/`, este campo generará errores 400/404 sin un mensaje claro de qué está mal.

**Fix sugerido:** Verificar contra la API real y agregar un test de contracto. Si el formato `models/{name}` es correcto, documentar con un comment que explique por qué se antepone:
```rust
// Gemini REST API requires the full resource name in the model field
model: format!("models/{}", self.model),
```

**Rationale de severidad:** HIGH porque podría causar fallos silenciosos en producción con nuevos modelos, y el fix es verificación + documentación, no un cambio de arquitectura.

---

### 3. `embedding_timeout_secs` del config es ignorado — timeout hardcoded a 30s

**Archivos:**
- `crates/providers/src/gemini.rs:31` (`Duration::from_secs(30)`)
- `crates/providers/src/openai.rs:34` (`Duration::from_secs(30)`)
- `crates/config/src/lib.rs:101` (`embedding_timeout_secs: u64`, default 30)
- `crates/config/src/lib.rs:328` (`CITE_EMBEDDING_TIMEOUT` env var)

**Problema:** El config define `embedding_timeout_secs` (default 30, configurable via env var `CITE_EMBEDDING_TIMEOUT`), pero los constructores de providers no aceptan timeout como parámetro. Ambos hardcodean 30 segundos. Un usuario que configura `CITE_EMBEDDING_TIMEOUT=120` no obtiene ningún cambio.

**Fix sugerido:** Agregar parámetro `timeout_secs: u64` a ambos constructores:
```rust
// GeminiProvider
pub fn new(model: &str, api_key: &str, timeout_secs: u64) -> Result<Self, CiteError> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        // ...
}

// OpenAICompatibleProvider
pub fn new(endpoint: &str, model: &str, api_key: &str, timeout_secs: u64) -> Result<Self, CiteError> {
    // ...
}
```

Y en CLI `create_provider`:
```rust
let timeout = config.ingest.embedding_timeout_secs;
let provider = GeminiProvider::new(&config.embedding.model, &api_key, timeout)?;
```

**Rationale de severidad:** HIGH porque la configuración existe y está documentada pero no funciona. El usuario cree que puede controlar el timeout pero no puede. Rompe el contrato de la config.

---

### 4. Dependencies `tokio` y `tracing` en Cargo.toml — nunca usadas

**Archivo:** `crates/providers/Cargo.toml:8-9`

**Problema:** `tokio` y `tracing` están declarados como dependencies pero ningún archivo fuente en `crates/providers/src/` los importa (`use tokio` o `use tracing`). El crate usa `reqwest::blocking::Client` (no async), así que `tokio` no es necesario. `tracing` no se usa para logging.

**Impacto:**
- Aumenta tiempo de compilación innecesariamente
- Infla `Cargo.lock` con dependencias transitivas de tokio
- Confunde a desarrolladores que buscan código async o tracing que no existe

**Fix sugerido:** Eliminar ambas líneas de Cargo.toml:
```toml
[dependencies]
common = { workspace = true }
reqwest = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
# tokio = { workspace = true }      ← eliminar
# tracing = { workspace = true }     ← eliminar
```

**Verificación adicional:** `serde_json` también podría no usarse directamente (los tipos usan `serde::{Serialize, Deserialize}` pero no `serde_json::` explícitamente). Verificar si `reqwest` lo trae como transitive dependency antes de eliminar.

**Rationale de severidad:** HIGH porque son dependencies no triviales (tokio especialmente) que impactan compilación y mantenibilidad. No afecta runtime pero degrada DX.

---

## 🟡 MEDIUM

### 5. Sin validación de `model` ni `endpoint` vacíos en constructores

**Archivos:**
- `crates/providers/src/gemini.rs:24` (`GeminiProvider::new`)
- `crates/providers/src/openai.rs:29` (`OpenAICompatibleProvider::new`)

**Problema:** Ambos constructores aceptan strings vacíos para `model` y (en el caso de OpenAI) `endpoint`. Un `model: ""` genera un endpoint como `.../v1beta/models/:embedContent` que falla con error HTTP sin indicar que el model ID está mal.

OpenAI valida el esquema HTTPS del endpoint pero no valida que no sea vacío ni que sea una URL bien formada.

**Fix sugerido:**
```rust
// En ambos constructores:
if model.is_empty() {
    return Err(CiteError::ConfigError {
        message: "Embedding model ID cannot be empty".into(),
    });
}

// En OpenAICompatibleProvider::new además:
if endpoint.is_empty() {
    return Err(CiteError::ConfigError {
        message: "Embedding endpoint cannot be empty".into(),
    });
}
```

**Rationale de severidad:** MEDIUM porque config vacía ocurre típicamente en setup incorrecto y el error HTTP es diagnósticable, aunque no ideal. La capa CLI ya valida config básica.

---

### 6. DRY violation: lógica de HTTP + error handling duplicada entre providers

**Archivos:**
- `crates/providers/src/gemini.rs:86-114` (embed impl)
- `crates/providers/src/openai.rs:89-117` (embed impl)

**Problema:** Ambas implementaciones de `embed()` siguen el mismo patrón:
1. Construir request struct
2. POST con `.send().map_err(...)` (mismo mapeo de timeout vs red)
3. Check `!response.status().is_success()` con lectura de body
4. Parse JSON con `.json().map_err(...)`
5. Extraer vector

El paso 2 es idéntico en ambas (mismo mensaje, misma lógica de timeout). El paso 3 también. Esto es ~30 líneas duplicadas.

**Fix sugerido:** Extraer helper en `lib.rs`:
```rust
fn send_and_parse<T: serde::de::DeserializeOwned>(
    request: reqwest::blocking::RequestBuilder,
    provider_name: &str,
) -> Result<T, CiteError> {
    let response = request.send().map_err(|e| {
        let context = if e.is_timeout() { "timed out" } else { "failed" };
        CiteError::EmbeddingProviderError {
            message: format!("{} embedding request {}: {}", provider_name, context, e),
        }
    })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(CiteError::EmbeddingProviderError {
            message: format!("{} API returned HTTP {}: {}", provider_name, status, body),
        });
    }

    response.json().map_err(|e| CiteError::EmbeddingProviderError {
        message: format!("Failed to parse {} embedding response: {}", provider_name, e),
    })
}
```

**Rationale de severidad:** MEDIUM porque no afecta funcionalidad pero incrementa superficie de mantenimiento. Un cambio en el error handling requiere tocar ambos archivos.

---

### 7. `EvalProvider` dim 6 mezcla compliance/policy con prompt injection

**Archivo:** `crates/providers/src/eval.rs:75-86`

**Problema:** La dimensión 6 combina dos conceptos diferentes en el mismo keyword set:
```rust
// Compliance keywords:
"policy", "compliance", "security policy", "data classification", "incident"
// Prompt injection keywords:
"ignore", "instructions", "prompt", "injection"
```

La keyword `"ignore"` es problemática: texto legítimo como "ignore trailing whitespace" activaría la dimensión de compliance/prompt injection. Además, `"prompt"` activaría compliance para cualquier texto que hable de "prompts" sin relación con inyección.

**Impacto:** Los tests golden pueden clasificar incorrectamente chunks que contienen estas palabras en contexto legítimo como "compliance-related". Los scores de similitud se distorsionan para fixtures que contengan "ignore" o "prompt" en contexto normal.

**Fix sugerido:** Separar en dos dimensiones o ser más específico con las keywords:
```rust
// Usar frases más específicas en vez de palabras sueltas:
"ignore previous instructions",
"ignore all instructions",
"forget your instructions",
"prompt injection",
"injection attack",
```

**Rationale de severidad:** MEDIUM porque afecta la calidad de los tests golden (falsos positivos en compliance) pero no el path de producción. El EvalProvider solo se usa en evaluación.

---

## 🟢 LOW

### 8. `serde_json` podría no ser necesario como dependency directa

**Archivo:** `crates/providers/Cargo.toml:7`

**Problema:** `serde_json` está en dependencies pero ningún archivo en `providers/src/` importa `serde_json::`. Las funciones `Serialize`/`Deserialize` vienen de `serde`, no de `serde_json`. La serialización real la hace `reqwest` internamente via `.json(&request)`.

**Verificación necesaria:** Confirmar que `reqwest` con feature `json` ya trae `serde_json` como transitive dependency. Si es así, `serde_json` puede eliminarse de las dependencies directas.

**Fix sugerido:** Si se confirma, eliminar de Cargo.toml. Si no, dejar como está — no afecta compilación ni runtime.

---

### 9. Test `test_embed_invalid_key_returns_error` depende de red

**Archivo:** `crates/providers/src/gemini.rs:141-153`

**Problema:** El test llama a `embed("hello world")` con key `"invalid-key"`, lo cual hace una request HTTP real a la API de Gemini. En entornos CI sin acceso a internet (o con firewall), este test falla con un error de conexión, no con el error HTTP 401 esperado.

**Riesgo:** Flaky test en CI. El `assert!` verifica que el mensaje contenga "HTTP" O "failed" O "timed out", lo cual cubre el caso de sin-red, pero no es el comportamiento que se intenta testear.

**Fix sugerido:** Agregar `#[ignore]` para tests que requieren red, o marcar con un feature flag:
```rust
#[test]
#[ignore = "requires network access to Gemini API"]
fn test_embed_invalid_key_returns_error() {
    // ...
}
```

Lo mismo aplica para `test_embed_invalid_endpoint_returns_error` en `openai.rs:143-157`.

---

### 10. Sin `#[non_exhaustive]` en `EmbeddingProvider` trait

**Archivo:** `crates/providers/src/lib.rs:8`

**Problema:** El trait es extensible por diseño (se pueden agregar métodos con default impl), pero no tiene protección contra downstream que implemente el trait. Si se agrega un método nuevo sin default, todos los implementors externos rompen.

**Contexto:** En un monorepo con workspace, todos los implementors están en el mismo repo, así que el riesgo es bajo. Pero si el trait se hace público para plugins externos, esto importa.

**Fix sugerido:** No requiere cambio ahora, pero considerar documentar que el trait es "workspace-internal" con un doc comment:
```rust
/// Embedding provider trait.
///
/// Note: This trait is workspace-internal. Adding methods without defaults
/// will break all implementors in this workspace.
pub trait EmbeddingProvider { ... }
```

---

### 11. `GeminiProvider` no valida esquema HTTPS del endpoint

**Archivo:** `crates/providers/src/gemini.rs:27`

**Problema:** A diferencia de `OpenAICompatibleProvider` que valida `starts_with("https://")`, `GeminiProvider` construye el endpoint hardcoded con HTTPS, así que no necesita validación. Sin embargo, si en el futuro se parametriza el endpoint (para proxies o testing), no hay validación de esquema.

**Contexto:** El endpoint es hardcoded ahora (`generativelanguage.googleapis.com`), así que el riesgo es hipotético. Consistencia menor con OpenAI provider.

**Fix sugerido:** Ninguno necesario ahora. Solo observación de diseño.

---

## ✅ Completados

| # | Fix | Fecha | Notas |
|---|-----|-------|-------|
| - | - | - | - |

---

*Última actualización: 2026-06-02*
