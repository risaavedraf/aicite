# RFC: Front‑Lobe Engine (Cite‑backed Orchestrator)

## Status: Draft

This RFC defines a **front‑lobe engine**: a separate orchestration layer that treats **Cite as the evidence store**. The front‑lobe decides *what* to persist, *how* to label it, and *when* to retrieve it, but it does **not** replace Cite’s retrieval or citation model.

## Quick path

1. Decide whether the front‑lobe is a **Pi skill**, a standalone CLI, or both.
2. Confirm the Evidence Protocol fields and metadata keys.
3. Agree on note update policy (append‑only vs editable).
4. Validate that retrieval outputs expose `source_kind` and metadata filters.

## Problem

Agents need a consistent way to persist decisions, behaviors, plans, and internal summaries as **evidence**. Today these are often stored in ad‑hoc memory tools or in transient prompts. That creates fragmentation:

- No single source of truth for agent knowledge.
- Retrieval can’t cite internal decisions.
- Evidence is split between documents and “memory”.

## Goals

1. Keep **Cite as the single evidence store** (“if it isn’t in Cite, it doesn’t exist”).
2. Orchestrate note creation using **`cite note add`** (hybrid format).
3. Normalize metadata keys and tagging conventions.
4. Provide a disciplined **write → retrieve → cite** loop for agents.

## Non‑goals

- Replacing Cite’s retrieval engine.
- Adding LLM answer generation inside Cite.
- Building a daemon or long‑running service.
- Introducing a new storage backend.

## Proposed approach

### Front‑lobe responsibilities

- Decide when to persist notes (decisions, behaviors, plans, summaries).
- Normalize metadata keys (`tag`, `agent`, `decision`, `behavior`, `source`).
- Choose retrieval parameters (top‑k, optional filters).
- Enforce a save protocol aligned with the **Evidence Protocol**.

### Cite responsibilities

- Persist notes as evidence via `cite note add`.
- Store evidence and return citations/context packs.
- Keep retrieval consistent and verifiable.

## Interfaces

### Write path

- `cite note add` (hybrid CLI + front‑matter)
- Metadata as key/data pairs
- `source_kind = note`

### Read path

- `cite search` for ranking
- `cite retrieve` for structured chunks
- `cite context` for context packs + citations

### Optional filters (future)

- `--source notes|documents|all`
- `--meta key=value` (e.g., `tag=auth`)

## Data model assumptions

- Notes are stored as documents with `source_kind = note`.
- Notes follow the same hierarchy:
  `Document (note) → Topic → Concept → Chunk`.
- Metadata is stored in `document_meta` and can be exposed in outputs.

## Alignment with Hybrid Notes RFC

This RFC **depends on** the hybrid notes ingestion design:

- Uses the same `cite note add` **hybrid input** (front‑matter + CLI overrides).
- Assumes `source_kind = note` and `document_meta` key/data pairs.
- Notes and documents mix in retrieval by default; `source_kind` distinguishes them.
- Metadata keys remain free‑form; the front‑lobe standardizes a **recommended set**.

## Evidence Protocol (initial)

**Save triggers** (front‑lobe decides):
- decisions
- bugs / fixes
- patterns / conventions
- summaries after milestones

**Minimum fields**:
- `title`
- `meta.tag` (repeatable)
- `topic` / `concept`
- body text (short, factual)

## Open questions

1. Should the front‑lobe be a **Pi skill**, standalone CLI, or both?
2. Should notes be append‑only or allow in‑place edits?
3. Should the front‑lobe maintain one “summary note” per session?

## Review plan

- [ ] Confirm hybrid input precedence (CLI overrides front‑matter).
- [ ] Confirm required metadata keys for the Evidence Protocol.
- [ ] Decide delivery vehicle (Pi skill vs standalone CLI).
- [ ] Decide note update policy (append‑only vs editable).
- [ ] Decide session summary strategy (per‑session summary note or not).
- [ ] Validate retrieval outputs include `source_kind` and optional filters.

## Related docs

- [Front‑Lobe Engine (Architecture)](../../architecture/front-lobe-engine.md)
- [RFC: Hybrid Notes Ingestion](./rfc-notes-hybrid.md)
- [Hybrid Notes Ingestion (Architecture)](../../architecture/cite-notes-hybrid.md)
