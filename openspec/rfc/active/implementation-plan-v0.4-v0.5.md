# Implementation Plan: v0.4.x Foundation + v0.5 Cite Agent Interface

**Status:** Draft — pending review
**Author:** el Gentleman + rikar
**Created:** 2026-06-06
**Scope:** Reviewable plan, no code changes yet
**Sources reviewed:**
- `rfc-cite-v1-skill-lsp.md`
- `release-scope-v0.4-line.md`
- `review-comments-v0.5-rfcs.md`
- `rfc-tags-and-note-add.md`
- `rfc-embedding-providers.md`
- `rfc-auto-docs-sync.md`
- `rfc-front-lobe-engine.md`
- `rfc-notes-hybrid.md`
- `rfc-cite-pi-integration.md`
- `rfc-rag-benchmark-framework.md`
- `RAG_Benchmark_Guide.md`
- `CITE_Pi_Integration.md`

---

## Part 1 — Decisions Resolved

These sixteen decisions were open across the RFCs. Each now has a recommendation with rationale.

### D1. v0.5 bridge default

**Decision: CLI JSON + Pi skill first. MCP optional. Real LSP deferred.**

Rationale:
- CLI JSON keeps standalone Cite independent and is the lowest implementation risk.
- Pi skill gives immediate agent behavior improvement with no daemon requirement.
- MCP or Pi extension bridge can follow once the contract is proven stable.
- Real LSP has editor-specific concepts (`textDocument/didOpen`, etc.) that are premature for Cite's agent-facing needs.
- This is **contract-first, not transport-first**.

Reference: `rfc-cite-v1-skill-lsp.md` § "Recommended v0.5 default".

### D2. "LSP" literal vs LSP-like protocol

**Decision: LSP-like protocol semantics. Not a real LSP server.**

Rationale:
- Cite's immediate need is an agent/tool protocol: context, read, search, notes, diagnostics, tags, trace.
- LSP's document synchronization model is not relevant to Cite's retrieval-only paradigm.
- "LSP-like" means: stable request/response shapes, capability negotiation, and workflow semantics — but implemented as CLI JSON + documented contract, not as a `langserver` binary.
- A real LSP becomes relevant only if editor integration (VS Code, Neovim) justifies LSP-specific semantics.

### D3. Skill distribution

**Decision: Both `.pi/skills/cite/SKILL.md` and `docs/agent-skill.md`.**

Rationale:
- `.pi/skills/cite/SKILL.md` is the primary Pi-native artifact.
- `docs/agent-skill.md` is a generic non-Pi version so Cite can describe its agent contract to other harnesses (Claude Code, Cursor, OpenCode, etc.).
- The two files share the same workflow contract; the Pi version adds Pi-specific loading instructions.
- Maintenance cost is low if both reference a shared contract section.

### D4. Required metadata for v0.5 evidence notes

**Decision: Required fields for agent-written evidence notes:**

| Field | Required | Notes |
|-------|----------|-------|
| `title` | yes | Short, searchable |
| `source_kind` | yes | `note` (vs `document`) |
| `workspace` | yes | Via `--workspace` or `name_project` meta key |
| `tag` | yes, at least one | e.g. `tag:decision`, `tag:bugfix`, `tag:pattern` |
| `body` | yes | The actual note content |

**Recommended but not required:**

| Field | Notes |
|-------|-------|
| `topic` / `concept` | Hierarchy placement; defaults to "Uncategorized" if omitted |
| `agent` | Which agent wrote the note |
| `source` | Provenance (e.g. `session:2026-06-06`) |
| `decision` / `behavior` | Semantic label for Evidence Protocol |

Rationale: title + source_kind + workspace + tag + body is the minimum that makes notes searchable, distinguishable, and filterable. topic/concept is valuable but auto-creates if missing, so it doesn't need to block.

### D5. Schema stability promise

**Decision: Documented field contracts + golden fixtures for v0.5. Formal JSON Schema for v1 candidates.**

Rationale:
- Exact JSON Schema files are expensive to maintain when the contract is still evolving.
- Documented field contracts (field name, type, required/optional, semantic description) give integrators enough to build against.
- Golden fixtures (sample JSON outputs for each command) are testable regression artifacts: if the output shape changes, the fixture breaks.
- When a command graduates to `stable-v1-candidate`, it gets a formal JSON Schema file as part of the v1 stabilization process.
- This is the minimum viable contract discipline without premature formalization.

### D6. Move `rfc-auto-docs-sync.md` out of active

**Decision: Move to `openspec/rfc/completed/`.**

Rationale:
- Phase 1 (MVP) is implemented and the RFC itself says so.
- Phase 2 (semantic comparison) and Phase 3 (CI integration) are tracked as future optional slices in `rfc-tags-and-note-add.md` and `release-scope-v0.4-line.md`.
- Keeping implemented RFCs in `active/` creates noise and misleads about what's actually pending.
- The `rfc-tags-and-note-add.md` tag-based approach supersedes Phase 2/3 tracking in the original RFC.

### D7. Minimum fixtures for v0.5 validation

**Decision: 10 golden fixtures minimum.**

| Fixture | Purpose |
|---------|---------|
| 1. ask → context pack | Agent retrieves context for a question |
| 2. ask → retrieve → read | Agent expands a specific citation |
| 3. search with tag filter | Agent filters by workspace/status |
| 4. note add (minimal) | Agent persists a decision with required metadata |
| 5. note add (full) | Agent persists with topic/concept/tags/workspace |
| 6. source_kind filter | Agent distinguishes notes from documents in retrieval |
| 7. doctor diagnostic | Agent detects provider/index/corpus issues |
| 8. low confidence failure | Agent handles `no_results` or `insufficient_context` |
| 9. stale embeddings | Agent detects model mismatch and recommends reembed |
| 10. planned-only content | Agent skips `status:planned` when asking about implemented |

