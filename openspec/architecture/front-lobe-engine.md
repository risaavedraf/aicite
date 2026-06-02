# Front‑Lobe Engine (Cite‑backed Orchestrator)

A separate orchestration layer that treats **Cite as the evidence store**. The front‑lobe engine decides *what* to persist, *how* to label it, and *when* to retrieve it, but it does **not** replace Cite’s retrieval or citation model.

## Quick path

1. Front‑lobe stores notes via `cite note add` (hybrid format).
2. Front‑lobe retrieves evidence via `cite search/retrieve/context`.
3. All reasoning uses Cite’s context packs and citations as the ground truth.

## Scope

### In scope

- Orchestrate note creation and updates (decisions, behaviors, plans).
- Normalize metadata keys and tagging conventions.
- Decide retrieval strategy (top‑k, filters, note vs document weighting).
- Store agent state as **notes** in Cite, not in memory‑only stores.

### Out of scope

- Replacing Cite’s retrieval engine.
- LLM answer generation inside Cite.
- Building a daemon or long‑running service.
- Managing low‑level storage/migrations.

## Core principle

> **If it isn’t in Cite, it doesn’t exist.**

The front‑lobe treats Cite as the canonical evidence source. Notes, decisions, behaviors, and internal summaries are persisted as **notes** and retrieved with citations like any other document.

## Interfaces

### Write path

- `cite note add` (primary)
- Format: hybrid (front‑matter + CLI overrides)
- Metadata: key/data pairs (tags, agent, source, decision, behavior)

### Read path

- `cite search` for quick ranking
- `cite retrieve` for structured chunks
- `cite context` for full context packs

### Optional filters (future)

- `--source notes|documents|all`
- `--meta key=value` (e.g., `tag=auth`)

## Data model assumptions

- Notes are stored as `documents.source_kind = note`.
- Notes participate in the same hierarchy:
  `Document (note) → Topic → Concept → Chunk`.
- Metadata is stored in `document_meta` and exposed in retrieval outputs.

## Design constraints

- Keep Cite CLI the single source of truth for persistence.
- Keep the front‑lobe engine stateless between runs unless it writes notes.
- Avoid introducing a new storage system.

## Open questions

1. Should the front‑lobe engine be implemented as a **Pi skill** or as a standalone CLI tool?
2. Should note updates be append‑only or allow in‑place edits?
3. Should the front‑lobe engine maintain a “summary note” per agent/session?

## Related docs

- [Hybrid Notes Ingestion](cite-notes-hybrid.md)
- [RFC: Hybrid Notes Ingestion](../rfc/active/rfc-notes-hybrid.md)
- [RFC: Front‑Lobe Engine](../rfc/active/rfc-front-lobe-engine.md)
