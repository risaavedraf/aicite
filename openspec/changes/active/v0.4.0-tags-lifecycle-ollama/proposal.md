# Make Cite filterable, lifecycle-aware, and local-embedding ready

v0.4.0 should give Cite a metadata foundation agents can trust: key:value tags on documents and chunks, lifecycle metadata for re-ingest decisions, and an Ollama provider path for local embeddings. The full target is larger than one review-safe PR, so this proposal recommends shipping it as small slices while preserving the v0.4.0 product intent.

## Problem / current-state gap

Cite can retrieve indexed content, but agents cannot reliably answer product-critical questions like:

- Is this result implemented, planned, changed, deprecated, or just documentation?
- Which workspace, source kind, or document type does this evidence belong to?
- Has the source file changed since the last ingest?
- Can retrieval be narrowed before vector ranking without relying on the legacy topic/concept hierarchy?
- Can users run embeddings locally to avoid cloud latency, rate limits, and cost?

Today, lifecycle state is mixed with ingestion pipeline status, chunks lack flexible semantic metadata, re-ingest does not use source hashes to skip unchanged files, and local embedding providers are not first-class.

## Product intent and target situations

v0.4.0 is the foundation slice for agent-grade Cite workflows.

Target users and situations:

- **Agents retrieving evidence** need tags and freshness/status hints to avoid presenting planned or stale content as implemented behavior.
- **Developers ingesting project docs** need re-ingest to skip unchanged files and mark changed chunks for review.
- **Maintainers reviewing docs** need `check-docs` to understand planned command examples instead of flagging every future feature as outdated.
- **Local-first users** need Ollama support to reduce Gemini/OpenAI dependency, latency, and rate-limit failures.

## Scope recommendation

### v0.4.0 full target

The full v0.4.0 target remains:

1. Tags table and tag CLI.
2. Tag filters for `search`, `retrieve`, `context`, and `list`.
3. Path-based auto-tags and reserved key enforcement.
4. Lifecycle metadata: `source_hash`, `ingested_at`, `file_modified_at`.
5. Re-ingest behavior: skip unchanged files; process/update changed files.
6. `status:changed` on changed chunks only.
7. `check-docs` markdown tag parsing.
8. Provider trait batch support and `BatchStrategy`.
9. Ollama provider, config, factory support, and health details.

### Review-safe delivery slices

Because the session uses `ask_always` and a 400-line review budget, the implementation should not be bundled into one PR.

| Slice | Recommended content | Why |
|---|---|---|
| 1. Tag schema + lifecycle columns | Migration, tag storage helpers, reserved-key validation, lifecycle fields | Establishes durable data model with low behavior risk |
| 2. Tag CLI + local filtering | `cite tag set/get/rm`, basic list/filter plumbing | Makes tags usable before deeper retrieval changes |
| 3. Ingest lifecycle + inheritance | auto-tags, document/chunk tag storage, hash skip/update, `status:changed` chunks | Highest semantic risk; deserves focused review |
| 4. Retrieval and docs semantics | `--tag` filters on retrieval/list, `check-docs` status tag parsing, legacy topic/concept compatibility | Keeps user-facing query behavior reviewable |
| 5. Provider trait/factory | `embed_batch`, `BatchStrategy`, no-key local provider factory path | Decouples provider abstractions from tag work |
| 6. Ollama provider + health/config | Ollama HTTP provider, config fields, health latency/details | Independent local-provider feature slice |

Before each slice after the proposal/spec/design phases, ask for explicit approval per `ask_always`.

## In scope

- Store tags as key:value rows for `document` and `chunk` entities.
- Store inherited/descriptive tags on **both documents and chunks**.
- Add lifecycle metadata to documents: `source_hash`, `ingested_at`, `file_modified_at`.
- Re-ingest rules:
  - unchanged source hash: skip;
  - changed source hash: process/update and mark changed content.
- Treat `status` as a tag, but explicitly **non-inheritable**.
- Put `status:changed` only on chunks whose content changed, not automatically on the whole document.
- Keep existing `topic`/`concept` filters as legacy behavior during the transition.
- Support `--tag key:value` filters while preserving key-only matching if supported by the existing tag parser design.
- Add Ollama as a local embedding provider without breaking Gemini or OpenAI-compatible providers.

## Non-goals

- Note add / document write workflows.
- Virtual documents with nullable file paths, except where schema choices should avoid blocking them later.
- `doctor`, `reembed`, retry/resume, or freshness reporting beyond the v0.4.0 lifecycle foundation.
- Semantic chunking, hybrid search, reranking, ONNX, or HuggingFace provider support.
- Removing legacy topic/concept filters in v0.4.0.
- Aggregating document status from chunk statuses unless a future command explicitly asks for aggregation.