Each fixture is a JSON file with: input command, expected output shape (fields present, types), and expected result_kind/status. These fixtures are the contract's test suite.

---

## Part 1B — Architectural Model Summary

The unified model uses the **tree metaphor**:

```
Database (trunk)
  └─ Workspace: aiharness (branch)
       └─ Document: "API Reference" (sub-branch)
       │    └─ Chunk 1 [tag:jwt, tag:problem]
       │    └─ Chunk 2 [tag:jwt, tag:solution]
       │    └─ Chunk 3 [tag:password]
       │
       └─ Document: "Architecture Decisions" (sub-branch)
            └─ Chunk 4 [tag:jwt, tag:decision]
            └─ Chunk 5 [tag:provider, tag:local]

  └─ Workspace: other-project (branch)
       └─ Document: "Security Policy" (sub-branch)
            └─ Chunk 6 [tag:jwt, tag:policy]
```

**Tree metaphor:**
- **Trunk** = database (single source of truth)
- **Branches** = workspaces/projects
- **Sub-branches** = documents (physical or virtual)
- **Leaves** = chunks (the actual information units)

**Key properties:**
- Documents are containers (physical or virtual)
- Chunks carry tags as **key:value pairs** (e.g. `tag:jwt`, `status:solved`, `priority:high`)
- Documents inherit tags from their chunks automatically
- Tags replace the old Topic/Concept hierarchy entirely — no more topic_id/concept_id
- Tags are **free-form**: the agent assigns whatever key:value pairs make sense
- Engine only reserves 4 keys: `workspace`, `type`, `session`, `source_kind`
- All other keys (`tag`, `status`, `priority`, `confidence`, etc.) are agent-managed
- Search by tag → finds documents → searches within (reduces comparisons)
- Search without tag → searches all chunks (full semantic)
- Context comes from document container + chunk_index (ordering within document)

**Tag format:**
- Tags are `key:value` pairs, NOT nested (no `tag:topic:jwt`)
- Examples: `tag:jwt`, `status:implemented`, `workspace:aiharness`, `type:prd`
- Multiple tags per chunk, multiple values per key allowed
- Agent decides the tagging vocabulary for their project

---

### D8. Unified document model

**Decision: Documents and notes are unified. A "document" can be physical (file_path) or virtual (no file_path). Tags replace hierarchy. Documents and notes are the same entity with different `source_kind` tags.**

Rationale:
- Physical documents (ingested from files) and virtual documents (written by agents) are the same entity
- A document is a container for chunks, with optional file_path metadata
- Tags on chunks provide flexible grouping — no more Topic/Concept hierarchy
- Documents inherit tags from chunks automatically
- Search by tag → finds documents → searches within (reduces comparisons)
- `source_kind:document` vs `source_kind:note` distinguishes origin, NOT entity type
- This model works for both "note add" (atomic) and "doc write" (full document)
- Tree metaphor: trunk=DB, branches=workspaces, sub-branches=documents, leaves=chunks

### D9. Tags replace hierarchy for retrieval

**Decision: Tags on chunks replace Topic → Concept hierarchy entirely. Documents inherit tags from chunks. Tags are key:value pairs assigned by the agent.**

Rationale:
- The 2-level hierarchy (Topic/Concept) was too rigid and didn't capture semantic relationships
- Tags are more flexible: a chunk can have `tag:jwt`, `tag:problem`, `tag:auth` simultaneously
- Tags are key:value pairs, NOT nested: `tag:jwt`, `status:solved`, `priority:high`
- NO `tag:topic:jwt` or `tag:concept:jwt` — those old concepts don't exist anymore
- The agent decides the tagging vocabulary; engine only reserves 4 keys
- Documents inherit all tags from their chunks → fast lookup by tag at document level
- Search by tag reduces the comparison space
- The document's `chunk_index` preserves ordering within the document for context expansion
- This solves the "problem" and "solution" being in different branches of the hierarchy

### D10. `--to` flag for note add

**Decision: Single `--to <name>` flag that auto-creates virtual documents if they don't exist.**

Rationale:
- `--to auth-decisions` → finds or creates document "auth-decisions"
- No need for separate `--new` and `--to` flags
- Agent doesn't need to create documents first; auto-create on first write
- Explicit `cite doc create` available if agent wants to pre-create an empty document

### D11. `cite doc write` for full documents

**Decision: `cite doc write --to <name> --stdin` writes a full document that gets chunked automatically.**

Rationale:
- `note add` is for atomic chunks (small, single piece of information)
- `doc write` is for full documents (gets chunked, tags are agent-assigned)
- If document already exists in workspace, agent gets conflict JSON (append or rename)
- The agent chooses the right command based on what they're writing

### D12. Auto-tag design

**Decision: Workspace from config/git detection, session from agent context, type from source.**

| Tag | Source | Auto? |
|-----|--------|-------|
| `workspace:<name>` | Config field or git repo root name | Yes |
| `type:note` | For agent-written documents | Yes |
| `type:<category>` | Path patterns during ingest (`prd`, `spec`, `rfc`) | Yes |
| `session:<id>` | Agent session context (if available) | Yes |
| `source_kind:document` | All documents | Yes |
| Custom tags | Agent provides via `--tag` | Explicit |

Workspace detection priority:
1. Explicit `workspace` field in config.toml
2. Git repo root directory name
3. Current working directory name

### D13. embed_batch strategy

