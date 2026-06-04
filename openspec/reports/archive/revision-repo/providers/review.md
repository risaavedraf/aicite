# Revisión: `crates/providers` — Proveedores de Embedding

**Revisor:** el Gentleman subagent de revisión
**Fecha:** 2026-06-02
**Alcance:** 4 archivos en `crates/providers/src/`
**Metodología:** Inspección manual de todos los archivos fuente + trazado de conexiones con crates dependientes

---

## Resumen del Crate

**Propósito:** Proveer implementaciones concretas del trait `EmbeddingProvider` para convertir texto en vectores de embedding (`Vec<f32>`). Es la capa de abstracción entre el dominio (engine) y los servicios externos de IA (Gemini, OpenAI). También provee un provider determinístico (`EvalProvider`) para tests golden y evaluación offline.

**Estructura:**
```
crates/providers/
├── Cargo.toml
└── src/
    ├── lib.rs    — Trait EmbeddingProvider + type alias Embedding
    ├── gemini.rs — Implementación Google Gemini API
    ├── openai.rs — Implementación OpenAI-compatible
    └── eval.rs   — Mock determinístico basado en keywords
```

**Dependencias declaradas en Cargo.toml:**
- `common`: `CiteError` (el único tipo importado de otro crate)
- `reqwest` (blocking): HTTP client para APIs externas
- `serde` / `serde_json`: serialización de request/response JSON
- `tokio`: ⚠️ declarado pero nunca usado
- `tracing`: ⚠️ declarado pero nunca usado

**Tamaño:** ~350 líneas de código productivo + ~150 de tests. Crate compacto y enfocado.

---

## Flujo Principal

### Flujo de creación de un provider

```
CLI (create_provider)
    │
    ├── resolve_api_key(config)  →  Option<String>
    │       Precedencia: CITE_EMBEDDING_API_KEY > GEMINI_API_KEY > OPENAI_API_KEY > config file
    │       ⚠️ Si None → unwrap_or_default() → string vacía (bug conocido en CLI #2)
    │
    ├── match config.embedding.provider
    │   ├── "gemini" → GeminiProvider::new(model, api_key)
    │   │       Construye HTTP client con header x-goog-api-key
    │   │       Endpoint: generativelanguage.googleapis.com/v1beta/models/{model}:embedContent
    │   │
    │   └── _  (default) → OpenAICompatibleProvider::new(endpoint, model, api_key)
    │           Construye HTTP client con header Authorization: Bearer
    │           Valida endpoint es HTTPS
    │           Endpoint: config.ingest.embedding_endpoint (default: api.openai.com)
    │
    └── Result<Box<dyn EmbeddingProvider>, CiteError>
```

### Flujo de embedding

```
Engine llama provider.embed(text)
    │
    ├── Construye request JSON (estructura específica del provider)
    │   ├── Gemini: {"model": "models/{model}", "content": {"parts": [{"text": "..."}]}}
    │   └── OpenAI: {"input": "...", "model": "..."}
    │
    ├── HTTP POST con timeout de 30s (hardcoded)
    │
    ├── Error handling:
    │   ├── Timeout → EmbeddingProviderError("timed out")
    │   ├── Otro error de red → EmbeddingProviderError("failed")
    │   ├── HTTP !2xx → EmbeddingProviderError("HTTP {status}: {body}")
    │   └── Parse error → EmbeddingProviderError("Failed to parse")
    │
    └── Extrae Vec<f32>:
        ├── Gemini: response.embedding.values
        └── OpenAI: response.data[0].embedding
```

### Flujo EvalProvider (offline/testing)

```
EvalProvider.embed(text)
    │
    ├── text.to_lowercase()
    ├── Detección de keywords en 8 dimensiones temáticas:
    │   dim 0: API/gateway (0.9)
    │   dim 1: database/storage (0.9)
    │   dim 2: auth/security (0.9)
    │   dim 3: logging/monitoring (0.9)
    │   dim 4: users/CRUD (0.9)
    │   dim 5: rate limiting (0.9)
    │   dim 6: compliance/policy (0.85)
    │   dim 7: general (0.0 por defecto)
    │
    ├── +0.05 de ruido a dims activas
    ├── Normalización a unit vector
    └── Devuelve Vec<f32> de 8 dimensiones
```

---

## Módulos/Archivos Clave

### `src/lib.rs` — Trait y type alias

**Contenido:** 11 líneas.

