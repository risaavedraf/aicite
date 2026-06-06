# SDD Kickoff: v0.4.0 — Tags + Lifecycle + Ollama

**Trigger:** `/sdd-new` or `/sdd-init`
**Change name:** `v0.4.0-tags-lifecycle-ollama`
**Scope:** First slice of the v0.4.x release train

---

## Context

We have a reviewed and approved implementation plan at:
`openspec/rfc/active/implementation-plan-v0.4-v0.5.md`

v0.4.0 is the foundation slice: tags system, lifecycle tracking, and Ollama local embedding provider. Everything else (v0.4.1+, v0.5) depends on this.

---

## What to build

### Tags system (WU-001 to WU-005D)

- `tags` table in SQLite: `tag_id`, `entity_id`, `entity_type`, `key`, `value`
- Tags on chunks, inherited automatically to documents
- `cite tag set/get/rm` CLI commands
- `--tag key:value` filter on `search`, `retrieve`, `context`, `list`
- Path-based auto-tags during ingest (e.g. `openspec/prd/*` → `type:prd`)
- Reserved tag keys enforced: `workspace`, `type`, `session`, `source_kind`
- `check-docs` reads `<!-- tag:status=planned -->` from markdown
- Lifecycle metadata: `source_hash`, `ingested_at`, `file_modified_at` stored on document metadata
- Change detection: re-ingest compares hash, sets `status:changed` tag if different

### Ollama provider (WU-006 to WU-010)

- `OllamaProvider` over local HTTP (`POST /api/embed`)
- `embed_batch` on provider trait with `BatchStrategy` enum (Native/RateLimited/Sequential)
- Provider factory: `gemini`, `openai-compatible`, `ollama`
- Config fields: `endpoint`, `dimensions`, `device`, `batch_size`, `workspace`
- `cite health` reports provider details, latency, batch strategy

---

## Key architectural decisions (from plan)

| Decision | Summary |
|----------|---------|
| D8 | Documents and notes are unified; `source_kind` distinguishes them |
| D9 | Tags replace Topic/Concept hierarchy entirely — no topic_id/concept_id |
| D10 | `--to <name>` auto-creates virtual documents |
| D12 | Workspace auto-detected: config → git root → CWD |
| D13 | `BatchStrategy` enum: Native / RateLimited / Sequential |
| D14 | Reserved keys: workspace, type, session, source_kind |
| D15 | Tags table with JOIN is correct; bottleneck is in-memory ranking |
| D16 | Lifecycle tracking: ingested_at, source_hash, status:changed detection |

---

## Tree metaphor (canonical model)

```
Database (trunk)
  └─ Workspace: aiharness (branch)
       └─ Document: "API Reference" (sub-branch)
            └─ Chunk 1 [tag:jwt, tag:problem]
```

- Trunk = database
- Branch = workspace
- Sub-branch = document (physical or virtual)
- Leaf = chunk (carries tags)

---

## Constraints

- **Language:** Rust (existing workspace)
- **DB:** SQLite (existing)
- **TDD:** Follow existing test patterns in the repo
- **No breaking changes:** Existing Gemini provider must keep working
- **Review workload:** ~1,130 LOC across 13 work units, split into 2 PRs:
  - PR 1: tags + lifecycle (WU-001–005D) ~400 LOC
  - PR 2: ollama provider (WU-006–010) ~540 LOC

---

## Non-goals for v0.4.0

- Note add / doc write (that's v0.4.2)
- Reembed / doctor (that's v0.4.1)
- Semantic chunking (v0.4.6)
- Hybrid search (v0.4.8+)
- v0.5 skill / bridge / validation

---

## Source RFCs

- `openspec/rfc/active/rfc-tags-and-note-add.md` — tags + note add design
- `openspec/rfc/active/rfc-embedding-providers.md` — Ollama provider design
- `openspec/rfc/active/implementation-plan-v0.4-v0.5.md` — full plan with all decisions

---

## First action

Read the implementation plan, then start SDD explore phase focusing on:
1. Current SQLite schema (what tables exist, what needs migration)
2. Current provider trait (what methods exist, what needs extending)
3. Current ingest pipeline (where to hook auto-tags and lifecycle metadata)
4. Current CLI structure (where to add `tag` subcommand)