**Decision: Provider trait includes `batch_strategy()` hint for adaptive behavior.**

```rust
pub enum BatchStrategy {
    Native,                        // Provider supports native batch
    RateLimited { max_concurrent: usize, delay_ms: u64 },  // Cloud with rate limits
    Sequential,                    // Simple sequential, no rate concerns
}
```

| Provider | Strategy | Notes |
|----------|----------|-------|
| Ollama | Native | HTTP batch API |
| Gemini | RateLimited | No native batch, needs backoff |
| OpenAI-compatible | Native | Supports batch |

### D14. Reserved tag keys

**Decision: Engine enforces a set of reserved tag keys that cannot be used freely.**

Reserved keys (engine-managed):
- `workspace` — project context (auto-detected: config → git root name → CWD name)
- `type` — document type (note, prd, spec, rfc, etc. — auto-tagged from path patterns)
- `session` — agent session ID (auto from context if available)
- `source_kind` — `document` vs `note` distinction (auto on ingest/write)

All other keys are free-form (agent/user can invent: `tag:auth`, `tag:jwt`, `status:implemented`, `priority:high`, `confidence:low`, etc.)

### D15. JOIN on tags table

**Decision: Separate tags table with JOIN is the correct design. Impact is minimal. `source_kind` is a reserved tag, not a column.**

Investigation findings:
- Current retrieval pipeline already does 3-5 JOINs (embeddings, chunks, documents, topics, concepts)
- The bottleneck is in-memory vector ranking (cosine similarity in Rust), not SQL fetch
- Adding a tags JOIN has negligible impact on performance
- Tags filtering can REDUCE the candidate set, improving overall performance
- Index on `(key, value)` makes the JOIN efficient
- `source_kind` lives in the tags table as a reserved key, keeping the documents table clean
- All metadata (workspace, type, session, source_kind) is uniform: tags

### D16. Document lifecycle tracking

**Decision: Cite tracks document lifecycle — change detection, staleness, and feature status — through engine-managed metadata and agent-managed status tags.**

The two problems:
1. **Change tracking**: Documentos que cambian pero no sabés dónde cambió ni cuándo
2. **Feature status**: No sabés si algo está implementado, en progreso, o planeado

**Engine-managed metadata (automatic):**

| Field | Location | Source |
|-------|----------|--------|
| `ingested_at` | chunk/document metadata | Timestamp of last ingest/re-ingest |
| `source_hash` | document metadata | Hash of source file content at ingest time |
| `file_modified_at` | document metadata | File system mtime at ingest time |

**Agent-managed tags (explicit):**

| Tag | Meaning | Example |
|-----|---------|---------|
| `status:implemented` | Feature exists and works | Agent verified after testing |
| `status:in-progress` | Being worked on | Current sprint work |
| `status:planned` | Documented but not built | PRD spec, not yet coded |
| `status:deprecated` | No longer relevant | Old architecture doc |
| `status:changed` | Content changed since last review | Auto-set on re-ingest with different hash |

**Freshness detection workflow:**
1. Ingest reads file, computes hash, stores `source_hash` + `ingested_at`
2. On re-ingest: compare new hash vs stored hash
3. If different → re-chunk, update hash, set `status:changed` tag, update `ingested_at`
4. If same → skip (no work needed)

**Agent queries enabled:**
```bash
# What changed recently?
cite list --tag status:changed

# What's actually implemented vs planned?
cite search "auth" --tag status:implemented
cite search "auth" --tag status:planned

# What's stale? (ingested > 30 days ago)
cite list --stale-days 30

# Freshness report for agent
cite doctor --freshness
```

**Trust indicators per chunk in retrieval output:**
```json
{
  "chunk_id": "c-042",
  "text": "JWT tokens rotate every 15 minutes...",
  "source_kind": "document",
  "status": "implemented",
  "ingested_at": "2026-06-01",
  "source_freshness": "current",
  "confidence": "high"
}
```

This gives agents the ability to:
- Distinguish implemented vs planned behavior
- Detect when retrieved info might be stale
- Trust but verify: know when the source was last checked
- Avoid presenting planned features as existing behavior

---

## Part 2 — v0.4.x Work Units

The v0.4.x line is a **release train**, not a hard version map. The important invariant is dependency order: metadata/providers/diagnostics before notes and retrieval quality; v0.5 only after the agent-facing contract can be credibly documented and validated.

### v0.4.0 — Metadata + Local Provider Foundation

**Theme:** Tags (with inheritance) + Ollama provider MVP + Lifecycle tracking foundation

| WU | Deliverable | Est. scope | Depends on |
|----|-------------|------------|------------|
| WU-001 | `tags` table + SQLite migration (tags on chunks + inherited to documents) | 1 file, ~100 LOC | — |
| WU-002 | `cite tag set/get/rm` CLI | 1 file, ~120 LOC | WU-001 |
| WU-003 | `--tag` filter on `search`, `retrieve`, `context`, `list` (searches documents by inherited tags, then chunks within) | 2-3 files, ~180 LOC | WU-002 |
| WU-004 | Path-based auto-tags during `ingest` + workspace detection (config/git/CWD) | 1 file, ~80 LOC | WU-001 |
| WU-005 | `check-docs` reads `<!-- tag:status=planned -->` | 1 file, ~40 LOC | WU-001 |
| WU-005B | Reserved tag keys enforcement (`workspace`, `type`, `session`, `source_kind`) | 1 file, ~40 LOC | WU-001 |
| WU-005C | `source_hash` + `ingested_at` + `file_modified_at` stored on document metadata during ingest | 1 file, ~60 LOC | WU-001 |
| WU-005D | Change detection: re-ingest compares hash, sets `status:changed` tag if different | 1 file, ~50 LOC | WU-005C |
| WU-006 | `OllamaProvider` over local HTTP | 1 file, ~200 LOC | — |
| WU-007 | `embed_batch` on provider trait with `BatchStrategy` enum (Native/RateLimited/Sequential) | 1 file, ~60 LOC | — |
| WU-008 | Provider factory: `gemini`, `openai-compatible`, `ollama` | 1 file, ~80 LOC | WU-006 |
| WU-009 | Config fields: `endpoint`, `dimensions`, `device`, `batch_size`, `workspace` | 1 file, ~80 LOC | WU-008 |
| WU-010 | `cite health` provider details + latency | 1 file, ~60 LOC | WU-008 |

