# AI Harness CLI — MVP Roadmap

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
- CLI crate with `harness health --json`
- Common crate with types, errors, exit codes
- CI pipeline (GitHub Actions)
- README + .env.example

**SDD artifacts**: `docs/sdd/phase-1-scaffold/`

---

### Phase 2: Ingest Pipeline 🔲
**Status**: Pending

**What**: Document ingestion — extract text, chunk, embed

**Deliverables**:
- File validation (type, size, path policy)
- Text extraction from PDF, TXT, MD
- Chunking with overlap (800-1200 tokens, 100-200 overlap)
- Embedding generation via configurable provider
- Document lifecycle management (pending → processing → ready → failed)
- `harness ingest <path>` command
- `harness list` and `harness get` commands
- Retry/backoff for failed ingestion
- Partial data cleanup on failure

**Key crates**: `ingest`, `providers`, `storage`, `engine`

**SDD artifacts**: `docs/sdd/phase-2-ingest/`

---

### Phase 3: Retrieval Pipeline 🔲
**Status**: Pending

**What**: Vector-first semantic search over indexed documents

**Deliverables**:
- Vector index storage and lookup
- Cosine similarity scoring
- Top-k retrieval with configurable k (1-10, default 5)
- `harness search` command
- `harness retrieve` command
- Source/section/chunk metadata attachment
- Partial-corpus handling (use ready docs only)

**Key crates**: `retrieval`, `graph`, `storage`, `engine`

**SDD artifacts**: `docs/sdd/phase-3-retrieval/`

---

### Phase 4: Context Packs + Citations 🔲
**Status**: Pending

**What**: Agent-consumable context packs with citations and traces

**Deliverables**:
- Context pack assembly (context_pack_id, result_kind, citations, metadata)
- Result-kind decision table (context, no_results, insufficient_context)
- Evidence floor and confidence threshold logic
- Citation model (citation_id, document_id, chunk_id, page, offset, text, score)
- `harness context` command
- `harness read` command (citation or chunk lookup)
- `harness trace` command
- Agent instruction template
- Verification disclaimer in output

**Key crates**: `engine`, `cli`, `retrieval`

**SDD artifacts**: `docs/sdd/phase-4-context/`

---

### Phase 5: Durability 🔲
**Status**: Pending

**What**: Durable locks, rate limits, and backlog management

**Deliverables**:
- Durable ingestion locks (sequential processing)
- Backlog/upsert on lock conflict (`operation_in_progress`)
- `harness ingest --next` / `--queued` for backlog processing
- Rate limiting for retrieval/context (20 req/min per key)
- Durable rate-limit counters (survive CLI restarts)
- `harness retry` command for failed documents
- `harness refresh` command with atomic snapshot swap
- Recovery of interrupted `processing` documents on startup

**Key crates**: `storage`, `engine`, `cli`

**SDD artifacts**: `docs/sdd/phase-5-durability/`

---

### Phase 6: Evaluation 🔲
**Status**: Pending

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

### Phase 7: Packaging + Docs 🔲
**Status**: Pending

**What**: Release packaging, documentation, and demo preparation

**Deliverables**:
- Reproducible CLI binary builds
- Packaged demo with sample documents
- Complete README with all commands, config, storage paths
- `.env.example` with all provider configs
- Chile privacy law compliance notes
- Provider disclosure in CLI output
- Demo acceptance flow (5-minute review)
- CI/CD pipeline for releases

**Key crates**: all

**SDD artifacts**: `docs/sdd/phase-7-packaging/`

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
```

Each phase depends on the previous one. Phases cannot be parallelized.

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
