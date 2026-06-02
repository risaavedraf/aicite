# Risks and Open Questions

This document tracks product and implementation risks before coding starts.

## High-priority risks

| Risk | Impact | Mitigation |
|---|---|---|
| Retrieved context is weak or irrelevant | Agents produce poor downstream answers | Calibrated retrieval threshold, citations, golden retrieval tests, and `insufficient_context` output |
| Semantic graph overdesign | Scope balloons before value is proven | Start with minimal source/section/chunk/citation metadata that supports retrieval and traceability |
| Document extraction is poor | Bad chunks lead to bad context | Start with PDF/TXT/MD, add extraction tests, expose snippets |
| CLI contract drifts | Agents lose trust in the tool surface | Freeze command names, schema versions, and JSON output rules early |
| Embedding/provider costs grow | Demo becomes expensive | Use small models, configurable providers, local/mock embeddings option, and durable rate limits |
| MVP scope expands too much | Project does not ship | Keep auth, billing, native app, MCP, full hybrid search, and built-in answer generation out of MVP |
| Public demo receives sensitive docs | Privacy issue | Public packaged demo disables uploads and uses sample documents only; local/private runs can import docs with warnings |
| Personal data is sent to AI providers without review | Legal/compliance risk | Disclose provider usage, minimize context, and require production legal review |
| AI output creates accountability gap | Ethical/legal risk | Make the cite provide evidence; require human/operator accountability for downstream answers |
| Rust/CLI learning curve slows delivery | Schedule risk | Keep the engine/CLI boundary small and ship one narrow happy path first |
| Output format drift between human and JSON modes | Integration bugs | Keep both modes backed by the same schema and test them together |
| Frontend polish delays engine value | Slower delivery | Do not introduce a native app until the CLI/engine is stable |

## Product trade-offs

| Decision | Trade-off |
|---|---|
| CLI-first cite | Better agent use, less immediate visual polish |
| Context packs over built-in answers | Less demo flash, much stronger agent interoperability and scope control |
| Rust engine | Strong performance and memory safety, higher initial implementation cost |
| One default corpus | Faster MVP, less flexible than multi-corpus product |
| No auth in MVP | Simpler demo, not safe for real private production use |
| Provider abstraction | Slightly more code, much better engineering signal |
| Context trace logging | More metadata to manage, but stronger governance and debugging |
| Sample-doc packaged demo | Less interactive publicly, but safer before auth/privacy controls |

## Open questions

| Question | Default answer until changed |
|---|---|
| Should the product add a native companion app after MVP? | Not for MVP; keep the CLI/engine focused and build a separate V2 only if needed |
| Should embeddings be paid API or local? | Support OpenAI-compatible embeddings first; keep mock/local embeddings for tests and packaged demos |
| Should document delete ship in MVP? | Full `delete_document` API is post-MVP, but README-documented local reset/delete steps are required for MVP |
| Should the packaged demo allow public uploads? | No. Sample documents only until auth, deletion, retention, privacy notice, and provider/legal review exist |
| Which models are initially allowed? | Mock/local providers are required for tests and packaged demos; real local/private runs use configurable embedding provider/model IDs that meet documented capabilities |
| Post-MVP: how much graph structure is needed beyond vector retrieval and MVP source links? | MVP stays at source -> section -> chunk hierarchy plus citation/source links for trace/read; expand only after vector-first retrieval is accepted and evaluation shows a clear benefit |
| Who is responsible for downstream AI answers? | A named human/team operator, not the model or the cite itself |
| When should MCP be added? | After the CLI JSON contract is stable enough to wrap without changing core schemas |
| When should full hybrid search be added? | After vector-first retrieval with minimal source metadata meets MVP acceptance |

## Cut lines if schedule is tight

If time gets short, cut in this order:

1. PDF page numbers.
2. Retrieval scores in human-readable CLI output.
3. Multiple file import.
4. Advanced graph relations beyond source/section/chunk/citation links.
5. Native app work.

Do not cut:

- citations;
- source snippet inspection;
- context-pack JSON;
- no-results behavior when context is insufficient;
- ingest-retrieve-read happy path;
- README and setup instructions;
- basic tests;
- privacy warning;
- provider disclosure;
- context trace metadata;
- durable local state for locks, indexes, and rate limits.

## Decision log

| Date | Decision | Reason |
|---|---|---|
| 2026-05-26 | Build PRD before coding | User wants product clarity and production-quality MVP boundaries. |
| 2026-05-26 | MVP focuses on a CLI-first grounded engine | The cite is the product value, not a desktop UI. |
| 2026-05-26 | Start without auth | Reduces scope and keeps focus on engine behavior. |
| 2026-05-26 | Packaged sample demo uses sample documents only | Avoids privacy risk while auth is out of scope. |
| 2026-05-26 | MVP corpus is mode-scoped | Public packaged demo retrieves sample documents only; local/private demo retrieves imported documents only; no mixed corpus in MVP. |
| 2026-05-26 | Document deletion API is post-MVP, but manual reset/delete docs are MVP | Keeps core scope small while giving local/private users a safe cleanup path. |
| 2026-05-26 | Native app moves to a separate V2 doc | Keeps the MVP focused on the CLI/engine contract. |
| 2026-05-26 | Local/private MVP stores original files | CLI-managed local storage keeps original files together with extracted text, chunks, embeddings, indexes, minimal graph/source metadata, and metadata so citations and cleanup are stable. |
| 2026-05-26 | Chile privacy laws are PRD requirements | Ley 19.628 and Ley 21.719 must be considered before production use. |
| 2026-05-26 | Provider/model registry is required | The registry tracks configurable provider/model IDs and required capabilities; named commercial models are non-normative candidates until access, pricing, terms, and privacy review are verified. |
| 2026-05-26 | CLI-first cite with Rust engine | The product is local-first and the CLI surface matches the new scope. |
| 2026-05-26 | Retrieval is vector-first with minimal graph/source metadata | Avoids overbuilding full hybrid ranking, graph expansion, or reranking before the core is reliable. |
| 2026-05-26 | Runtime modes are explicit | Public packaged demo disables uploads, local/private demo enables uploads with warnings, and production stays blocked until compliance is complete. |
| 2026-05-26 | Built-in answer generation is post-MVP | The MVP must help external agents retrieve and verify document context instead of becoming an app-hosted assistant. |
| 2026-05-26 | MCP access is post-MVP | Stabilize the CLI JSON contract first, then wrap it as MCP later if useful. |
| 2026-05-26 | Single-shot durable process model | Matches normal CLI/agent invocations and avoids a required daemon in MVP. |

## Related docs

- [Product Brief](./01-product-brief.md)
- [MVP Scope](./03-mvp-scope.md)
- [Acceptance Criteria](./10-acceptance-criteria.md)
- [Legal and Privacy Compliance](./12-legal-privacy-compliance.md)
- [AI Ethics and Governance](./13-ai-ethics-governance.md)