**Review workload:** ~1,130 LOC across 13 work units. Each unit is independently reviewable.

### v0.4.1 — Migration + Diagnostics

**Theme:** Reembed, retry-failed, resume, doctor, actionable errors

| WU | Deliverable | Est. scope | Depends on |
|----|-------------|------------|------------|
| WU-011 | `cite ingest --reembed` (row swap in transaction: temp table → re-embed → delete old → insert new → commit) | 1 file, ~150 LOC | WU-008 |
| WU-012 | `cite ingest --retry-failed` (re-attempt docs in `failed` state) | 1 file, ~80 LOC | WU-011 |
| WU-013 | `cite ingest --resume` (continue interrupted ingest; implies `--retry-failed`; stale-lock auto-recovery) | 1 file, ~100 LOC | WU-011 |
| WU-014 | `cite doctor` (config, provider, DB, embeddings, retrieval, freshness — JSON-optimized for agents) | 1 file, ~200 LOC | WU-008, WU-010, WU-005C |
| WU-015 | Actionable error messages (provider errors catalog with remediation steps) | 1 file, ~100 LOC | WU-008 |

**Flag interaction rules (documented in help text):**
- `--reembed` is exclusive (cannot combine with other flags)
- `--resume` implies `--retry-failed` for failed docs
- `--force` clears stale locks, combines with any flag
- `--retry-failed` only re-attempts `failed` state docs

**Reembed atomic swap strategy:**
```sql
BEGIN;
  CREATE TEMP TABLE chunks_new AS SELECT ... FROM chunks WHERE document_id = ?;
  -- re-embed into chunks_new with new provider
  DELETE FROM chunks WHERE document_id = ?;
  INSERT INTO chunks SELECT * FROM chunks_new;
  DROP TABLE chunks_new;
COMMIT;
```

**Doctor output is agent-optimized JSON:**
```json
{
  "config": { "found": true, "path": "~/.config/cite/config.toml" },
  "provider": { "name": "ollama", "model": "nomic-embed-text", "status": "ok", "latency_ms": 12 },
  "database": { "documents": 20, "chunks": 186, "failed": 0 },
  "embeddings": { "model_mismatch": true, "db_model": "gemini-embedding-001", "current_model": "nomic-embed-text" },
  "freshness": { "stale_count": 3, "recently_changed": 2 },
  "warnings": ["Model mismatch: run 'cite ingest --reembed'"],
  "errors": [],
  "recommended_actions": ["cite ingest --reembed"]
}
```

**Review workload:** ~630 LOC across 5 work units.

### v0.4.2 — Agent Knowledge Capture (Unified Document Model)

**Theme:** Note add + doc write + unified document model with tags + lifecycle tracking

| WU | Deliverable | Est. scope | Depends on |
|----|-------------|------------|------------|
| WU-016 | Unified document model: documents can be physical (file_path) or virtual (no file_path), no hierarchy | 1 file, ~60 LOC | WU-001 |
| WU-017 | `--to <name>` flag: find or auto-create virtual document in workspace | 1 file, ~80 LOC | WU-016 |
| WU-018 | `cite note add` (atomic: adds single chunk to document via `--to`, with `--tag` flags) | 1 file, ~150 LOC | WU-016, WU-017 |
| WU-018B | `cite doc write` (full document: checks workspace for existing doc, warns if exists, asks append/rename, chunks content) | 1 file, ~200 LOC | WU-016, WU-017 |
| WU-019 | Tag inheritance: document automatically inherits tags from its chunks | 1 file, ~60 LOC | WU-018, WU-001 |
| WU-020 | `cite doc create` (explicit empty document creation, optional) | 1 file, ~30 LOC | WU-016 |
| WU-021 | `--source_kind` filter on search/retrieve/context (document vs note, via reserved tag) | 1 file, ~50 LOC | WU-003, WU-005B |
| WU-021B | Freshness queries: `--stale-days N`, `--recently-changed`, freshness indicators in retrieval output | 1 file, ~80 LOC | WU-005C, WU-005D |

**Review workload:** ~710 LOC across 8 work units.

**Design notes:**
- `cite note add "text" --to auth-doc --tag problem --tag jwt` → adds chunk, inherits tags to doc
- `cite doc write --to auth-doc --stdin` → checks if doc exists in workspace; if yes: warns and asks agent (append or rename); if no: creates, chunks, done
- Tags are key:value pairs assigned by the agent: `tag:jwt`, `status:solved`, `priority:high`
- NO nested tags like `tag:topic:jwt` — the old Topic/Concept hierarchy is gone
- `source_kind` is a reserved tag, not a column — all metadata lives in tags
- Documents are the "branches", chunks are the "leaves", tags provide semantic grouping
- Search by tag → finds documents first → searches within (reduces comparisons)
- Lifecycle: `status:implemented` / `status:planned` / `status:changed` tags + `ingested_at` freshness

