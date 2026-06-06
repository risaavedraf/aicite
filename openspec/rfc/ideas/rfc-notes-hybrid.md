# RFC: Hybrid Notes Ingestion (cite note add)

## Status: Draft

This RFC proposes a **hybrid input format** for notes so they can be created from a human‑friendly Markdown document **and** be fully controllable by agent skills via CLI flags. Notes are persisted in Cite and participate in normal retrieval alongside documents, while remaining distinguishable by `source_kind`.

## Quick path

1. Confirm hybrid input precedence (CLI overrides front‑matter).
2. Decide `source_kind` values and metadata key conventions.
3. Decide exposure in outputs (`document_meta` in list/get?).
4. Decide whether notes need a dedicated list command.

## Problem

Cite currently ingests files and builds a document‑centric hierarchy. Agents, however, need to persist **notes, decisions, behaviors, and scratch knowledge** as evidence, then retrieve them with citations. Without first‑class notes:

- The agent has no canonical place to store memory/evidence.
- Retrieval cannot cite internal reasoning or decisions.
- External memory systems become the source of truth, fragmenting evidence.

## Goals

1. Persist notes as **evidence** in the same store as documents.
2. Mix notes and documents in retrieval by default, but keep them distinguishable.
3. Provide a **human‑friendly** format for manual notes.
4. Provide **agent‑friendly** CLI flags for structured ingestion.
5. Use tags for flexible grouping: documents are containers, chunks carry tags. No hierarchy.

## Non‑goals

- Replace retrieval pipelines with a new memory system.
- Build a daemon or interactive editor.
- Implement a “front‑lobe” reasoning engine (separate layer).
- Add LLM answer generation inside Cite.

## Proposed approach

### New command

```
cite note add [--file <path> | --stdin]
```

### Hybrid input rules

- Notes can define metadata **inside front‑matter**.
- CLI flags can **override** front‑matter for title/topic/concept.
- CLI `--meta` entries **append** to front‑matter metadata.

### CLI flags

| Flag | Purpose | Notes |
|---|---|---|
| `--title` | Display name | Overrides front‑matter `title` |
| `--tag key:value` | Tag assignment | Repeated; agent assigns freely (e.g. `--tag topic:Auth`) |
| `--workspace` | Workspace scope | Sets `workspace:<name>` reserved tag |
| `--to` | Target document | Find or auto-create virtual document |
| `--file` / `--stdin` | Input body | Mutually exclusive |

#### Example (CLI‑driven)

```bash
cite note add \
  --title "JWT rotation" \
  --tag topic:Authentication \
  --tag concept:Token-rotation \
  --workspace aicite \
  --tag tag:auth \
  --tag tag:security \
  --stdin
```

### Front‑matter format (human‑friendly)

```md
---
title: "JWT rotation"
meta:
  - key: workspace
    data: aicite
  - key: tag
    data: auth
  - key: tag
    data: security
  - key: topic
    data: Authentication
  - key: concept
    data: Token-rotation
---
JWT access tokens rotate every 15 minutes.
Refresh tokens last 7 days.
```

Note: `topic` and `concept` are tags in front‑matter, not separate fields. All metadata lives in the `meta` key/data pairs.

### Metadata model

- `meta` is a **list of key/data pairs**.
- Repeated keys are allowed (e.g., multiple `tag`).
- Recommended keys: `tag`, `name_project`, `agent`, `source`, `decision`, `behavior`.

## Storage changes

### source_kind

`source_kind` is a reserved tag (not a column) on documents:
- `source_kind:document` — ingested from physical file
- `source_kind:note` — created via `note add`

All metadata (workspace, type, session, source_kind) lives in the tags table.

### New table

```
document_meta(
  document_id TEXT NOT NULL,
  key TEXT NOT NULL,
  value TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  PRIMARY KEY (document_id, key, value)
)
```

**Optional indexes** (for future filters):
- `idx_document_meta_key_value (key, value)`

## Retrieval behavior

- Notes mix with documents by default.
- Outputs include `source_kind` for each result/citation.
- Future filter: `--source notes|documents|all` (default `all`).

## Chunking

- Notes are chunked flat with tags. No hierarchy.
- Single notes are typically one chunk (atomic).
- Longer notes (via `doc write`) are chunked by size, tags carried per chunk.

## Migration + compatibility

- Existing docs remain valid (`source_kind = document`).
- No changes to current `ingest` behavior.
- Notes create a new document record but do not require a file path.

## Open questions

1. Should `--meta` support overwrite semantics (e.g., `--meta-override tag:auth`)?
2. Should `document_meta` be exposed in JSON output for `list/get`?
3. Do we need `cite note list` or should notes appear in `cite list` only?

## Review plan

- [ ] Confirm hybrid input precedence (CLI overrides front‑matter).
- [ ] Confirm `source_kind` and metadata key conventions.
- [ ] Decide `document_meta` exposure in `list/get` outputs.
- [ ] Decide whether to add `cite note list`.
- [ ] Validate retrieval outputs include `source_kind`.

## Related docs

- [Hybrid Notes Ingestion (Architecture)](../../architecture/cite-notes-hybrid.md)
- [API Contract](../../prd/09-api-contract.md)
- [System Architecture](../../prd/07-system-architecture.md)

---

## Review Notes

Updated to align with D9: tags replace Topic/Concept hierarchy. `topic_id`/`concept_id` removed; use tags for any grouping. `source_kind` is a reserved tag, not a column. Front‑matter `topic`/`concept` fields moved into `meta` key/data pairs as tags. Chunking section simplified: no hierarchy, tags only.
