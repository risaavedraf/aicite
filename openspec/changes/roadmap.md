# AI Cite CLI — MVP Roadmap

## Overview

7 phases to deliver the MVP: a CLI-first semantic document engine that ingests private documents, retrieves cited context through stable CLI commands, and exposes agent-consumable context packs.

## Phases

### Phase 1: Scaffold ✅
**Status**: Complete (commit 6001579)

**What**: Project foundation — workspace, config, storage, CLI skeleton

**Deliverables**:
- Cargo workspace with 9 crates
- Config crate with env/file/flag precedence
- Storage crate with SQLite + WAL + migrations
- CLI crate with `cite health --json`
- Common crate with types, errors, exit codes
- CI pipeline (GitHub Actions)
- README + .env.example

**SDD artifacts**: `docs/sdd/phase-1-scaffold/`

---

### Phase 2: Ingest Pipeline ✅
**Status**: Complete

**What**: Document ingestion — extract text, chunk, embed

**Deliverables**:
- File validation (type, size, path policy)
- Text extraction from PDF, TXT, MD
- Chunking with overlap (800-1200 tokens, 100-200 overlap)
- Embedding generation via configurable provider
- Document lifecycle management (pending → processing → ready → failed)
- `cite ingest <path>` command
- `cite list` and `cite get` commands
- Retry/backoff for failed ingestion
- Partial data cleanup on failure

**Key crates**: `ingest`, `providers`, `storage`, `engine`

**SDD artifacts**: `docs/sdd/phase-2-ingest/`

---

### Phase 3: Retrieval Pipeline ✅
**Status**: Complete

**What**: Vector-first semantic search over indexed documents

**Deliverables**:
- Vector index storage and lookup
- Cosine similarity scoring
- Top-k retrieval with configurable k (1-10, default 5)
- `cite search` command
- `cite retrieve` command
- Source/section/chunk metadata attachment
- Partial-corpus handling (use ready docs only)

**Key crates**: `retrieval`, `graph`, `storage`, `engine`

**SDD artifacts**: `docs/sdd/phase-3-retrieval/`

---

### Phase 4: Context Packs + Citations ✅
**Status**: Complete

**What**: Agent-consumable context packs with citations and traces

**Deliverables**:
- Context pack assembly (context_pack_id, result_kind, citations, metadata)
- Result-kind decision table (context, no_results, insufficient_context)
- Evidence floor and confidence threshold logic
- Citation model (citation_id, document_id, chunk_id, page, offset, text, score)
- `cite context` command
- `cite read` command (citation or chunk lookup)
- `cite trace` command
- Agent instruction template
- Verification disclaimer in output

**Key crates**: `engine`, `cli`, `retrieval`

**SDD artifacts**: `docs/sdd/phase-4-context/`

---

### Phase 5: Durability ✅
**Status**: Complete (commits 682a98c..a100b44)

**What**: Durable locks, rate limits, and backlog management

**Deliverables**:
- Durable ingestion locks (sequential processing)
- Backlog/upsert on lock conflict (`operation_in_progress`)
- `cite ingest --next` / `--queued` for backlog processing
- Rate limiting for retrieval/context (20 req/min per key)
- Durable rate-limit counters (survive CLI restarts)
- `cite retry` command for failed documents
- `cite refresh` command with atomic snapshot swap
- Recovery of interrupted `processing` documents on startup

**Key crates**: `storage`, `engine`, `cli`

**SDD artifacts**: `docs/sdd/phase-5-durability/`

---

### Phase 6: Evaluation ✅
**Status**: Complete (commits 83e44d1..ddb3630)

**What**: Golden dataset and sample corpus for acceptance testing

**Deliverables**:
- Sample corpus (3+ docs, 10+ facts, structured content)
- Golden dataset (8 fixtures minimum):
  - 3 direct-fact cases (3/3 must pass)
  - 2 no-results cases (2/2 must pass)
  - 1 ambiguous query
  - 1 multi-chunk query
  - 1 prompt-injection fixture
- Retrieval quality metric (80% top-5 hit rate)
- Evaluation command/script
- Runtime mode enforcement tests

**Key crates**: `engine`, `cli`, test fixtures

**SDD artifacts**: `docs/sdd/phase-6-evaluation/`

---

### Phase 7: Packaging + Docs ✅
**Status**: Complete

**What**: Release packaging, documentation, and demo preparation

**Deliverables**:
- Reproducible CLI binary builds (cross-platform release workflow)
- Demo corpus (3 documents: architecture, API reference, security policy)
- Complete README with all 12 commands, config, storage paths
- `.env.example` with all provider configs
- Chile privacy law compliance notes
- Provider disclosure in CLI output (`--no-banner` flag)
- Demo acceptance flow (5-minute review) in `docs/demo.md`
- CI/CD pipeline for releases (GitHub Actions)

**Key crates**: all

**SDD artifacts**: `docs/sdd/phase-7-packaging/`

---

### v0.4.0: Tags + Local Provider Foundation ✅ (PR 1-6)
**Status**: PRs #31-#36 implemented and open; PR 7-8 pending

**What**: Tags system + lifecycle tracking + Ollama provider MVP

**Source RFCs**: `rfc-tags-and-note-add.md`, `rfc-embedding-providers.md`

**Deliverables (PR 1-6 ✅)**:
- `tags` table + migration
- `cite tag set/get/rm`
- `--tag` filter on `search`, `retrieve`, `context`, `list`
- Auto-tags on ingest (`source_kind`, `workspace`, `type:*`)
- Lifecycle metadata: `source_hash`, `ingested_at`, `file_modified_at`
- Changed re-ingest: reuse document_id, atomic chunk replacement, `status:changed`
- `check-docs` reads `<!-- tag:status=planned -->`