**`doc write` conflict resolution flow:**
```
cite doc write --to auth-doc --stdin
  → Check workspace for document named "auth-doc"
  → If NOT exists: create document, chunk content, done
  → If EXISTS: return JSON warning to agent:
    {
      "conflict": true,
      "existing_document": "auth-doc",
      "chunk_count": 12,
      "options": ["append", "rename"],
      "message": "Document 'auth-doc' already exists in workspace 'aiharness' with 12 chunks. Use 'cite note add --to auth-doc' to append, or choose a different name."
    }
```

### v0.4.3 — Docs Verification Polish

**Theme:** Smart comparison + metadata headers

| WU | Deliverable | Est. scope | Depends on |
|----|-------------|------------|------------|
| WU-022 | Regex/semantic comparison for dynamic output | 1 file, ~100 LOC | — |
| WU-023 | Metadata headers on behavioral docs (`verified_with`, `last_verified`) | 1 file, ~30 LOC | — |

**Review workload:** ~130 LOC. Optional slice; can ship with v0.4.2 if small enough.

### v0.4.4 — Advanced Providers (if desired)

| WU | Deliverable | Est. scope | Depends on |
|----|-------------|------------|------------|
| WU-024 | `OnnxProvider` with `ort` crate + CUDA support | 1 file, ~250 LOC | WU-008 |
| WU-025 | `HuggingFaceProvider` (Inference API) | 1 file, ~150 LOC | WU-008 |

**Review workload:** ~400 LOC. Deferred unless local demand is clear.

### v0.4.5 — Setup + Benchmark UX

| WU | Deliverable | Est. scope | Depends on |
|----|-------------|------------|------------|
| WU-026 | `cite setup` interactive wizard | 1 file, ~200 LOC | WU-008 |
| WU-027 | Provider recommendation based on hardware | 1 file, ~80 LOC | WU-026 |
| WU-028 | Provider benchmark (compare on current corpus) | 1 file, ~120 LOC | WU-008 |

**Review workload:** ~400 LOC.

### v0.4.6 — Semantic Chunking

| WU | Deliverable | Est. scope | Depends on |
|----|-------------|------------|------------|
| WU-029 | Heading-boundary-aware chunker | 1 file, ~200 LOC | — |
| WU-030 | Sentence-boundary detection | 1 file, ~80 LOC | WU-029 |
| WU-031 | Variable chunk sizes (300-800 chars) | 1 file, ~60 LOC | WU-029 |
| WU-032 | Preserve code blocks as coherent chunks | 1 file, ~60 LOC | WU-029 |
| WU-033 | Reembed validation after chunking change | 1 file, ~40 LOC | WU-011, WU-029 |

**Review workload:** ~440 LOC.

### v0.4.7 — Re-ranking

| WU | Deliverable | Est. scope | Depends on |
|----|-------------|------------|------------|
| WU-034 | Cross-encoder re-ranker integration | 1 file, ~200 LOC | — |
| WU-035 | Two-stage retrieval: vector candidates → re-rank | 1 file, ~100 LOC | WU-034 |
| WU-036 | Benchmark: current vs re-ranked on golden dataset | 1 file, ~80 LOC | WU-034 |

**Review workload:** ~380 LOC.

### v0.4.8–v0.4.10 — Hybrid Search

| WU | Deliverable | Est. scope | Depends on |
|----|-------------|------------|------------|
| WU-037 | FTS5 index over chunk text | 1 file, ~100 LOC | — |
| WU-038 | Vector + BM25 hybrid scoring | 1 file, ~120 LOC | WU-037 |
| WU-039 | Configurable scoring weights | 1 file, ~40 LOC | WU-038 |
| WU-040 | Benchmark: pure vector vs hybrid | 1 file, ~80 LOC | WU-038 |
| WU-041 | Tests for exact technical-token queries | 1 file, ~60 LOC | WU-038 |

**Review workload:** ~400 LOC.

### v0.4.x Totals

| Slice | Work Units | Est. LOC | Risk |
|-------|-----------|----------|------|
| v0.4.0 | 13 | ~1,130 | low |
| v0.4.1 | 5 | ~630 | low |
| v0.4.2 | 8 | ~710 | low |
| v0.4.3 | 2 | ~130 | low |
| v0.4.4 | 2 | ~400 | medium (ONNX crate) |
| v0.4.5 | 3 | ~400 | low |
| v0.4.6 | 5 | ~440 | medium (chunking migration) |
| v0.4.7 | 3 | ~380 | medium (model integration) |
| v0.4.8–10 | 5 | ~400 | medium (scoring tuning) |
| **Total** | **46** | **~4,620** | |

---

## Part 3 — v0.5 Work Units (Cite Agent Interface)

v0.5 starts after v0.4.0–v0.4.2 are stable (tags, providers, diagnostics, notes, source_kind).

### Phase A — Skill and Workflow Contract

| WU | Deliverable | Est. scope | Depends on |
|----|-------------|------------|------------|
| WU-050 | `.pi/skills/cite/SKILL.md` — full Cite usage skill | 1 file, ~300 LOC (markdown) | v0.4.0–v0.4.2 |
| WU-051 | `docs/agent-skill.md` — generic non-Pi version | 1 file, ~250 LOC | WU-050 |
| WU-052 | Command decision table (context vs retrieve vs read vs search vs trace vs note add vs doc write vs doctor vs tag) | 1 section in skill | WU-050 |
| WU-053 | Evidence Protocol section in skill (write → retrieve → cite loop) | 1 section in skill | WU-050, WU-018 |
| WU-054 | Failure handling workflow (low confidence, stale embeddings, planned-only, provider mismatch) | 1 section in skill | WU-050 |

