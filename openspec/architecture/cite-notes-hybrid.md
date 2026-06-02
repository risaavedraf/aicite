# Hybrid Notes Ingestion (CLI + Front‑Matter)

Define a hybrid input format for `cite note add` so notes can be created from a human‑friendly document **and** be fully controllable by agent skills via CLI flags. Notes are persisted in Cite and participate in normal retrieval alongside documents, but remain distinguishable by `source_kind`.

## Quick path

1. Create a note in Markdown with front‑matter (or pass metadata via CLI flags).
2. Run `cite note add --file <note.md>` (or `--stdin`).
3. Retrieve normally with `cite search/retrieve/context` — notes are mixed with documents by default.

## CLI surface (proposed)

```bash
# Minimal, front‑matter only
cite note add --file note.md

# Hybrid: CLI metadata overrides front‑matter
cite note add \
  --title "JWT rotation" \
  --topic "Authentication" \
  --concept "Token rotation" \
  --meta "name_project:aicite" \
  --meta "tag:auth" \
  --meta "tag:security" \
  --stdin
```

### Flags

| Flag | Purpose | Notes |
|---|---|---|
| `--title` | Display name | Overrides front‑matter `title` |
| `--topic` | Topic label | Overrides front‑matter `topic` |
| `--concept` | Concept label | Overrides front‑matter `concept` |
| `--meta key:data` | Repeated metadata | Appends (no implicit overwrite) |
| `--file` / `--stdin` | Input body | Mutually exclusive |

### Precedence rules

1. **CLI flags override front‑matter** for title/topic/concept.
2. `--meta` entries **append** to front‑matter metadata.
3. If no topic/concept is provided, infer from Markdown headings when possible.

## Front‑matter format (human‑friendly)

```md
---
title: "JWT rotation"
meta:
  - key: name_project
    data: aicite
  - key: tag
    data: auth
  - key: tag
    data: security
topic: "Authentication"
concept: "Token rotation"
---
JWT access tokens rotate every 15 minutes.
Refresh tokens last 7 days.
```

### Metadata model

- Metadata is a **list of key/data pairs**.
- Keys are free‑form, but recommended keys:
  - `tag`, `name_project`, `agent`, `source`, `decision`, `behavior`
- Repeated keys are allowed (ex: multiple `tag`).

## Storage model (proposed)

- `documents.source_kind` new enum: `document | note | <future>`
- `document_meta(document_id, key, value)` table
  - Optional index on `(key, value)` for tags and filters

Notes are ingested into the existing hierarchy:

```
Document (note) → Topic → Concept → Chunk
```

## Retrieval behavior

- Notes **mix with documents by default**.
- Results include `source_kind` so clients can distinguish.
- Optional future flag: `--source notes|documents|all` (default `all`).

## Chunking

- Notes use the same sentence‑based chunking (30–200 chars) when hierarchy is enabled.
- If heading inference fails, a default topic/concept is generated from title.

## Edge cases

- Missing title: derive from first heading or first 40 chars of body.
- Conflicting metadata: CLI wins for title/topic/concept, metadata list appends.
- Invalid front‑matter: treat entire file as body; CLI flags still apply.

## Front‑lobe engine (separate layer)

A “front‑lobe” orchestrator can be implemented as a **Pi skill** that:

- chooses what to store and how
- formats `cite note add` inputs
- standardizes metadata keys

This keeps Cite as the **evidence engine** and moves reasoning into a higher‑level layer.

## Next step

- Confirm whether `--meta` should allow overwrite semantics (ex: `--meta-override tag:auth`).
- Decide if `document_meta` should live in storage (persistent) or only be embedded in JSON output.

## Related docs

- [RFC: Hybrid Notes Ingestion](../rfc/active/rfc-notes-hybrid.md)