**Deliverables (PR 7-8 🔲)**:
- `OllamaProvider` over local HTTP
- `embed_batch` + `BatchStrategy` enum
- Provider factory: `gemini`, `openai-compatible`, `ollama`
- Config fields: `endpoint`, `dimensions`, `device`, `batch_size`
- `cite health` provider details + latency

**SDD**: `openspec/changes/active/v0.4.0-tags-lifecycle-ollama/`

---

### v0.4.1: Migration + Diagnostics 📋
**Status**: RFC approved, pending

**What**: Reembed, retry-failed, resume, doctor, actionable errors

**Source RFC**: `rfc-embedding-providers.md`

**Deliverables**:
- `cite ingest --reembed` (atomic row swap between providers)
- `cite ingest --retry-failed` / `--resume` / `--force`
- `cite doctor` (config, provider, DB, embeddings, freshness — agent-optimized JSON)
- Actionable provider error messages with remediation steps

---

### v0.4.2: Agent Knowledge Capture 📋
**Status**: RFC approved, pending

**What**: Note add + doc write + unified document model with tags

**Source RFC**: `rfc-tags-and-note-add.md`

**Deliverables**:
- `cite note add --to <name>` (atomic chunk with tags)
- `cite doc write --to <name> --stdin` (full document, auto-chunked)
- Unified document model: physical (file_path) or virtual (no file_path)
- `source_kind` reserved tag: `document` vs `note`
- Freshness queries: `--stale-days`, `--recently-changed`

---

### v0.4.3: Docs Verification Polish 📋
**Deliverables**: Smart comparison for dynamic output, metadata headers on behavioral docs.

### v0.4.4: Advanced Providers 📋
**Deliverables**: ONNX provider with CUDA, HuggingFace API provider.

### v0.4.5: Setup + Benchmark UX 📋
**Deliverables**: `cite setup` wizard, provider recommendation, provider benchmarks.

### v0.4.6: Semantic Chunking 📋
**Deliverables**: Heading-boundary-aware chunker, sentence boundaries, variable chunk sizes, code block preservation.

### v0.4.7: Re-ranking 📋
**Deliverables**: Cross-encoder re-ranker, two-stage retrieval, benchmarks.

### v0.4.8-10: Hybrid Search 📋
**Deliverables**: FTS5 index, vector + BM25 scoring, configurable weights, benchmarks.

### v0.5.0: Cite Agent Interface 📋
**Status**: RFC approved, deferred until v0.4.0-v0.4.2 stable

**What**: Cite skill, stable CLI JSON contract, v1 direction

**Source RFC**: `rfc-cite-v1-skill-lsp.md`

**Deliverables**:
- `.pi/skills/cite/SKILL.md` + `docs/agent-skill.md`
- Documented field contracts + shape fixtures for v1-candidate commands
- Stability labels: `stable-v1-candidate`, `experimental`, `internal`
- CLI `--json` agent-optimized output
- 10 workflow fixtures + failure case suite
- `openspec/architecture/cite-v1-agent-interface.md`

---

## Dependency graph

```
Phase 1 (Scaffold)
    └─→ Phase 2 (Ingest)
            └─→ Phase 3 (Retrieval)
                    └─→ Phase 4 (Context Packs)
                            └─→ Phase 5 (Durability)
                                    └─→ Phase 6 (Evaluation)
                                            └─→ Phase 7 (Packaging)

Post-MVP (v0.4.x line):
    v0.4.0 (tags + ollama + lifecycle) ← Phase 2
    v0.4.1 (reembed + doctor) ← v0.4.0
    v0.4.2 (note add) ← v0.4.0 tags
    v0.4.3 (docs polish) ← independent
    v0.4.4 (advanced providers) ← v0.4.0 factory
    v0.4.5 (setup + benchmarks) ← v0.4.0 factory
    v0.4.6 (semantic chunking) ← independent, needs reembed
    v0.4.7 (re-ranking) ← independent
    v0.4.8-10 (hybrid search) ← independent

Critical path to v0.5:
    v0.4.0 → v0.4.1 → v0.4.2 → v0.5.0 (agent interface)

v0.5.0 (Cite Agent Interface):
    v0.5 Phase A (skill) ← v0.4.2
    v0.5 Phase B (schema) ← Phase A
    v0.5 Phase C (v1 arch) ← Phase A + B
    v0.5 Phase D (bridge) ← Phase A + B
    v0.5 Phase E (validation) ← Phase D
```

Detailed scope artifacts:

- `openspec/rfc/active/release-scope-v0.4-line.md`
- `openspec/rfc/active/implementation-plan-v0.4-v0.5.md`
- `openspec/rfc/active/rfc-cite-v1-skill-lsp.md`

## Estimated scope

| Phase | Estimated lines | Key complexity |
|---|---|---|
| 1 | ~1200 | ✅ Done |
| 2 | ~1500 | PDF extraction, chunking strategy, provider abstraction |
| 3 | ~800 | Vector index, cosine similarity, metadata joins |
| 4 | ~1000 | Result-kind logic, citation assembly, trace records |
| 5 | ~800 | Durable locks, rate limiting, atomic snapshots |
| 6 | ~500 | Sample corpus authoring, fixture validation |
| 7 | ~300 | Packaging, docs, demo flow |
| **Total** | ~6100 | |
