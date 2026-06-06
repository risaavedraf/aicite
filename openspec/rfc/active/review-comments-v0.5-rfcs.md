# Review Comments: v0.4.x to v0.5 RFC Scope

**Status:** Review notes
**Author:** el Gentleman + rikar
**Created:** 2026-06-06
**Reviewed scope:** active RFCs plus related ideas for the v0.5 Cite Agent Interface direction

---

## Executive take

The direction is now coherent:

- **v0.4.x** should absorb the technical foundation: tags, notes, local providers, diagnostics, chunking, re-ranking, hybrid search.
- **v0.5** should become the **Cite Agent Interface** milestone: skill, protocol contract, Evidence Protocol, validation, and v1 direction.

The main risk is not scope ambition. The main risk is **collapsing contract, transport, retrieval quality, and product positioning into one implementation step**.

Recommended rule:

> Every RFC should say whether it is foundation, interface contract, bridge transport, validation, or v1 positioning.

---

## Comment 1 — `release-scope-v0.4-line.md`

### What works

- Good separation between v0.4.x foundation and v0.5 interface milestone.
- Keeps hybrid search inside v0.4.x unless it becomes product-significant.
- Prevents v0.5 from becoming a giant feature bucket.

### Needs clarification

The v0.4.x sequence should be treated as a **release train**, not a hard promise that every patch number maps exactly to one feature.

### Suggested comment to apply

```md
Reviewer note: Treat the v0.4.x table as a release train, not a contractual version map. The important invariant is dependency order: metadata/providers/diagnostics before notes and retrieval-quality experiments; v0.5 only after the agent-facing contract can be credibly documented and validated.
```

---

## Comment 2 — `rfc-cite-v1-skill-lsp.md`

### What works

- Strong product thesis: v0.5 is the Cite Agent Interface release.
- Correctly chooses **contract-first, not transport-first**.
- Pulls in front-lobe behavior without prematurely building a separate engine.
- Uses benchmark docs to require validation rather than just prose.

### Needs clarification

The RFC still has one decision that should be made explicit soon:

- **Default v0.5 bridge:** CLI JSON + Pi skill.
- MCP/Pi extension: optional follow-up once the contract is stable.
- Real LSP: explicitly deferred unless editor semantics become necessary.

### Suggested comment to apply

```md
Reviewer note: Approve CLI JSON + Pi skill as the default v0.5 bridge unless a specific consumer requires MCP. This keeps v0.5 contract-first and avoids confusing “LSP-like” with “must implement LSP now.”
```

### Open decision

Do we want `docs/agent-skill.md` in addition to `.pi/skills/cite/SKILL.md`?

Recommendation: **yes**, because Cite should describe its agent contract outside Pi too.

---

## Comment 3 — `rfc-tags-and-note-add.md`

### What works

- Tags are the missing metadata layer.
- Note add is the path toward Cite as durable agent evidence.
- Retrieval quality roadmap is useful, but belongs mostly to v0.4.x.

### Needs clarification

This RFC currently mixes three different concerns:

1. metadata/tags;
2. agent notes/evidence;
3. retrieval quality roadmap.

That is okay as an umbrella RFC, but implementation should split them.

### Suggested comment to apply

```md
Reviewer note: Split implementation tasks by concern. Tags are v0.4.0 foundation; note add depends on tags/source metadata; retrieval-quality work should remain later v0.4.x. Do not make note add or hybrid search block the first tags release.
```

### Important dependency for v0.5

v0.5 needs at least one stable source classification concept:

- `source_kind = document | note`, or
- equivalent `source_type` output field.

Without that, the agent skill cannot reliably distinguish external docs from agent-written evidence.

---

## Comment 4 — `rfc-embedding-providers.md`

### What works

- Solves real blocker: Gemini 429 and high latency.
- Ollama-first is the right MVP.
- `doctor`, reembed, resume, retry-failed are not extras; they are required UX for provider changes.

### Needs clarification

Do not let ONNX/HuggingFace/setup wizard expand the first local-provider slice.

### Suggested comment to apply

```md
Reviewer note: Keep v0.4.0 provider work Ollama-first. ONNX, HuggingFace, setup wizard, and provider fallback chains are valuable, but they should not delay the minimal local-provider + reembed + doctor path needed by the v0.5 agent-interface contract.
```

### Important dependency for v0.5

The agent interface needs provider state in diagnostics:

