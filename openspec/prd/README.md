# Product Requirements — AI Cite CLI

This PRD defines a CLI-first cite that helps AI agents and technical operators read, retrieve, and use information from private document corpora. The MVP is not a native app and does not embed an assistant as the product center; it exposes reliable document context through stable CLI commands and JSON schemas.

## Document map

| File | Purpose |
|---|---|
| [01 — Product Brief](./01-product-brief.md) | Product thesis, value proposition, and non-goals |
| [02 — Users and Problems](./02-users-and-problems.md) | Personas, jobs, and user pain points |
| [03 — MVP Scope](./03-mvp-scope.md) | MVP boundary, runtime modes, and deferred scope |
| [04 — Functional Requirements](./04-functional-requirements.md) | Required CLI, engine, retrieval, and context behavior |
| [05 — Non-Functional Requirements](./05-non-functional-requirements.md) | Quality, security, config, observability, and reliability |
| [06 — UX Flows](./06-ux-flows.md) | CLI and agent flows for ingest, retrieve, read, and verify |
| [07 — System Architecture](./07-system-architecture.md) | CLI-first architecture, process model, and module boundaries |
| [08 — AI and Retrieval Design](./08-ai-retrieval-design.md) | Vector-first retrieval, context packs, citations, and evaluation |
| [09 — API Contract](./09-api-contract.md) | Stable CLI commands, JSON schemas, errors, and exit codes |
| [10 — Acceptance Criteria](./10-acceptance-criteria.md) | Definition of done for the MVP |
| [11 — Risks and Open Questions](./11-risks-open-questions.md) | Risks, trade-offs, cut lines, and decisions |
| [12 — Legal and Privacy Compliance](./12-legal-privacy-compliance.md) | Privacy surfaces, runtime policies, and production checklist |
| [13 — AI Ethics and Governance](./13-ai-ethics-governance.md) | Accountability, provider registry, and trace requirements |
| [14 — Future Native App](./14-future-native-app.md) | V2 companion UI that wraps the CLI/engine |
| [15 — Market Landscape](./15-market-landscape.md) | Market clusters, differentiation, and positioning |
| [PRD Changelog](./prd_changelog.md) | Append-only record of PRD revisions |

## MVP north star

> If an external AI agent cannot call the CLI, receive cited context, inspect source snippets, and understand why no context was returned, the MVP is not done.

## Current product frame

| Area | Decision |
|---|---|
| Product form | Local-first CLI cite for agents and operators |
| Core value | Agent-consumable document context with citations, source inspection, and traceability |
| Main proof | Ingestion, vector-first retrieval, context packs, stable JSON, durable local state, and privacy/governance guardrails |
| Explicit non-goal | Native app, chat UI, hosted assistant, and built-in answer generation as MVP scope |
