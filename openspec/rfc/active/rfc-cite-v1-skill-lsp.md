# RFC: Cite Skill/LSP Bridge Toward v1

**Status:** Draft
**Author:** el Gentleman + rikar
**Created:** 2026-06-06
**Target:** v0.5 direction-setting release
**Related:** release-scope-v0.4-line.md, rfc-cite-pi-integration.md, CITE_Pi_Integration.md, rfc-front-lobe-engine.md, rfc-notes-hybrid.md, rfc-rag-benchmark-framework.md, RAG_Benchmark_Guide.md, rfc-tags-and-note-add.md, rfc-embedding-providers.md

---

## Decision

v0.5 should not be just another retrieval-internals release.

v0.5 should establish the **agent-facing contract** for Cite on the path to v1:

> This is how agents use Cite, this is the workflow Cite expects, and this is the interface that will become stable for v1.

The v0.4.x line can finish tags, providers, notes, diagnostics, chunking, re-ranking, and hybrid search. v0.5 should then package those capabilities into a coherent agent integration layer.

---

## Problem

Cite currently has strong CLI primitives, but agents still need external instruction to decide:

- when to call `search` vs `retrieve` vs `context`;
- how to filter by workspace/tags/status;
- how to cite evidence;
- when to add notes;
- when to re-ingest or run diagnostics;
- how to avoid treating planned docs as implemented behavior;
- how to move from retrieval to durable project memory without inventing answers.

Without an explicit agent-facing contract, Cite risks becoming a bag of commands instead of a reliable tool protocol.

---

## v0.5 Product Thesis

v0.5 is the **Cite Agent Interface** release.

It should answer:

1. **How should an AI agent use Cite today?**
2. **What tool contract should integrations rely on?**
3. **What parts are experimental vs v1-candidate stable?**
4. **How does Cite become a first-class local context substrate by v1?**

---

## Plan Review

The direction is sound, but the plan needs one hard decision before implementation:

> v0.5 should be **contract-first**, not transport-first.

That means the first deliverable is not “build an LSP server.” The first deliverable is the agent workflow contract and stable JSON/request-response shapes. Once that is clear, the bridge can be exposed through Pi tools, MCP, JSON-RPC, or a future real LSP without rewriting the product semantics.

### Recommended sequencing

| Step | Output | Why it comes first |
|---|---|---|
| 1 | Cite usage skill | Teaches agents the correct behavior immediately |
| 2 | Stable command/schema contract | Prevents integrations from depending on accidental CLI shapes |
| 3 | v1 agent-interface architecture doc | Explains what becomes stable by v1 |
| 4 | Smallest bridge | Exposes the contract through Pi/MCP/JSON-RPC/CLI JSON |
| 5 | Workflow validation | Proves an agent can retrieve, cite, filter, note, and diagnose correctly |

### Key correction

Use “LSP-like” as a protocol design goal unless a real editor LSP becomes clearly necessary. Real LSP has editor-specific concepts that may be premature. Cite’s immediate need is an agent/tool protocol: context, read, search, notes, diagnostics, tags, and trace.

---

## Source Ideas Pulled Into Scope

The v0.5 plan should reuse these idea/RFC documents instead of starting from scratch:

| Source | Useful for v0.5 | Scope impact |
|---|---|---|
| `openspec/rfc/ideas/rfc-cite-pi-integration.md` | Pi skill/extension architecture, lazy model loading, native Pi tools | Confirms Pi skill/extension as a valid bridge, but keeps CLI standalone |
| `openspec/improvements/ideas/CITE_Pi_Integration.md` | Practical model/resource guidance and extension structure | Adds latency/resource expectations for any native bridge |
| `openspec/rfc/ideas/rfc-front-lobe-engine.md` | Evidence Protocol and write → retrieve → cite loop | Adds front-lobe behavior to the skill contract, not as a new engine in v0.5 |
| `openspec/rfc/ideas/rfc-notes-hybrid.md` | Hybrid note input, `source_kind`, metadata conventions | Makes note workflow concrete and schema-ready |
| `openspec/rfc/ideas/rfc-rag-benchmark-framework.md` | Precision/recall/hit-rate/latency methodology | Adds validation criteria for agent interface and retrieval behavior |
| `openspec/improvements/ideas/RAG_Benchmark_Guide.md` | Practical benchmark checklist and target metrics | Adds benchmark acceptance and workflow validation requirements |
| `openspec/rfc/completed/rfc-install-setup-ux.md` | Setup/health/config UX already implemented | Keeps v0.5 from re-solving setup; uses setup/health as diagnostic primitives |
| `openspec/rfc/ideas/rfc-landing-page.md` | Agent discoverability, `llms.txt`, clear public positioning | Adds optional v0.5/v1 communication artifact, not core implementation |

---

## Scope

### 1. Cite usage skill

Create a repository-facing skill that teaches agents the correct Cite workflow.