- current provider;
- model ID;
- DB embedding model mismatch;
- failed/stale documents;
- recommended next command.

---

## Comment 5 — `rfc-auto-docs-sync.md`

### What works

- Phase 1 is implemented and should leave active scope.
- It supports v0.5 indirectly by making docs verifiable.

### Needs clarification

The RFC should not remain active as if its MVP is pending.

### Suggested comment to apply

```md
Reviewer note: Move this RFC to completed for Phase 1. Track smart comparison and CI integration as future optional slices. v0.5 should consume check-docs as a diagnostic/doc-trust primitive, not re-own this RFC as core scope.
```

---

## Comment 6 — `rfc-front-lobe-engine.md`

### What works

- The Evidence Protocol is exactly what v0.5 needs.
- “Cite as evidence store” is the correct v1 direction.

### Needs clarification

Do not build a separate front-lobe engine in v0.5.

### Suggested comment to apply

```md
Reviewer note: Promote the Evidence Protocol into the v0.5 skill/contract, but defer a separate front-lobe engine. For v0.5, the front-lobe is behavior: when to save, how to tag, how to retrieve, and how to cite.
```

---

## Comment 7 — `rfc-notes-hybrid.md`

### What works

- Hybrid front-matter + CLI flags is a good human/agent compromise.
- `source_kind` and metadata conventions are directly relevant to v0.5.

### Needs clarification

The v0.5 contract should define required vs recommended metadata keys.

### Suggested comment to apply

```md
Reviewer note: Decide minimum metadata for agent-written evidence before v0.5: title, source_kind, workspace/name_project, tag, topic/concept, and body. Other metadata can remain recommended.
```

---

## Comment 8 — `rfc-cite-pi-integration.md` and `CITE_Pi_Integration.md`

### What works

- Confirms Pi integration as a native-feeling bridge.
- Lazy model loading is the right performance posture.
- Keeps standalone CLI mode intact.

### Needs clarification

Pi extension should not become mandatory for v0.5.

### Suggested comment to apply

```md
Reviewer note: Use Pi skill as the first-class v0.5 artifact. Treat Pi extension/native tools as a bridge once the CLI JSON contract is stable. This preserves standalone Cite and avoids coupling v0.5 to Pi runtime internals.
```

---

## Comment 9 — `rfc-rag-benchmark-framework.md` and `RAG_Benchmark_Guide.md`

### What works

- Provides metrics that prevent hand-wavy quality claims.
- Context Precision, Recall, Hit Rate @K, latency, and faithfulness are enough for v0.5 validation.

### Needs clarification

v0.5 does not need a full benchmark platform, but it does need repeatable workflow validation.

### Suggested comment to apply

```md
Reviewer note: For v0.5, require a small validation fixture set for the agent interface: retrieve, cite, filter, note, diagnose, and low-confidence failure cases. Full benchmark automation can remain later.
```

---

## Comment 10 — `rfc-landing-page.md`

### What works

- Agent discoverability matters for v1 positioning.
- `llms.txt` fits the “Cite Agent Interface” direction.

### Needs clarification

Landing page should remain optional for v0.5 unless public positioning is part of the release goal.

### Suggested comment to apply

```md
Reviewer note: Consider `llms.txt` and README agent-positioning updates as optional v0.5 communication artifacts. Do not block the skill/protocol contract on the landing page.
```

---

## Consolidated decisions to make next

1. **Bridge default:** approve CLI JSON + Pi skill as v0.5 default?
2. **LSP meaning:** confirm “LSP-like” means protocol semantics, not real LSP server yet.
3. **Skill distribution:** `.pi/skills/cite/SKILL.md` only, or also `docs/agent-skill.md`?
4. **Evidence metadata:** which fields are required for v0.5?
5. **Stable schemas:** exact JSON Schema files or documented field contracts + golden fixtures?
6. **Completed RFC hygiene:** move `rfc-auto-docs-sync.md` out of active?
7. **Validation minimum:** how many workflow fixtures are enough for v0.5?

---

## Recommended next edit pass

1. Add reviewer notes directly to each RFC or convert them into `## Review Notes` sections.
2. Move implemented/obsolete active RFCs to completed/ideas.
3. Create `openspec/architecture/cite-v1-agent-interface.md` skeleton.
4. Create `.pi/skills/cite/SKILL.md` skeleton once the skill contract decisions are approved.