**Evidence Protocol — concrete definition:**

| Element | Definition |
|---------|------------|
| Save triggers | decisions, bug fixes, patterns/conventions, milestone summaries, architecture choices |
| Minimum fields | title, source_kind=note, workspace, tag (at least one), body |
| Recommended fields | topic (as tag), concept (as tag), agent, source, decision, behavior |
| Retrieval rule | Notes and documents mix by default; `--tag source_kind:note` filters |
| Citation rule | Notes cite same as documents: chunk_id + document_id |
| Update policy | Append-only by default; explicit update only with `cite note update` (future) |

**Review workload:** ~550 LOC markdown. All prose, no code.

### Phase B — Stable Schema Contract

| WU | Deliverable | Est. scope | Depends on |
|----|-------------|------------|------------|
| WU-060 | Documented field contracts for all v1-candidate commands | 1 doc, ~400 LOC | WU-050 |
| WU-061 | **Shape fixture**: context pack JSON structure | 1 file, ~50 LOC | v0.4.0 |
| WU-062 | **Shape fixture**: citation/chunk read JSON structure | 1 file, ~50 LOC | v0.4.0 |
| WU-063 | **Shape fixture**: search result JSON structure | 1 file, ~50 LOC | v0.4.0 |
| WU-064 | **Shape fixture**: note add response JSON structure | 1 file, ~50 LOC | v0.4.2 |
| WU-065 | **Shape fixture**: diagnostic result JSON structure (agent-optimized) | 1 file, ~50 LOC | v0.4.1 |
| WU-066 | **Shape fixture**: trace result JSON structure | 1 file, ~50 LOC | v0.4.0 |
| WU-067 | **Shape fixture**: tag/workspace listing JSON structure | 1 file, ~50 LOC | v0.4.0 |
| WU-068 | Stability labels: `stable-v1-candidate`, `experimental`, `internal` | 1 section in field contracts doc | WU-060 |

**Shape fixtures vs workflow fixtures:**
- **Shape fixtures** (Phase B): validate JSON schema — fields present, types correct, structure valid
- **Workflow fixtures** (Phase E): validate behavior — input → expected output → expected agent action
- They are complementary, not interchangeable

**Review workload:** ~800 LOC across 9 work units. Mostly JSON fixtures + prose.

### Phase C — v1 Direction Architecture

| WU | Deliverable | Est. scope | Depends on |
|----|-------------|------------|------------|
| WU-070 | `openspec/architecture/cite-v1-agent-interface.md` skeleton | 1 file, ~200 LOC | WU-050, WU-060 |
| WU-071 | Stable CLI/API surface candidates with stability labels | 1 section | WU-068 |
| WU-072 | Migration path from v0.4.x features to v1 contract | 1 section | WU-070 |
| WU-073 | Compatibility promises for CLI, Pi skill, MCP/JSON-RPC bridge | 1 section | WU-070 |

**v1 contract surface (preliminary):**

| Command | Stability | Output |
|---------|-----------|--------|
| `cite context` | stable-v1-candidate | Context pack JSON |
| `cite retrieve` | stable-v1-candidate | Chunk array JSON |
| `cite read` | stable-v1-candidate | Single chunk JSON |
| `cite search` | stable-v1-candidate | Ranked results JSON |
| `cite note add` | stable-v1-candidate | Note confirmation JSON |
| `cite doc write` | experimental | Document write confirmation JSON |
| `cite doctor` | stable-v1-candidate | Diagnostic JSON |
| `cite tag set/get/rm` | stable-v1-candidate | Tag operations JSON |
| `cite trace` | experimental | Trace explanation JSON |
| `cite ingest` | internal | No stable contract |
| `cite health` | internal | No stable contract |

**v1 one-sentence definition:**
> Cite v1 is a local, CLI-first evidence and context substrate for agents, with stable retrieval, citation, note, diagnostic, lifecycle tracking, and integration contracts — all outputs JSON-optimized for agent consumption.

**Review workload:** ~400 LOC. Architecture prose.

### Phase D — Smallest Bridge

| WU | Deliverable | Est. scope | Depends on |
|----|-------------|------------|------------|
| WU-080 | `--json` flag on all v1-candidate commands (agent-optimized JSON output, NOT human-readable mirror) | 2-3 files, ~200 LOC | WU-060 |
| WU-081 | Pi skill wired to CLI JSON (calls cite CLI, parses JSON) | 1 file, ~100 LOC | WU-050, WU-080 |
| WU-082 | End-to-end: agent loads skill → retrieves → cites → notes → diagnoses | manual test | WU-081 |

**JSON output design principle:**
`--json` produces agent-optimized output, NOT a mirror of human-readable text. The JSON includes metadata that text output omits:

```bash
# Human-readable:
cite context "JWT auth" --limit 3
# → formatted text with inline citations

# Agent-optimized JSON:
cite context "JWT auth" --limit 3 --json
# → {
#   "query": "JWT auth",
#   "citations": [...],
#   "confidence": 0.82,
#   "chunks": [...],
#   "source_kinds": ["document", "note"],
#   "freshness": { "stale_count": 0 },
#   "recommended_action": null
# }
```

Every JSON response includes:
- The data the agent asked for
- Confidence/reliability indicators
- Freshness/lifecycle metadata
- `recommended_action` when the agent should do something next (e.g., reembed, check doctor)

**Review workload:** ~300 LOC. The bridge is thin by design.

### Phase E — Workflow Validation and Benchmark