- `pub type Embedding = Vec<f32>` — alias para vectores de embedding
- `pub trait EmbeddingProvider` — trait principal con 3 métodos:
  - `fn embed(&self, text: &str) -> Result<Embedding, CiteError>` — genera el vector
  - `fn model_id(&self) -> &str` — identificador del modelo
  - `fn provider_id(&self) -> &str` — identificador del provider

**Observación:** El trait es `Send + Sync` implícitamente (Rust lo infiere para types que lo cumplen), lo cual permite que los providers se usen en `Box<dyn EmbeddingProvider>`. Correcto.

**Re-exportaciones:** Solo `eval` se exporta como `pub mod`. Los providers Gemini y OpenAI también son `pub mod`. Esto permite que `engine/golden_provider.rs` acceda a `providers::eval::EvalProvider` y que CLI acceda a `providers::gemini::GeminiProvider`.

### `src/gemini.rs` — Google Gemini Provider

**Estructura pública:** `GeminiProvider` con campos privados: `client`, `model`, `endpoint`.

**Constructor (`new`):**
- Construye endpoint con modelo embebido en la URL
- Crea `reqwest::blocking::Client` con timeout de 30s
- Setea header `x-goog-api-key` como default header
- Valida que el header value sea ASCII válido (via `HeaderValue::from_str`)
- NO valida que `api_key` no sea vacío

**Request types (privados, serializables):**
- `GeminiRequest` → `model` + `content`
- `GeminiContent` → `parts`
- `GeminiPart` → `text`

**Response types (privados, deserializables):**
- `GeminiResponse` → `embedding`
- `GeminiEmbedding` → `values: Vec<f32>`

**`embed()` impl:**
- Serializa `GeminiRequest` con `model: "models/{model}"` en el body
- HTTP POST, clasifica errores por tipo (timeout vs red vs HTTP status)
- Lee body en errores HTTP para diagnóstico
- Parsea JSON response

**Tests (3):**
- `test_provider_creation`: Verifica constructor y getters
- `test_provider_endpoint_format`: Verifica formato de URL
- `test_embed_invalid_key_returns_error`: Integration test contra API real (necesita red, puede fallar offline)

### `src/openai.rs` — OpenAI-Compatible Provider

**Estructura pública:** `OpenAICompatibleProvider` con campos privados.

**Constructor (`new`):**
- **Valida HTTPS** en el endpoint (rechaza `http://`) — buen patrón de seguridad
- Crea client con timeout de 30s y header `Authorization: Bearer {key}`
- NO valida que `api_key` no sea vacío
- `provider_id` hardcoded a `"openai-compatible"`

**Request/Response types (privados):**
- `EmbeddingRequest` → `input` + `model`
- `EmbeddingResponse` → `data: Vec<EmbeddingData>`
- `EmbeddingData` → `embedding: Vec<f32>`

**`embed()` impl:**
- Serializa request, POST, manejo de errores similar a Gemini
- Extrae `data[0].embedding` — retorna error si `data` está vacío
- Maneja correctamente el caso de respuesta vacía (el Gemini provider no lo hace)

**Tests (5):**
- `test_provider_creation_valid`: HTTPS correcto
- `test_provider_creation_rejects_http`: Valida rechazo de HTTP
- `test_provider_model_id` / `test_provider_provider_id`: Getters
- `test_embed_invalid_endpoint_returns_error`: Integration test contra endpoint inexistente

### `src/eval.rs` — Eval Provider (Deterministic Mock)

**Estructura pública:** `EvalProvider` (unit struct, sin campos).

**Método estático `compute_vector`:**
- Keyword-based topic detection en 8 dimensiones
- Cada dimensión tiene un set de keywords y un valor base (0.9 o 0.85)
- Añade ruido de +0.05 a dims activas
- Normaliza a unit vector
- Dim 7 es general/noise (siempre 0.0, solo se activa si nada matchea)

**Diseño intencional:**
- Determinístico para la misma entrada
- Solo 8 dimensiones (vs ~1536 de text-embedding-3-small de OpenAI o ~768 de Gemini)
- Usado por `engine/golden_provider.rs` como base para `GoldenProvider` (agrega cache)

**Tests (4):**
- Determinismo, topic API, topic DB, unknown text → vector casi nulo

---

## Decisiones de Diseño

### ✅ Aciertos

1. **Trait-based abstraction:** `EmbeddingProvider` como trait permite que engine sea agnóstico al provider. `Box<dyn EmbeddingProvider>` permite inyección en runtime. Patrón correcto y extensible.