Candidate path:

```text
.pi/skills/cite/SKILL.md
```

The skill should define:

- retrieval workflow: `context` first for answerable context packs, `retrieve` for full chunks, `read` for exact citation expansion;
- evidence rules: never answer beyond retrieved evidence; cite source IDs; distinguish evidence from inference;
- tag/workspace rules: prefer `--tag workspace:<name>`, `--tag status:implemented`, and avoid planned content unless planning;
- note workflow: when to use `cite note add`, what tags/source metadata to include, how hybrid front-matter/CLI overrides work, and when not to persist noisy observations;
- doc-sync workflow: when to run `check-docs`, how planned docs are tagged, and how to interpret stale docs;
- diagnostics workflow: when to run `cite doctor`, `health`, `evaluate`, `trace`;
- failure handling: provider mismatch, stale locks, failed docs, low retrieval confidence;
- Evidence Protocol: write → retrieve → cite loop for decisions, fixes, patterns, and milestone summaries;
- v1 compatibility promise: which commands/outputs are stable enough for integrations.

### 2. Agent protocol / LSP-like interface

Define a stable integration layer for agents and editors. This can be a real LSP later, but v0.5 can start as an **LSP-like protocol contract**.

Candidate capabilities:

| Capability | Purpose |
|---|---|
| `cite/context` | Return context pack for query with citations and confidence metadata |
| `cite/read` | Expand citation/chunk by ID |
| `cite/search` | Lightweight candidate search |
| `cite/noteAdd` | Persist agent discovery or decision |
| `cite/diagnostics` | Surface provider/database/staleness issues |
| `cite/tags` | List/filter known tags and workspaces |
| `cite/trace` | Explain why retrieval returned a result |

The first implementation does not need a full editor LSP server. Acceptable v0.5 forms:

- an MCP server;
- a Pi extension/tool bridge;
- a JSON-RPC local process;
- a documented protocol over CLI JSON output.

The recommended v0.5 default is:

1. **CLI JSON contract first** — lowest implementation risk and keeps standalone Cite independent.
2. **Pi skill second** — immediate agent behavior improvement with no daemon requirement.
3. **MCP or Pi extension bridge third** — native tool surface once the contract is stable.
4. **Real LSP later** — only if editor integration needs justify LSP-specific semantics.

The important part is the contract: stable request/response shapes and workflow semantics.

### 3. Evidence Protocol / front-lobe behavior

The Front-Lobe RFC should influence v0.5, but v0.5 should not build a separate front-lobe engine yet.

v0.5 should define the behavior as part of the skill/protocol contract:

- when agents should persist evidence;
- minimum note fields: title, topic/concept, tags, source kind, body;
- recommended metadata keys: `tag`, `workspace` or `name_project`, `agent`, `source`, `decision`, `behavior`;
- append-only default unless an explicit update policy is approved;
- retrieval outputs must distinguish notes from documents using `source_kind` or equivalent;
- notes and documents mix by default, with source filters planned or supported.

### 4. Benchmark and validation framework

v0.5 should prove that the agent interface improves reliability, not just document it.

Minimum benchmark/validation inputs from the RAG benchmark docs:

- evaluation dataset with representative queries;
- Context Precision, Context Recall, Hit Rate @K, latency;
- failure cases for low-confidence retrieval and planned-only content;
- workflow-level faithfulness check: final agent answer must be grounded in retrieved citations;
- version-to-version comparison when retrieval behavior changes.

This does not require a full benchmark platform in v0.5. It does require a repeatable validation checklist or fixtures for the agent workflow.

### 5. v1 readiness document

Create a v1 direction artifact that states what Cite is becoming.

Candidate path:

```text
openspec/architecture/cite-v1-agent-interface.md
```

It should define:

- Cite's role: retrieval-only evidence substrate, not an answer generator;
- stable CLI/API surface candidates;
- what agents can rely on by v1;
- what remains experimental;
- migration path from v0.4.x features to the v1 contract;
- compatibility expectations for skills, Pi, MCP, and editor integrations;
- agent discoverability posture, including whether to add `llms.txt` or equivalent public guidance.

---

## Non-goals

- Do not replace the CLI.
- Do not build answer generation into Cite.
- Do not require a daemon for normal CLI use.
- Do not make every v0.4.x retrieval feature block v0.5.
- Do not promise full LSP/editor integration if the first stable contract is MCP/JSON-RPC/CLI JSON.

---

## Recommended Dependency on v0.4.x

v0.5 should start after these v0.4.x capabilities exist or are at least stable enough to document:

- tags and workspace filtering;
- local provider/reembed/diagnostics path;
- note add or explicit decision to defer notes from the v1 contract;
- `source_kind` or equivalent source classification for documents vs notes;
- metadata conventions for notes/evidence;
- retrieval quality baseline with current vector/hybrid behavior;
- JSON outputs stable enough for tool integration.