| WU | Deliverable | Est. scope | Depends on |
|----|-------------|------------|------------|
| WU-090 | 10 **workflow fixtures** (input + expected output + expected agent action) | 10 files, ~300 LOC total | WU-060–WU-067 |
| WU-091 | Scripted agent workflow: ask → retrieve → cite → filter → note → diagnose | 1 script, ~150 LOC | WU-082, WU-090 |
| WU-092 | Failure case suite: low confidence, stale embeddings, planned-only, provider mismatch | 4 fixtures, ~120 LOC | WU-090 |
| WU-093 | Minimum metrics report: Context Precision, Recall, Hit Rate @K, latency, faithfulness | 1 script, ~100 LOC | WU-091 |

**Workflow fixtures (distinct from Phase B shape fixtures):**

| # | Scenario | Input | Expected behavior |
|---|----------|-------|-------------------|
| 1 | Context pack retrieval | `cite context "how does auth work" --json` | Returns citations, confidence ≥ 0.7, chunks with source_kind |
| 2 | Retrieve + read expansion | `cite retrieve ... → cite read <chunk_id>` | Full chunk with metadata, tags, freshness |
| 3 | Tag-filtered search | `cite search "jwt" --tag status:implemented --json` | Only implemented docs, no planned content |
| 4 | Note add (minimal) | `cite note add "decision" --to notes --tag tag:decision --json` | Returns chunk_id, document_id, inherited tags |
| 5 | Note add (full) | `cite note add ... --workspace X --tag status:implemented --json` | All metadata populated, source_kind=note |
| 6 | Source kind filter | `cite context "decisions" --tag source_kind:note --json` | Only notes, no documents |
| 7 | Doctor diagnostic | `cite doctor --json` | Provider status, freshness, warnings, recommended_actions |
| 8 | Low confidence failure | `cite context "obscure query" --json` | confidence < 0.3, recommended_action: "try broader query" |
| 9 | Stale embeddings | `cite doctor --json` with model mismatch | warnings: model mismatch, recommended_action: "cite ingest --reembed" |
| 10 | Planned-only content | `cite search "feature" --tag status:planned --json` | Returns results but agent skill says "not yet implemented" |

**Review workload:** ~670 LOC across 4 work units.

### v0.5 Totals

| Phase | Work Units | Est. LOC | Risk |
|-------|-----------|----------|------|
| A — Skill | 5 | ~550 | low |
| B — Schema | 9 | ~800 | low |
| C — v1 Arch | 4 | ~400 | low |
| D — Bridge | 3 | ~300 | low |
| E — Validation | 4 | ~670 | medium (fixture accuracy) |
| **Total** | **25** | **~2,720** | |

---

## Part 4 — Dependency Graph

```
v0.4.0 (tags + ollama + lifecycle)
    ├── v0.4.1 (reembed + doctor) ← needs provider + tags + lifecycle
    ├── v0.4.2 (note add) ← needs tags + source_kind + lifecycle
    │       └── v0.5 Phase A (skill) ← needs notes + source_kind + lifecycle
    │               ├── v0.5 Phase B (schema) ← needs skill contract
    │               ├── v0.5 Phase C (v1 arch) ← needs skill + schema
    │               ├── v0.5 Phase D (bridge) ← needs skill + JSON contract
    │               └── v0.5 Phase E (validation) ← needs bridge + fixtures
    ├── v0.4.3 (docs polish) ← independent, can ship with v0.4.2
    ├── v0.4.4 (advanced providers) ← needs provider factory
    ├── v0.4.5 (setup wizard) ← needs provider factory
    ├── v0.4.6 (semantic chunking) ← independent but needs reembed
    ├── v0.4.7 (re-ranking) ← independent
    └── v0.4.8-10 (hybrid search) ← independent
```

**Critical path to v0.5:** v0.4.0 → v0.4.1 → v0.4.2 → v0.5 Phase A → B → C → D → E

**Non-blocking v0.4.x work:** v0.4.3, v0.4.4, v0.4.5, v0.4.6, v0.4.7, v0.4.8–10 can proceed in parallel with v0.5 planning, but v0.5 implementation should not start until v0.4.2 is stable.

**Lifecycle tracking dependency:** WU-005C and WU-005D (ingest metadata + change detection) are part of v0.4.0 but feed into v0.4.1 (doctor freshness) and v0.4.2 (freshness queries, status tags). They should be completed early in the v0.4.0 cycle.

---

## Part 5 — Review Workload Guard

| Group | Work Units | Total Est. LOC | PR strategy |
|-------|-----------|----------------|-------------|
| v0.4.0 | 13 | ~1,130 | 2 PRs: tags+inheritance+lifecycle (WU-001–005D) + ollama (WU-006–010) |
| v0.4.1 | 5 | ~630 | 1 PR |
| v0.4.2 | 8 | ~710 | 1 PR (note add + doc write + freshness unified) |
| v0.4.3 | 2 | ~130 | 1 PR (can combine with v0.4.2) |
| v0.4.6 | 5 | ~440 | 1 PR |
| v0.4.7 | 3 | ~380 | 1 PR |
| v0.4.8–10 | 5 | ~400 | 1–2 PRs |
| v0.5 Phase A | 5 | ~550 | 1 PR (markdown only) |
| v0.5 Phase B | 9 | ~800 | 1–2 PRs |
| v0.5 Phase C | 4 | ~400 | 1 PR |
| v0.5 Phase D | 3 | ~300 | 1 PR |
| v0.5 Phase E | 4 | ~670 | 1 PR |

**Rule:** No PR should exceed 400 changed lines without explicit approval. v0.4.0 should split into tags+lifecycle PR (~400 LOC) and ollama PR (~540 LOC, borderline — may need sub-split).

---

## Part 6 — Acceptance Criteria

### v0.4.x line is complete when:

- [ ] Tags work on chunks with automatic inheritance to documents
- [ ] Tags are key:value pairs (NOT nested), agent-assigned, engine reserves only 4 keys
- [ ] `--tag` filter searches documents by inherited tags, then chunks within
- [ ] Reserved tag keys (`workspace`, `type`, `session`, `source_kind`) are enforced by engine
- [ ] `source_kind` is a reserved tag (not a column), distinguishes documents from notes
- [ ] Workspace auto-detection works (config → git → CWD)
- [ ] No Topic/Concept hierarchy remains — tags replace it entirely
- [ ] Ollama provider works with `embed_batch` and `BatchStrategy` enum
- [ ] `cite health` reports provider details, latency, and batch strategy
- [ ] `cite ingest --reembed` migrates between providers atomically (row swap in transaction)
- [ ] `--reembed` is exclusive, `--resume` implies `--retry-failed`, `--force` clears locks
- [ ] `cite doctor` reports config, provider, DB, embeddings, freshness, and retrieval status (JSON-optimized for agents)
- [ ] `cite note add --to <doc>` creates chunks with tags, inherits to document
- [ ] `cite doc write --to <doc> --stdin` checks workspace for conflicts, warns agent if exists
- [ ] Documents can be physical (file_path) or virtual (no file_path)
- [ ] `source_kind` distinguishes documents from notes via reserved tag
- [ ] Lifecycle tracking: `ingested_at`, `source_hash`, `status:changed` detection on re-ingest
- [ ] Agent can query by freshness (`--stale-days`, `--recently-changed`)
- [ ] Agent can filter by feature status (`status:implemented`, `status:planned`)
- [ ] All provider errors have actionable messages with remediation steps
- [ ] `check-docs` reads markdown tag annotations

### v0.5 is successful when:

- [ ] An agent can load `.pi/skills/cite/SKILL.md` and use Cite correctly without external instruction
- [ ] `docs/agent-skill.md` describes the same contract for non-Pi agents
- [ ] The stable/experimental command surface is documented with stability labels
- [ ] Shape fixtures exist for all v1-candidate command JSON outputs
- [ ] Workflow fixtures validate agent behavior for 10 scenarios
- [ ] `openspec/architecture/cite-v1-agent-interface.md` defines Cite v1 with concrete contract surface
- [ ] CLI JSON output is agent-optimized (not human-readable mirror)
- [ ] Every JSON response includes confidence, freshness, and recommended_action when relevant
- [ ] The agent workflow (ask → retrieve → cite → filter → note → diagnose) passes all 10 workflow fixtures
- [ ] Failure cases (low confidence, stale embeddings, planned-only, provider mismatch) are handled gracefully

### One-sentence v1 definition:

> Cite v1 is a local, CLI-first evidence and context substrate for agents, with stable retrieval, citation, note, diagnostic, lifecycle tracking, and integration contracts — all outputs JSON-optimized for agent consumption.

---

## Part 7 — Recommended RFC Hygiene Actions

Before starting implementation:

| Action | File | Destination |
|--------|------|-------------|
| Move | `rfc-auto-docs-sync.md` | `openspec/rfc/completed/` |
| Move | `EVALUACION_CITE.md` | `openspec/reports/` (evidence, not RFC) |
| Move | `SESSION_CONTEXT_2026-06-06.md` | `openspec/reports/` or archive |
| Add review notes | All active RFCs | Inline `## Review Notes` sections per `review-comments-v0.5-rfcs.md` |
| Create skeleton | — | `openspec/architecture/cite-v1-agent-interface.md` (empty, for Phase C) |
| Clean up | `rfc-tags-and-note-add.md` | Remove Topic/Concept hierarchy references; align with D9 (tags replace hierarchy) |
| Clean up | `rfc-notes-hybrid.md` | Remove topic_id/concept_id references; use tags as key:value pairs only |

---

## Part 8 — Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| v0.4.x scope creep delays v0.5 | high | medium | v0.4.0–v0.4.2 are the critical path; v0.4.3+ can defer |
| Skill becomes prose-only, untestable | medium | high | Pair with shape fixtures (Phase B) and workflow fixtures (Phase E) |
| Bridge duplicates CLI semantics | low | medium | Define one contract and expose through CLI/Pi; no per-transport behavior |
| v1 promise is too vague | medium | medium | Stability labels (`stable-v1-candidate`, `experimental`, `internal`) + concrete contract surface table |
| Note workflow pollutes retrieval | medium | medium | `source_kind` reserved tag + skill defines when not to persist |
| Ollama provider breaks for edge cases | low | low | Sequential fallback default; provider factory isolates each impl |
| Semantic chunking migration breaks existing vectors | medium | high | WU-033 validates reembed after chunking change; keep old vectors during swap |
| Tags model confusion (old hierarchy remnants) | medium | medium | D9 is clear: hierarchy is gone; all docs/code referencing topic_id/concept_id should be cleaned up during v0.4.0 |
| Doc write conflict not handled | low | medium | WU-018B checks workspace, returns conflict JSON to agent, never silently overwrites |
| Lifecycle metadata adds ingest overhead | low | low | Hash computation is cheap (~microseconds per file); `ingested_at` is a timestamp write |
| Agent ignores freshness/status indicators | medium | medium | Skill (Phase A) must define trust workflow: check status before citing, check freshness before trusting

---

## Next Action

1. Review this plan.
2. Confirm or adjust the 16 decisions in Part 1.
3. Approve the work unit breakdown.
4. Execute the RFC hygiene actions (Part 7).
5. Start v0.4.0: WU-001 (tags schema) and WU-006 (Ollama provider) can begin in parallel.
