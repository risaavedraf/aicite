# Revisión de Compliance — Junio 2026

> Documentación interna. No subir a GitHub.
> Generado por análisis de PRDs, implementación, y documentos de gobernanza.

## Propósito

Análisis de consistencia entre los PRDs del proyecto y la implementación real, enfocado en el compliance checklist que bloquea el modo production.

---

## 🔴 Incongruencias detectadas (requieren decisión)

### 1. ✅ Actualizado: `check_ingest_allowed` sí se llama en el path CLI de ingest

**Archivo**: `crates/engine/src/runtime_guard.rs` (fn `check_ingest_allowed`)
**Archivo**: `crates/cli/src/commands/ingest.rs`

Verificación CR-2 (2026-06-04): el comando `ingest` invoca `engine::runtime_guard::check_ingest_allowed(&config.runtime.mode)` desde `execute()` y maneja el error en salida JSON o humana. Por lo tanto, production/public-demo ingestion está bloqueado en el entrypoint CLI actual.

**Riesgo restante:** los entrypoints internos `engine::ingest::ingest`, `ingest_next` e `ingest_internal` no re-ejecutan el guard. Si el engine se usa fuera del CLI, el caller debe aplicar el boundary check o el proyecto debe mover/documentar el enforcement en engine.

**Referencia de tests:** `engine/tests/runtime_mode.rs` y tests unitarios de `engine::runtime_guard` ejercitan las variantes de `check_ingest_allowed`.

---

### 2. PRD dice "Full deletion API is post-MVP" pero compliance requiere "data deletion workflow" antes de production

**Archivo**: `openspec/prd/03-mvp-scope.md` → Out of scope: "Full `delete_document` API and automated retention workflow"
**Archivo**: `openspec/prd/12-legal-privacy-compliance.md` → Checklist: "Add a data deletion workflow for documents, chunks, embeddings..."

Hay una contradicción directa: el scope dice que deletion es post-MVP, pero el compliance checklist (necesario para desbloquear production) requiere deletion.

**Pregunta**: ¿El "data deletion workflow" del checklist se refiere al reset manual documentado (ya implementado) o a una API de deletion granular? Si es lo segundo, production no se puede desbloquear sin hacer deletion (que es post-MVP).

**Decisión necesaria**: Clarificar si el compliance checklist se satisface con el reset manual documentado, o si realmente necesita una API `delete_document`.

---

### 3. No hay auth en MVP pero compliance requiere "identificar data controller"

**Archivo**: `openspec/prd/03-mvp-scope.md` → Decision: "Start without auth"
**Archivo**: `openspec/prd/12-legal-privacy-compliance.md` → "Identify the data controller/responsible party"

Si no hay auth, ¿quién es el data controller? En local_private_demo el "controller" es el operador que corre el CLI, pero eso no está formalmente documentado.

**Pregunta**: ¿Basta con documentar que "el operador del CLI es el data controller" para satisfacer el checklist, o se necesita auth real antes de production?

---

### 4. Display name en production: slicing por bytes vs UTF-8

**Archivo**: `crates/ingest/src/validator.rs` → `sanitize_display_name` línea ~95

```rust
if trimmed.len() > 255 {
    trimmed[..255].to_string()
}
```

`trimmed.len()` devuelve **bytes**, no caracteres. Si el nombre tiene caracteres multi-byte (acentos, CJK, etc.), `trimmed[..255]` puede cortar en medio de un carácter UTF-8 y **panic en runtime**.

**Pregunta**: ¿Se debe cambiar a `chars().take(255).collect()` para safety? Esto es un bug real que puede causar panic en production mode con nombres internacionalizados.

**Impacto**: MEDIUM — solo afecta production mode, pero es un crash evitable.

---

### 5. White-box registry vs implementación real de providers

**Archivo**: `openspec/prd/13-ai-ethics-governance.md` → Registry tiene `embedding-configured-default` y `embedding-mock-local`
**Archivo**: `README.md` → Menciona "Gemini or OpenAI-compatible" como providers

El PRD dice que los modelos comerciales son "non-normative candidates" y el registry solo tiene entries genéricas, pero el README nombra providers específicos.

**Pregunta**: ¿Los providers "Gemini" y "OpenAI-compatible" están formalmente en el registry o son candidatos no verificados? Si son candidatos, ¿el README debería ser más cauteloso?

---

### 6. Compliance checklist dice "privacy notice" pero no hay template ni ejemplo

**Archivo**: `openspec/prd/12-legal-privacy-compliance.md` → "Publish a privacy notice explaining purpose, data categories, providers, retention, rights, and contact channel"

No hay template, ejemplo, ni referencia a dónde iría esta privacy notice. ¿En el README? ¿En un archivo separado? ¿En la CLI output?

**Pregunta**: ¿Dónde debería vivir la privacy notice? ¿Basta con documentar en README o se necesita un archivo formal separado?

