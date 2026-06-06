# RFC: Auto-documentation sync from cite binary

**Status:** Implemented (Phase 1 complete, 2026-06-06)

> Phase 1 (MVP) is implemented: `check-docs` parses markdown, executes commands against binary, compares output, reports in human/JSON format. Phase 2 (semantic comparison) and Phase 3 (CI integration) tracked in rfc-tags-and-note-add.md as tag-based solution.
**Author:** el Gentleman + rikar
**Created:** 2026-06-06
**Related:** EVALUACION_CITE.md, agent-usage-guide.md desincronizado

---

## Problem

La documentación de cite se desincroniza con el binario. Ejemplo concreto:

- `agent-usage-guide.md` dice que "compact mode es una propuesta"
- El binario v0.2.0+ ya implementa compact como default
- Nadie detectó el desync hasta la evaluación

**Causa raíz:** No hay mecanismo automático para verificar que la documentación refleje lo que el binario realmente hace.

**Impacto:**
- Usuarios confían en docs que no son correctas
- Agentes AI siguen instrucciones obsoletas
- Esfuerzo manual para mantener docs actualizadas

---

## Proposal

Un sistema donde **cite verifica su propia documentación** y reporta desincronizaciones.

### Core idea

```
┌─────────────────────────────────────────────────────────┐
│                    cite check-docs                       │
├─────────────────────────────────────────────────────────┤
│  1. Parsea archivos .md buscando bloques de código      │
│  2. Ejecuta cada bloque contra el binario actual        │
│  3. Compara output esperado vs output real              │
│  4. Reporta qué está desincronizado                     │
└─────────────────────────────────────────────────────────┘
```

### Ejemplo de uso

```bash
cite check-docs openspec/guides/agent-usage-guide.md
```

**Output:**
```
Checking agent-usage-guide.md against cite v0.3.0...

❌ OUTDATED: Section "Compact/snippet mode"
   Line 45-52
   Claim: "compact mode is a proposal"
   Reality: compact is default since v0.2.0
   Suggestion: Update to reflect current behavior

✅ OK: Section "Real-world invocation"
   Line 12-18
   All examples pass

⚠️  WARNING: Section "Performance characteristics"
   Line 120
   Claim: "Query latency: 800-1500ms"
   Last verified: 2026-05-28 (9 days ago)
   Suggestion: Re-verify with current binary

Results: 1 outdated, 1 warning, 5 OK
```

### Batch mode

```bash
cite check-docs openspec/ --recursive
```

Recorre todos los `.md` y reporta el estado general.

---

## Technical design

### 1. Code block parser

Extract fenced code blocks from markdown:

````markdown
```bash
cite search "test" --topic "Auth"
```
````

Identify:
- Command to run
- Expected output (if next block is ```json or ```output)
- Flags used

### 2. Command executor

Run extracted commands against current binary:

```rust
fn execute_and_capture(cmd: &str) -> Result<Output> {
    // Execute: cite <args>
    // Capture: stdout, stderr, exit code
    // Timeout: 30s per command
}
```

### 3. Output comparator

Compare actual vs expected:

| Scenario | Status |
|----------|--------|
| Output matches expected | ✅ OK |
| Command fails but doc says it works | ❌ OUTDATED |
| Command works but output format changed | ⚠️ CHANGED |
| Doc claims feature doesn't exist | ❌ OUTDATED |
| Example uses deprecated flag | ⚠️ DEPRECATED |

### 4. Metadata header

Each behavioral doc should have:

```yaml
---
type: behavioral
verified_with: v0.3.0
last_verified: 2026-06-05
verification_status: pass | fail | stale
---
```

### 5. Report format

Human-readable (default):
```
3 outdated, 2 warnings, 10 OK
```

Machine-readable:
```json
{
  "file": "agent-usage-guide.md",
  "binary_version": "0.3.0",
  "results": [
    {
      "section": "Compact/snippet mode",
      "status": "outdated",
      "line": 45,
      "detail": "compact is default, not proposal"
    }
  ]
}
```

---

## Scope

### In scope

- [ ] Parse markdown for bash/code blocks
- [ ] Execute commands against cite binary
- [ ] Compare output (exact match + semantic match)
- [ ] Report outdated sections
- [ ] Add metadata headers to docs
- [ ] Batch mode for full openspec/ scan

### Out of scope (future)

- [ ] Auto-fix outdated docs (suggest but don't modify)
- [ ] CI integration (GitHub Action)
- [ ] Auto-generate docs from binary
- [ ] Verify PDFs or non-code content

---

## Files affected

| File | Change |
|------|--------|
| `src/cli/check_docs.rs` | New command implementation |
| `src/cli/mod.rs` | Register new subcommand |
| `openspec/guides/*.md` | Add metadata headers |
| `openspec/rfc/active/rfc-auto-docs-sync.md` | This RFC |

---

## Benefits

1. **Catch desyncs early** — before users or agents see them
2. **Reduce manual effort** — automated verification
3. **Build trust** — docs are verified against actual binary
4. **Agent safety** — agents won't follow outdated instructions

---

## Trade-offs

| Aspect | Consideration |
|--------|---------------|
| Maintenance | New command to maintain |
| False positives | Some outputs are dynamic (latency, timestamps) |
| Coverage | Can only verify executable examples, not prose |
| Performance | Full scan could be slow (many commands) |

**Mitigation:**
- Skip dynamic values (latency, UUIDs, timestamps)
- Use regex patterns for variable output
- Cache results per file

---

## Open questions

1. **Should `check-docs` run on every release?**
   - Option A: Manual only
   - Option B: CI gate before release
   - Option C: Scheduled (weekly)

2. **What about prose that's outdated but has no code example?**
   - Currently: not detectable
   - Future: LLM-based semantic check?

3. **Who fixes the docs?**
   - Option A: Human maintains
   - Option B: `cite check-docs --fix` auto-updates (risky)
   - Option C: PR with suggested changes

---

## Implementation plan

### Phase 1: MVP (v0.3.1)

- [ ] Basic command parsing from markdown
- [ ] Execute against cite binary
- [ ] Simple output comparison (exact match)
- [ ] Human-readable report

### Phase 2: Smart comparison (v0.3.2)

- [ ] Regex patterns for dynamic output
- [ ] Semantic comparison (ignore whitespace, order)
- [ ] Metadata headers in docs

### Phase 3: CI integration (v0.4.0)

- [ ] GitHub Action
- [ ] Block release if docs outdated
- [ ] Auto-generate report in PR

---

## References

- EVALUACION_CITE.md (in this folder) — Full evaluation documenting the desync problem
- agent-usage-guide.md — Example of outdated doc
- README.md — Current feature documentation
