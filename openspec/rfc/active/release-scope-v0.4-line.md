# Release Scope: v0.4 Line

**Status:** Draft
**Author:** el Gentleman + rikar
**Created:** 2026-06-06
**Related:** rfc-tags-and-note-add.md, rfc-embedding-providers.md, rfc-auto-docs-sync.md, rfc-cite-v1-skill-lsp.md, EVALUACION_CITE.md

---

## Decision

Do **not** treat the active RFCs as requiring v0.5.

Recommended release framing:

- Keep the current roadmap inside the **v0.4 line** as incremental slices: `v0.4.0`, `v0.4.1`, `v0.4.2`, etc.
- Reserve **v0.5.0** for the Cite agent-interface milestone: skill/LSP-like bridge, integration contract, and v1 direction.

Why: the active RFCs are important, but most of them are foundation, pipeline, metadata, diagnostics, and quality-improvement work. They can ship as v0.4.x increments without forcing a v0.5 label. The release risk is not the number; it is bundling too many capabilities into one oversized drop.

---

## v0.4 Line Recommended Scope

The v0.4 line should make Cite operationally stronger and progressively improve retrieval quality without needing to jump to v0.5.

### Must have

#### 1. Tags

Source RFC: `rfc-tags-and-note-add.md`

Deliverables:

- `tags` table and migration.
- `cite tag set/get/rm`.
- `--tag` filters on `search`, `retrieve`, `context`, and `list`.
- Path-based auto-tags during ingest.
- `check-docs` support for markdown tags such as `<!-- tag:status=planned -->`.

Reason: tags solve current metadata gaps and unblock better filtering, doc-sync classification, and workspace-aware retrieval.

#### 2. Ollama/local provider MVP

Source RFC: `rfc-embedding-providers.md`

Deliverables:

- `OllamaProvider` over local HTTP.
- `embed_batch` on the provider trait with sequential fallback.
- Provider factory support for `gemini`, `openai-compatible`, and `ollama`.
- Config fields for endpoint, dimensions, device, and batch size where relevant.
- `cite health` provider details and latency.

Reason: Gemini rate limits are already blocking ingestion. Local providers are the fastest path to stable evaluation and full corpus ingestion.

#### 3. Migration and diagnostics UX

Source RFC: `rfc-embedding-providers.md`

Deliverables:

- `cite ingest --reembed`.
- `cite ingest --retry-failed` or equivalent failed-document retry.
- `cite ingest --resume` / stale-lock recovery path.
- `cite doctor` for config, provider, database, embeddings, and retrieval status.
- Actionable provider and stale-lock errors.

Reason: switching providers without migration and diagnostics would create a support burden and fragile user experience.

### Should have

#### 4. Note add

Source RFC: `rfc-tags-and-note-add.md`

Deliverables:

- `source_type` for chunks or equivalent note source classification.
- `cite note add`.
- Virtual note document per workspace.
- Tags and optional hierarchy on notes.

Reason: agent knowledge capture is valuable, but it depends naturally on tags and workspace semantics. It can be v0.4.1 or later if the must-have items need their own release first.

### Could have

#### 5. Smart docs comparison

Source RFC: `rfc-auto-docs-sync.md`

Deliverables:

- Regex or semantic comparison for dynamic output.
- Metadata headers on behavioral docs.

Reason: Phase 1 of `check-docs` is implemented. Smart comparison is useful but not central to the v0.4 foundation unless docs drift becomes the primary release risk.

### Later v0.4.x

These should remain in the v0.4 line unless they introduce a breaking compatibility or product-positioning change:

#### Semantic chunking

Deliverables:

- Heading-boundary-aware chunking.
- Sentence-boundary handling.
- Variable chunk sizes.
- Preserve code blocks as coherent chunks.

Recommended slice: `v0.4.6` or similar.

#### Re-ranking

Deliverables:

- Two-stage retrieval: vector/hybrid candidate set then re-ranker.
- Benchmark against current golden data and expanded retrieval-quality fixtures.

Recommended slice: `v0.4.7` or similar.

#### Hybrid search