---

### 7. Rate limiting: FR-109 define 20 req/min pero ¿está implementado?

**Archivo**: `openspec/prd/04-functional-requirements.md` → FR-109: "20 retrieval/context requests per minute per `runtime_mode + corpus_id + provider_id + retrieval_scope`"

Necesitamos verificar si el rate limiting durable con esa clave compuesta realmente está implementado en storage/engine, o si es un requirement documentado sin implementar.

**Pregunta**: ¿Existe la tabla de rate limits en SQLite con la key compuesta, o es TODO?

---

### 8. Refresh con atomic snapshot swap: ¿implementado?

**Archivo**: `openspec/prd/04-functional-requirements.md` → FR-015: "atomic snapshot semantics: reads keep using the last ready snapshot while staging rebuilds, then switch atomically"

El requirement es claro, pero necesitamos verificar si `cite refresh` realmente implementa staging + atomic swap, o solo rebuilda in-place.

**Pregunta**: ¿El refresh usa staging tables + atomic rename, o rebuilda directamente sobre la data live?

---

## 🟡 Observaciones de consistencia (no bloqueantes pero importantes)

### 9. Ley 21.719 entry en force: diciembre 2026

El PRD menciona que la Ley 21.719 tiene "deferred entry into force reported for December 2026". Si el proyecto quiere production antes de esa fecha, solo aplica Ley 19.628. Si es después, aplica el régimen completo.

**Observación**: La fecha de production afecta qué ley aplica. Esto debería estar en el checklist como decisión documentada.

### 10. "Responsible owner" en traces es optional en local/private pero required en production

**Archivo**: `openspec/prd/13-ai-ethics-governance.md` → Context trace schema: `responsible_owner` es "optional or null in local/private mode"

Pero el compliance checklist requiere "Assign a responsible product owner/operator" antes de production.

**Observación**: Está bien diseñado, pero la transición de "null en local" a "required en production" necesita un mecanismo para configurar el owner.

### 11. No hay tests de golden dataset visibles en el repo

El acceptance criteria requiere "Golden dataset includes at least 3 direct-fact cases, 2 no-results cases, 1 ambiguous query, 1 multi-chunk query, and 1 prompt-injection fixture."

**Pregunta**: ¿Existe el golden dataset en el repo? ¿O es parte de `cite evaluate` con datos bundled?

### 12. `install.sh` hace pipe a sh — riesgo de supply chain

**Archivo**: `install.sh` → `curl -sSf ... | sh`

El README promueve `curl | sh` como método de instalación. Esto es estándar en la industria pero es un vector de supply chain si el repo es comprometido.

**Observación**: Para el PRD de seguridad, considerar si vale la pena documentar checksums o firmas para los releases.

### 13. `dialoguer` como dependencia interactiva

El comando `cite setup` usa `dialoguer` para prompts interactivos, pero el PRD dice "Agent-safe overrides exist... without requiring interactive prompts."

**Pregunta**: ¿`cite setup --non-interactive` realmente funciona sin stdin? ¿Está testeado en CI?

### 14. Documentación en español en carpeta Improvements

Hay archivos en español en `openspec/Improvements/` que no están referenciados desde el index principal.

**Observación**: Si son válidos, deberían estar en el index. Si son borradores, aclararlo.

---

## 🟢 Cosas bien diseñadas (para reconocer)

1. **Runtime modes con enforcement explícito** — Los 3 modos están bien diferenciados con reglas claras.
2. **Data minimization** — Solo chunks relevantes van al provider, no documentos completos.
3. **Log hygiene** — La allowlist de campos seguros es exhaustiva y bien pensada.
4. **Trace schema** — Los campos de trazabilidad cubren auditoría sin exponer contenido sensible.
5. **Privacy-by-design** — Las 6 áreas (purpose limitation, data minimization, transparency, user control, security, auditability) están bien cubiertas en el diseño.
6. **Prohibited uses** — La lista de usos prohibidos es clara y cubre los casos de alto riesgo.
7. **Chilean privacy law** — Considerar ambas leyes (actual + futura) es buena práctica.

---

## Próximos pasos

- [ ] Verificar si `check_ingest_allowed` realmente bloquea ingest en production (test manual)
- [ ] Verificar si rate limiting durable está implementado en storage
- [ ] Verificar si refresh usa atomic snapshot swap
- [ ] Verificar si el golden dataset existe y pasa
- [ ] Verificar si `cite setup --non-interactive` funciona sin stdin
- [ ] Decidir si "data deletion workflow" = reset manual o API granular
- [ ] Decidir si "data controller" = operador CLI o necesita auth
- [ ] Fix del slicing UTF-8 en `sanitize_display_name`
- [ ] Revisar crate por crate la implementación

---

*Documento generado: 2026-06-02*
*Revisado por: el Gentleman (análisis automático)*
*Estado: PENDIENTE de verificación manual*
