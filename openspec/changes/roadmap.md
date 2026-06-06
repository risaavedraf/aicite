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

### Phase 8: Tag System (v0.3.2) 📋
**Status**: RFC approved, pending implementation

**What**: Flexible key:value tag system for all entities

**RFC**: `openspec/rfc/active/rfc-tags-and-note-add.md`

**Deliverables**:
- `tags` table + migration
- CLI: `tag set`, `tag get`, `tag rm`
- Integration: `--tag` filter on `search`, `retrieve`, `context`, `list`
- Auto-tag on `ingest` from path patterns
- `check-docs` reads `<!-- tag:status=planned -->` from markdown

---

### Phase 9: Pluggable Embedding Providers (v0.3.2) 📋
**Status**: RFC approved, pending implementation

**What**: Configurable embedding provider system — local and cloud

**RFC**: `openspec/rfc/active/rfc-embedding-providers.md`

**Deliverables**:
- Ollama provider (HTTP to local Ollama, GPU automatic)
- `embed_batch` in provider trait (3-10x faster ingestion)
- Config extension: `device`, `dimensions`, `batch_size`, `endpoint`
- `cite doctor` — full pipeline diagnostics
- `cite ingest --reembed` — atomic migration between providers
- `cite ingest --resume` / `--retry-failed` — resumable ingestion
- Actionable error messages for all provider errors

---

### Phase 10: Note Add + Agent Knowledge Capture (v0.3.3) 📋
**Status**: RFC approved, pending implementation

**What**: Agent-facing command to write knowledge directly into Cite

**RFC**: `openspec/rfc/active/rfc-tags-and-note-add.md`

**Deliverables**:
- `source_type` column on chunks (file vs agent_note)
- `cite note add` with hierarchy + tags
- Virtual documents per workspace
- Auto-create topics/concepts on first note

---

### Phase 11: Local Embedder + ONNX (v0.4.1+) 📋
**Status**: RFC approved, deferred from v0.4 core

**What**: In-process local embedding with ONNX Runtime and advanced provider setup

**RFC**: `openspec/rfc/active/rfc-embedding-providers.md`

**Deliverables**:
- ONNX provider with CUDA support
- HuggingFace API provider
- `cite setup` interactive wizard
- GPU/hardware auto-detection
- Provider recommendation based on hardware

---

## Release slicing recommendation

Do **not** bundle all active RFC content into v0.5.

Recommended framing:

- Keep the roadmap inside the **v0.4 line** as incremental releases: `v0.4.0`, `v0.4.1`, `v0.4.2`, ... up to `v0.4.10` if needed.
- Reserve **v0.5.0** for the Cite agent-interface milestone: skill/LSP-like bridge, integration contract, and v1 direction.

Suggested high-level slices:

- `v0.4.0`: tags + Ollama/local provider MVP.
- `v0.4.1`: re-embed/resume/retry-failed, `cite doctor`, actionable errors.
- `v0.4.2`: `note add` and workspace notes.
- `v0.4.6+`: semantic chunking / re-ranking.
- `v0.4.8-v0.4.10`: hybrid search with FTS5 + vector scoring and benchmarks.
- `v0.5.0`: Cite skill/LSP-like agent interface and v1 direction contract.

Detailed scope artifacts:

- `openspec/rfc/active/release-scope-v0.4-line.md`
- `openspec/rfc/active/rfc-cite-v1-skill-lsp.md`

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

Post-MVP (can be parallelized):
    Phase 8 (Tags) ← Phase 2
    Phase 9 (Providers) ← Phase 2
    Phase 10 (Note Add) ← Phase 8
    Phase 11 (ONNX) ← Phase 9

Release dependencies:
    v0.4.0 foundation ← Phase 8 + Phase 9
    v0.4.1 migration/diagnostics ← stable provider config + storage migration path
    v0.4.2 notes ← Phase 8 tags
    v0.4.6+ retrieval quality ← stable v0.4 reembed path + expanded benchmarks
    v0.4.8-v0.4.10 hybrid search ← FTS5 + vector scoring + quality evidence
    v0.5.0 agent interface ← stable v0.4.x features + Cite skill + LSP/MCP/JSON contract + v1 direction
```

Phases 1-7 are sequential (MVP). Phases 8-11 are post-MVP and can be partially parallelized, but release scope should stay reviewable. v0.5 should be saved for the agent-facing interface that points toward Cite v1, not for bundling every active RFC.

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
