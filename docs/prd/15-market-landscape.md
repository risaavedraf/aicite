# Market Landscape — CLI-First Private Semantic Document Engine

The market is fragmented: there are CLI/MCP local knowledge engines, app-first assistants, platform-first memory/context layers, and framework-first RAG stacks. Our MVP should be the **CLI-native, private, semantic document-context engine for AI agents**—not a chat app, not a note app, not a platform, and not a framework the user must assemble.

## Market clusters

| Cluster | Representative tools | Strengths we can borrow | Weaknesses to avoid | Relevance to us |
|---|---|---|---|---|
| CLI / MCP / local-first engines | Linkly AI CLI, docsearch-mcp, qi, CtxVault, SAME, lokb | Clean operator workflows, retrieval UX, local/offline defaults, provenance, vault semantics, diagnostics, MCP wrappers | Often stop at search or memory; some blur engine vs UI; some lack a strong document model | Closest cluster; confirms CLI is a real wedge |
| App / workspace-first assistants | Khoj, AnythingLLM, Onyx | Better onboarding, polished UX, connectors, familiar chat flows | Too UI-heavy, weaker engine identity, cloudy boundary between product and surface | Useful contrast, not our primary shape |
| Platform / API-first context layers | Graphlit, Mem0, Zep | Strong context assembly, persistence, agent integration, scalable APIs | Require assembly; not a local document engine; cloud gravity | Good integration ideas, but not our product form |
| Framework-first RAG stacks | LlamaIndex | Broad ingestion/retrieval building blocks, flexibility, ecosystem reach | Not a product; user must assemble the system; operational burden shifts to buyer | Useful backend patterns, not customer-facing positioning |

## What to borrow

- **Progressive retrieval UX**: search first, then refine, then read.
- **Retrieval pragmatism**: start vector-first with minimal source metadata; add graph expansion, lexical/hybrid ranking, and reranking only when the core loop is reliable.
- **Vault/scoped organization**: explicit collections, scopes, and isolation.
- **Provenance and freshness**: show where context came from and how current it is.
- **Diagnostics and repairability**: health, stats, refresh/reindex paths, and inspection paths.
- **Tool-friendly surfaces**: make the engine easy for agents to call through stable CLI JSON now and MCP later.

## What to avoid

- A chat app with documents attached.
- A generic note-taking or second-brain UI.
- An enterprise suite with connector sprawl and cloud gravity.
- A framework the user has to assemble into a product.
- A memory-only layer that treats documents as secondary.
- Premature protocol breadth before the CLI contract is stable.

## Differentiation

Our differentiator is not just retrieval quality; it is **operational integrity**:

- local-first and private by default,
- CLI-native and agent-accessible,
- context packs with citations,
- source-read and trace inspection,
- durable local state for normal CLI invocations,
- explicit scopes and isolation,
- provider/model-agnostic retrieval behavior.

That combination is the whitespace the market does not cover cleanly today.

## Recommended positioning

**A local-first, CLI-native semantic document-context engine for AI agents that turns private files into cited, traceable context packs with source inspection and durable local state.**

## Strategic principles

1. **Engine over UI** — the core value is the cite, not the surface.
2. **Evidence over answers** — the MVP returns cited context; downstream agents may generate answers.
3. **Vector-first before hybrid** — prove semantic retrieval with minimal source metadata before adding graph expansion, keyword/vector hybrid ranking, or reranking stacks.
4. **Local/private by default** — privacy is part of the product, not a checkbox.
5. **Agent-friendly contracts** — the CLI and outputs should be stable and machine-readable.
6. **Clarity over breadth** — do the document engine well before expanding into a general platform.

## Deferred but recoverable

| Deferred item | Why deferred | Recovery path |
|---|---|---|
| MCP access | No MVP contract exists yet; adding it now would blur the CLI-first scope | Wrap the stable CLI JSON schemas after `search`, `retrieve`, `context`, `read`, and `trace` are accepted |
| Full hybrid search | Current retrieval design is vector-first with minimal source metadata | Add graph expansion, keyword/FTS ranking, and reranking after golden retrieval tests show where vector-first fails |
| Built-in answer adapter | The pivot makes external agents the answer layer | Add an optional adapter over context packs without changing retrieval/source schemas |
| Native app | Product moved from app to CLI cite | Build as V2 companion after the CLI/engine is stable |

## Open questions

- Post-MVP: after vector-first retrieval is accepted, what graph structure beyond the MVP minimum (source -> section -> chunk hierarchy plus citation/source links) measurably improves retrieval or traceability?
- Which CLI commands are essential on day one versus later?
- What minimum provenance data is enough for trust without overcomplicating the model?
- When does a native UI become worth adding as a V2 surface?
- When is the CLI JSON contract stable enough to wrap with MCP?