Source RFC: `rfc-tags-and-note-add.md`

Deliverables:

- FTS5 index over chunk text.
- Vector + BM25 scoring.
- Configurable or at least documented scoring weights.
- Benchmark comparing pure vector vs hybrid retrieval.
- Tests for exact technical-token queries where vector-only retrieval is weak.

Recommended slice: `v0.4.8` to `v0.4.10`, depending on how many preparatory releases are needed.

Reason: hybrid search is a meaningful retrieval improvement, but it does not automatically require v0.5 if the project wants the whole current roadmap to be the v0.4 stabilization line.

---

## Active RFC Disposition

| File | Disposition | Release placement |
|---|---|---|
| `rfc-auto-docs-sync.md` | Phase 1 implemented; move to completed or keep only future phases referenced | v0.3.1 implemented; smart comparison v0.4 optional; CI v0.4/v0.5 optional |
| `rfc-tags-and-note-add.md` | Split into metadata foundation and retrieval-quality roadmap | tags v0.4.0; note add v0.4.1+; semantic chunking/re-ranking v0.4.6+; hybrid v0.4.8+ |
| `rfc-embedding-providers.md` | Split into local-provider MVP and advanced providers | Ollama/reembed/doctor/resume v0.4.0-v0.4.2; ONNX/HuggingFace/setup wizard v0.4.3+ |
| `EVALUACION_CITE.md` | Treat as evidence/report, not release scope | informs v0.4.x acceptance |
| `SESSION_CONTEXT_2026-06-06.md` | Treat as session handoff, not release scope | archive after scope is extracted |

---

## Proposed v0.4.x Slices

| Version | Theme | Scope |
|---|---|---|
| `v0.4.0` | Metadata + local-provider foundation | Tags, Ollama provider MVP, provider factory/config, health details |
| `v0.4.1` | Migration + diagnostics | Reembed, retry-failed/resume/force, doctor, actionable errors |
| `v0.4.2` | Agent knowledge capture | Note add, virtual workspace notes, source type filters |
| `v0.4.3` | Docs verification polish | Smart `check-docs` comparison and metadata headers |
| `v0.4.4` | Advanced providers | ONNX/HuggingFace if still desired |
| `v0.4.5` | Setup and benchmark UX | Setup wizard, provider recommendations, provider benchmarks |
| `v0.4.6` | Semantic chunking | Heading/sentence-aware chunker and reembed validation |
| `v0.4.7` | Re-ranking | Two-stage retrieval experiments and acceptance benchmarks |
| `v0.4.8`-`v0.4.10` | Hybrid search | FTS5, vector+BM25 scoring, benchmarked quality improvements |

This keeps the roadmap cohesive: **v0.4 is the “make Cite good enough for serious local agent memory/retrieval” line**.

## v0.5 Direction: Cite Agent Interface

Use `v0.5.0` as the milestone that points toward Cite v1.

v0.5 should answer:

> This is how agents use Cite, this is the tool contract integrations can rely on, and this is the path to v1.

Recommended v0.5 scope:

- Cite usage skill, likely `.pi/skills/cite/SKILL.md`.
- LSP-like or protocol contract for agents/editors: MCP, Pi extension, JSON-RPC, or stable CLI JSON.
- Stable/experimental command surface definition.
- v1 architecture direction document.
- End-to-end agent workflow validation: retrieve, cite, filter, note, diagnose.

Detailed v0.5 RFC: `openspec/rfc/active/rfc-cite-v1-skill-lsp.md`.

Hybrid search can still finish inside `v0.4.8-v0.4.10`. It does not need to own the v0.5 label unless it becomes a breaking/product-significant semantic change.

---

## Review Workload Guard

Do not implement v0.4 as one giant change. Suggested work units:

1. Tags schema + storage API.
2. Tags CLI + retrieval filters.
3. Check-docs tag integration.
4. Ollama provider + batch trait.
5. Config/provider factory/health details.
6. Reembed/resume/retry-failed migration UX.
7. Doctor/actionable errors.
8. Note add, only after tags are stable.

Each unit should carry focused tests and a fresh review before merging.
