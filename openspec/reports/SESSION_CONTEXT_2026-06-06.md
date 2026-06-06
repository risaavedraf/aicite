# Cite CLI v0.3.1 — Estado de sesión (2026-06-06 nocturna)

## Contexto de continuación

Sesión enfocada en testing de jerarquía, análisis de retrieval quality, y diseño de tags/note add.

---

## COMPLETADO en esta sesión

### ✅ Re-ingesta con jerarquía
- 19 docs ingeridos con `CITE_BUILD_HIERARCHY=true` (5 fallaron por Gemini 429)
- DB: 20 docs ready, 186 chunks, **152 topics, 70 concepts**
- Docs ingeridos: PRDs 01-15 (excepto 03, 06, prd_changelog), guides (agent-usage-guide, demo), architecture (cite-notes-hybrid, front-lobe-engine, rename-to-cite), README

### ✅ Filtros --topic/--concept con datos reales
- `--topic "Non-goals"` filtra correctamente (2 results de product-brief vs 5 sin filtro)
- `--concept "Corpus owner / operator"` filtra a 1 result en users-and-problems

### ✅ Breadcrumb en search/retrieve/context --full
- `breadcrumb: "04-functional-requirements.md > Corpus management"`
- `breadcrumb: "02-users-and-problems.md > Primary persona > Corpus owner / operator"` (3 niveles)

### ✅ Evaluate 10/10 con nueva DB

### ✅ Análisis de retrieval quality
- **Scores bajos:** top score 0.63-0.76, ninguna query llega a 0.8
- **Spread bajo:** 0.05-0.08 entre rank 1 y 5 (poca discriminación)
- **Causas:** gemini-embedding-001 general-purpose, chunking 1000 chars fixed, sin re-ranking, sin hybrid search
- **Evaluate 10/10 es espejismo:** golden fixtures escritos para matchear con este modelo

### ✅ RFC: tags + note add + retrieval roadmap
- **Archivo:** `openspec/rfc/active/rfc-tags-and-note-add.md`
- Tags key:value (múltiples por entidad, búsqueda por key/value/key:value)
- Note add para que el agente escriba directo en Cite
- Roadmap: v0.3.2/0.3.3 nomic embedder, v0.4 semantic chunking + re-ranking, v0.5 hybrid search

### ✅ RFC auto-docs-sync cerrado como implementado
- `openspec/rfc/active/rfc-auto-docs-sync.md` → Status: Implemented

### ✅ EVALUACION_CITE.md actualizada
- Sección de hierarchy testing results
- Sección de retrieval quality analysis con scores reales

---

## PENDIENTE / PRÓXIMA SESIÓN

### Temas a definir:

1. **Embedder local (v0.3.2/0.3.3)** — RFC completo: `openspec/rfc/active/rfc-embedding-providers.md`
   - Sistema de providers pluggable (Ollama, ONNX, HuggingFace, Gemini, OpenAI)
   - Ollama como primer provider local (GPU automática, HTTP, 0 setup)
   - qwen3-embedding 4B como modelo recomendado para GPU (MTEB ~67, RTX 3070 8GB)
   - nomic-embed-text v1.5 como fallback CPU (MTEB 62.28, 137M params)
   - `cite doctor` para diagnósticos del pipeline
   - Error messages accionables para todos los providers
   - `cite ingest --reembed` para migración atómica entre providers
   - `cite ingest --resume` / `--retry-failed` para ingesta resumible

2. **Tags + Note Add (v0.3.2/0.3.3)** — RFC completo: `openspec/rfc/active/rfc-tags-and-note-add.md`
   - Tags key:value para todos los filtros jerárquicos
   - Note add para que el agente escriba directo en Cite
   - Roadmap retrieval quality (semantic chunking, re-ranking, hybrid search)

3. **Skill/LSP/MCP para Cite (v0.4/0.5)**
   - Usuario quiere una skill que defina el workflow de Cite
   - Idea: que el modelo lea el archivo y estructure la documentación en Cite
   - Idea: tags como reemplazo de estructura de carpetas
   - Necesita definición arquitectónica más profunda

4. **5 docs que fallaron por 429**
   - 03-mvp-scope, 06-ux-flows, prd_changelog, installation.md, v0.2.0-hierarchical-graph.md
   - Se resolverán automáticamente al cambiar a provider local (Ollama)

### Archivos relevantes:
- `openspec/rfc/active/rfc-embedding-providers.md` — RFC providers pluggable
- `openspec/rfc/active/rfc-tags-and-note-add.md` — RFC tags + retrieval roadmap
- `openspec/rfc/active/EVALUACION_CITE.md` — Evaluación actualizada
- `openspec/improvements/ideas/CITE_Pi_Integration.md` — Info de integración con Pi

---

## PROBLEMAS CONOCIDOS

1. **Gemini 429 rate limit** — 5 docs no se pudieron ingestar. Embedder fix para v0.3.2/0.3.3
2. **durable_lock stale** — Si el proceso muere mid-ingest, el lock queda en SQLite. Hay que limpiar manualmente:
   ```python
   python -c "
   import sqlite3, os
   db = os.path.expanduser('~/AppData/Roaming/cite/cite.db')
   conn = sqlite3.connect(db)
   conn.execute('DELETE FROM durable_locks')
   conn.execute('DELETE FROM ingest_backlog')
   conn.commit(); conn.close()
   "
   ```
3. **Doc desincronizada** — 8/13 checks outdated en agent-usage-guide.md (features planificadas documentadas como existentes)
4. **health tarda 1.3s** — llama a Gemini cada vez

---

## COMANDOS ÚTILES

```bash
# Build release
cargo build --release

# Tests
cargo test

# Ingest con jerarquía
CITE_BUILD_HIERARCHY=true ./target/release/cite.exe ingest <file> --no-banner

# Verificar DB
python -c "
import sqlite3, os
db = os.path.expanduser('~/AppData/Roaming/cite/cite.db')
conn = sqlite3.connect(db)
c = conn.cursor()
c.execute('SELECT COUNT(*) FROM documents WHERE status=\"ready\"')
print(f'Docs: {c.fetchone()[0]}')
c.execute('SELECT COUNT(*) FROM chunks')
print(f'Chunks: {c.fetchone()[0]}')
c.execute('SELECT COUNT(*) FROM topics')
print(f'Topics: {c.fetchone()[0]}')
c.execute('SELECT COUNT(*) FROM concepts')
print(f'Concepts: {c.fetchone()[0]}')
conn.close()
"

# Check docs
./target/release/cite.exe check-docs openspec/guides/ --recursive --no-banner

# Evaluate
./target/release/cite.exe evaluate --no-banner --json
```