## Business and product rules

### Tag model

- Tags are flat `key:value` pairs, not nested hierarchies.
- Multiple values per key are allowed.
- Compound filters use AND semantics unless a later spec explicitly defines OR.
- Reserved engine-managed keys are: `workspace`, `type`, `session`, `source_kind`.
- User-driven tag commands must not freely overwrite reserved keys; engine-owned ingest/write paths may set them.

### Inheritance rules

- Descriptive/inherited tags are stored locally on both documents and chunks for simpler filtering and list behavior.
- Document-level descriptive tags may reflect document-wide classification such as `workspace`, `type`, and `source_kind`.
- Chunk-level descriptive tags should include the tags needed for retrieval filtering without requiring runtime inference.

### Critical `status` semantics

- `status` is a tag, but it is **non-inheritable**.
- A document must not automatically receive `status:changed` because one or more chunks changed.
- A filter by `status` must use local tags only:
  - `cite search ... --tag status:changed` matches chunks locally tagged as changed.
  - `cite list --tag status:changed` matches documents only if the document itself has a local `status:changed` tag, not because a child chunk has one.
- Future document-level status aggregation must be explicit in command/API design.

### Legacy compatibility

- Existing `topic` and `concept` filters remain available during the transition.
- Tags become the preferred metadata model, but this change must not silently break current users relying on legacy filters.

## Affected areas

- SQLite migrations and storage helpers.
- Common document/chunk types for lifecycle metadata.
- Ingest engine: hash calculation, skip/update behavior, path auto-tags, lifecycle writes.
- Retrieval/storage query path for tag filtering.
- CLI commands: `tag`, `list`, `search`, `retrieve`, `context`, `check-docs`, `health`.
- Provider trait, provider factory, config loading, and Ollama HTTP implementation.
- Tests and fixtures around migration, tag semantics, ingest behavior, and provider factory behavior.

## Risks and mitigations

| Risk | Mitigation |
|---|---|
| Tag inheritance creates confusing status behavior | Make `status` explicitly non-inheritable and test local-only status filters |
| v0.4.0 exceeds review budget | Use the review-safe slices above and ask before each apply slice |
| Re-ingest update semantics accidentally duplicate documents | Spec/design should define lookup by file path and unchanged-hash skip behavior before implementation |
| Reserved keys block engine auto-tags | Separate user-facing validation from engine-internal tag writes |
| Retrieval behavior changes break topic/concept users | Keep topic/concept filters as legacy behavior and add tags incrementally |
| Ollama no-key provider breaks current factory assumptions | Refactor factory so local providers bypass API-key validation |
| `status:changed` granularity requires diffing chunks | Start with deterministic chunk comparison; if exact changed-chunk detection is not possible in the first slice, fail safe by tagging only chunks known to be new/changed and documenting limitations |

## Rollback / degradation strategy

- Migration should be additive: tags table and lifecycle columns can remain unused if a slice is rolled back.
- If tag filters fail, retrieval can degrade to existing unfiltered vector search plus legacy topic/concept filters.
- If lifecycle update fails, ingestion can fall back to current full-process behavior while preserving existing provider behavior.
- If Ollama is unavailable, existing Gemini and OpenAI-compatible providers remain supported.
- If batch embedding fails, providers should fall back to sequential `embed` through the default trait implementation.

## Success criteria / acceptance criteria

- [ ] Tags can be set, read, and removed for documents and chunks.
- [ ] Descriptive tags can be stored on both documents and chunks.
- [ ] `status` tags are local-only and non-inheritable.
- [ ] `status:changed` is applied to changed chunks only, not automatically to the parent document.
- [ ] `--tag` filters work for retrieval/list use cases without removing legacy topic/concept filters.
- [ ] Re-ingest skips unchanged files by source hash.
- [ ] Re-ingest processes changed files and updates lifecycle metadata.
- [ ] Path-based auto-tags and reserved tag key enforcement work as specified.
- [ ] `check-docs` can read markdown status tags such as `<!-- tag:status=planned -->`.
- [ ] Ollama can embed through local HTTP without requiring an API key.
- [ ] Provider health can report provider identity, latency, and batch strategy.
- [ ] Existing Gemini/OpenAI-compatible behavior remains intact.

## Next phase recommendation

Proceed to the **spec** phase next. The spec should lock down:

1. Exact tag schema and indexes.
2. Local-only vs inherited tag query semantics, especially for `status`.
3. Re-ingest update/replace behavior and changed-chunk detection limits.
4. CLI argument shapes and output expectations.
5. Provider trait defaults and Ollama factory/config behavior.
