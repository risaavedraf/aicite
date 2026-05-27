# MVP Scope

The MVP must be small enough to build quickly and strong enough to look like a real product that an external AI agent can use reliably.

## MVP thesis

A CLI-first private semantic document engine with cited retrieval, source inspection, context packs, durable local state, and reproducible packaging is more valuable than a broad but fragile AI assistant demo.

## In scope

| Area | MVP capability |
|---|---|
| Corpus loading | Ingest `.pdf`, `.txt`, and `.md` files in local/private mode; the public packaged demo uses sample documents only |
| Corpus management | One default private corpus in the MVP |
| Ingestion | Extract text, build a minimal source/section/chunk structure, split into chunks, create embeddings, and store metadata |
| Search/retrieval | Retrieve ranked chunks and source/citation metadata from ready documents through the CLI |
| Context packs | Emit agent-consumable JSON with chunks, citations, source metadata, retrieval scores, and trace IDs |
| Source inspection | Read cited chunks/snippets and inspect trace output from the CLI |
| Unknown handling | Return no-results or insufficient-context responses instead of inventing answers |
| Interface | Canonical CLI commands: `health`, `ingest`, `refresh`, `list`, `get`, `search`, `retrieve`, `context`, `read`, `trace`, and `retry` |
| Process model | Single-shot durable commands: each invocation persists local data/state, indexes, locks, backlog records, and rate-limit records, then exits; no required daemon in MVP |
| Observability | Structured logs for ingest attempts, ingestion state, retrieval, context-pack creation, rate limiting, and errors |
| Configuration | Documented env vars, config file locations, precedence, provider secrets, runtime mode, cache/index paths, and redacted diagnostics |
| Testing | Unit tests for chunking/retrieval helpers; command/API tests for core interfaces |
| Packaging | Reproducible local builds and a packaged CLI demo |
| Legal/privacy guardrails | Chile privacy-law baseline, demo safety policy, privacy warnings, and no-sensitive-data guidance |
| AI governance | Provider/model disclosure for embedding providers and optional external integrations, context trace requirements, and human accountability |

## MVP runtime modes

| Mode | Upload behavior | Purpose | MVP rule |
|---|---|---|---|
| Public packaged demo | Disabled | Safe public demo | Uses preloaded sample documents only; no public user uploads. |
| Local/private demo | Enabled | Developer or controlled private evaluation | Allows supported files into the private corpus after no-sensitive-data warning and provider disclosure. |
| Production | Blocked until compliance work is complete | Real users / real private data | Do not enable until auth, deletion, retention, privacy notice, and provider/legal review are done. |

## Out of scope for MVP

| Feature | Reason |
|---|---|
| Built-in answer generation / internal assistant | The MVP is a context harness for external agents; an answer adapter can be added later without defining the core product |
| Native app / desktop UI | Moves to a separate future/V2 document |
| Web companion | Not part of the MVP contract |
| MCP server/access | Deferred extension; the MVP CLI JSON contract should be good enough to wrap later |
| Full hybrid vector + keyword ranking | MVP is vector-first with minimal source/section/chunk metadata; keyword ranking, reranking, and graph expansion are deferred |
| Multi-user auth | Adds complexity; can be added after core product works |
| Billing | Not needed for this MVP |
| Fine-tuning | Grounded retrieval solves the target problem with lower cost and risk |
| Complex autonomous agent loops | The MVP exposes a tool-like CLI; broader agent orchestration can come later |
| Real-time collaborative editing | Not part of the core value proposition |
| Full enterprise RBAC | Too heavy for first release |
| Every document format | Start with PDF, TXT, MD to keep ingestion controllable |
| Public unrestricted uploads | Public packaged demo must use sample documents only until auth, deletion, retention, and privacy policy exist |

## MVP release boundary

The MVP is done when:

1. A local/private user can ingest at least one supported document into the private corpus; the public packaged demo starts from preloaded sample documents only.
2. The system builds the semantic structure during `ingest` and can refresh already-ingested sources through `refresh`, without manual scripts.
3. The user or an agent can retrieve cited context through the CLI.
4. The context pack includes citations, source snippets, retrieval scores where available, and trace metadata.
5. The user or agent can read cited snippets or trace output.
6. The system returns explicit no-results or insufficient-context output instead of hallucinating.
7. The CLI can be run locally from documented commands.
8. The output discloses provider usage when provider calls occur and warns users to verify downstream AI answers against citations.
9. Public packaged uploads stay disabled unless production privacy controls are implemented.
10. README documents config, local storage paths, durable state, and manual reset/delete steps for local/private data.

## Post-MVP candidates

- Built-in answer-generation adapter over retrieved context.
- MCP server/access wrapper around the stable CLI/machine contract.
- Auth and private user accounts.
- Multiple named corpora.
- Full `delete_document` API and automated retention workflow.
- Full hybrid vector + keyword ranking.
- Evaluation dashboard.
- Agent workflows over retrieved context.
- Native companion app if the product later needs a visual shell.
- Slack/Discord/Telegram integration.
- Admin analytics and usage tracking.
- Production-grade privacy workflow: deletion, data subject request handling, retention automation, and legal-reviewed privacy notice.

## Deferred-scope note

The pivot intentionally removes built-in answer generation, MCP access, full hybrid ranking, graph expansion/reranking, and native app work from MVP scope. Keep these ideas recoverable in future design work, but do not let them drive the first implementation.

## Related docs

- [Functional Requirements](./04-functional-requirements.md)
- [Non-Functional Requirements](./05-non-functional-requirements.md)
- [Acceptance Criteria](./10-acceptance-criteria.md)
- [Legal and Privacy Compliance](./12-legal-privacy-compliance.md)
- [AI Ethics and Governance](./13-ai-ethics-governance.md)