Hybrid search is useful before v0.5, but not the reason v0.5 exists. v0.5 exists to define the **agent interface**.

---

## Recommended v0.5 Plan

### Phase A — Skill and workflow contract

Deliverables:

- `.pi/skills/cite/SKILL.md`.
- Generic non-Pi version, e.g. `docs/agent-skill.md`, if Cite should support other agent harnesses.
- Evidence Protocol section based on the front-lobe RFC.
- Decision table for command choice:
  - `context` for answerable context packs;
  - `retrieve` for full chunks;
  - `read` for exact citation expansion;
  - `search` for lightweight discovery;
  - `trace` for retrieval debugging;
  - `note add` for durable discoveries/decisions;
  - `doctor`/`health`/`evaluate` for pipeline state.

### Phase B — Stable schema contract

Deliverables:

- JSON schema or documented shapes for:
  - context pack;
  - citation/chunk read;
  - search result;
  - note add response;
  - diagnostic result;
  - trace result;
  - tag/workspace listing;
  - evidence metadata/source kind.
- Stability labels:
  - `stable-v1-candidate`;
  - `experimental`;
  - `internal`.

### Phase C — v1 direction architecture

Deliverables:

- `openspec/architecture/cite-v1-agent-interface.md`.
- Definition of Cite v1 as retrieval-only evidence substrate.
- Compatibility promises for CLI, Pi skill, MCP/JSON-RPC bridge, and future LSP/editor integrations.

### Phase D — Smallest bridge

Deliverables:

- Choose one bridge for v0.5 MVP:
  - preferred: CLI JSON + Pi skill;
  - optional: MCP or Pi extension if time allows.
- Bridge exposes the same contract from Phase B, not new semantics.

### Phase E — Workflow validation and benchmark

Deliverables:

- End-to-end scripted/manual agent workflow:
  1. ask question;
  2. retrieve context;
  3. cite evidence;
  4. filter by tags/workspace/status;
  5. add a note when appropriate;
  6. diagnose provider/index/corpus issues.
- Failure cases: low confidence, planned-only docs, stale embeddings, provider mismatch.
- Minimum metrics from benchmark docs: Context Precision, Context Recall, Hit Rate @K, latency, and workflow faithfulness.

---

## Acceptance Criteria

v0.5 is successful when:

- an agent can load one Cite skill and know how to use the tool correctly;
- the stable/experimental command surface is documented;
- tool integrations have stable request/response shapes or documented CLI JSON contracts;
- Cite has a clear v1 direction document;
- planned vs implemented knowledge is distinguishable via tags/workflow;
- note/evidence behavior is governed by an explicit Evidence Protocol;
- local/private operation is diagnosed through `cite doctor`/health/evaluate workflows;
- the user can explain Cite v1 in one sentence:

> Cite v1 is a local, CLI-first evidence and context substrate for agents, with stable retrieval, citation, note, diagnostic, and integration contracts.

---

## Open Questions

1. Should the v0.5 bridge be **CLI-JSON + Pi skill first** as recommended, or should MCP be mandatory for v0.5?
2. Is “LSP” literal, or do we mean LSP-like protocol semantics for agents/editors?
3. Which commands become v1-stable: `context`, `retrieve`, `read`, `trace`, `note add`, `doctor`, `tag`?
4. Should `note add` be required for v0.5, or can the skill define it as optional until v1?
5. Should the skill live in `.pi/skills/cite/` only, or should Cite also ship a generic `docs/agent-skill.md` for non-Pi agents?
6. What is the minimum schema stability promise: exact JSON schema, documented field contract, or compatibility tests?
7. Which Evidence Protocol metadata keys are required vs recommended?
8. Should v0.5 include `llms.txt`/agent-discoverability docs, or leave that to the landing-page RFC?

---

## Suggested Work Units

1. Draft Cite skill contract.
2. Draft command decision table and failure-handling workflow.
3. Define stable JSON schemas for context/read/search/note/diagnostics/tags/trace.
4. Draft v1 agent-interface architecture document.
5. Choose v0.5 bridge transport, with CLI JSON + Pi skill as the low-risk default.
6. Implement the smallest bridge.
7. Validate with an agent workflow: ask, retrieve, cite, filter, note, diagnose.
8. Add compatibility tests or golden fixtures for v1-candidate JSON outputs.

## Risks and Mitigations

| Risk | Mitigation |
|---|---|
| Building a real LSP too early | Treat v0.5 as LSP-like contract first; defer editor-specific protocol until needed |
| Skill becomes prose-only and not enforceable | Pair the skill with schemas, fixtures, and workflow validation |
| Bridge duplicates CLI semantics | Define one contract and expose it through CLI/Pi/MCP instead of inventing per-transport behavior |
| v1 promise is too vague | Label surfaces as `stable-v1-candidate`, `experimental`, or `internal` |
| Note workflow pollutes memory | Skill must define when not to persist notes and which tags are required |