2. **HTTPS enforcement en OpenAI provider:** Rechazar endpoints no-HTTPS previene envío de API keys en texto plano. Falta el mismo check en Gemini (aunque Gemini hardcoded HTTPS).

3. **Error handling con contexto:** Cada punto de fallo (timeout, red, HTTP status, parse) produce un mensaje descriptivo con el error original. Los HTTP errors incluyen el body para diagnóstico.

4. **EvalProvider compartido:** Definido en `providers` pero consumido por `engine/golden_provider.rs` y `cli/commands/evaluate.rs`. Evita duplicación de la lógica de embedding determinístico.

5. **Bloqueo de request body en Gemini vs OpenAI:** Los structs de request/response son privados y específicos de cada módulo. No se filtran al API pública del crate. Correcto encapsulamiento.

6. **Tests de integración realistas:** Los tests de `gemini.rs` y `openai.rs` intentan llamadas reales (con key inválida). Esto valida el flujo completo de error handling sin necesidad de mock HTTP.

### ⚠️ Tradeoffs y Observaciones

1. **Blocking HTTP vs async:** El crate usa `reqwest::blocking::Client` y declara `tokio` como dependencia sin usarlo. La decisión de usar blocking es pragmática para un CLI, pero `tokio` queda como dependencia muerta en Cargo.toml. Si no se planea migrar a async, `tokio` debería eliminarse.

2. **Timeout hardcoded a 30s:** Ambos providers hardcodean `Duration::from_secs(30)`. El config tiene `embedding_timeout_secs` que se puede configurar via `CITE_EMBEDDING_TIMEOUT` env var, pero nunca se pasa al provider. La configuración existe pero está desconectada.

3. **Sin validación de API key vacía:** Ni Gemini ni OpenAI verifican que `api_key` no sea vacío en el constructor. La validación es responsabilidad del caller (CLI), pero el caller falla silenciosamente (bug #2 del CLI). Los providers podrían ser más defensivos.

4. **Gemini body `models/` prefix:** El campo `model` en el request body se construye como `"models/{model}"` (gemini.rs:91). La API de Gemini espera el nombre del modelo como `models/{name}` en el path del endpoint, pero el campo `model` en el body JSON es debatible — algunos endpoints de Gemini esperan solo el nombre sin el prefijo. Este es un punto que requiere verificación manual contra la API real.

5. **Sin retry ni backoff:** Los providers hacen un único intento. Para una CLI interactiva es aceptable (el usuario reintenta), pero para uso programático (engine batch) sería deseable tener retry con backoff exponencial.

6. **EvalProvider es keyword-only:** La detección de temas usa `text.contains(kw)` sobre texto lowercase. No maneja variaciones morfológicas ("authenticating" no matchea "authentication"), plurales, ni sinónimos. Es aceptable para un mock de evaluación, pero limita la cobertura del golden dataset.

---

## Conexiones con Otros Crates

| Crate | Relación con `providers` |
|-------|--------------------------|
| **common** | `providers` importa exclusivamente `CiteError`. Es la dependencia más ligera posible — un solo tipo. |
| **config** | `providers` NO importa `config`. La configuración es inyectada por CLI al construir los providers (model, endpoint, api_key pasados como parámetros). |
| **cli** | Consumer directo. `commands/mod.rs` (`create_provider`) construye providers desde config y los inyecta en `CommandContext`. `commands/evaluate.rs` usa `EvalProvider`. `commands/setup.rs` usa ambos providers para test de conexión. |
| **engine** | Consumer principal a través del trait. `ingest.rs`, `retrieve.rs`, `evaluate.rs`, `context.rs` reciben `&dyn EmbeddingProvider`. `golden_provider.rs` envuelve `EvalProvider` con cache. |
| **storage** | Sin relación directa. |
| **ingest** | Sin relación directa. |

**Diagrama de dependencias:**
```
common (CiteError)
   ↑
providers (trait + 3 impls)
   ↑
   ├── engine (ingest, retrieve, evaluate, context, golden_provider)
   └── cli (create_provider, evaluate, setup)
```

**Patrón de inyección:** El trait `EmbeddingProvider` funciona como interface de inyección de dependencias. CLI crea el provider concreto → lo boxing como `Box<dyn EmbeddingProvider>` → lo almacena en `CommandContext` → engine lo recibe como `&dyn EmbeddingProvider`. Este es un patrón limpio con bajo acoplamiento.
