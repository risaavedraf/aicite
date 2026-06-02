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
