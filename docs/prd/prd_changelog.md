# PRD Changelog

This file is append-only. Add one row for every PRD revision so the team can track what changed, which file changed, who audited it, and which model implemented it.

| Date | File | Change | Human Auditor | Implementing Agent | Model Used | Notes |
|---|---|---|---|---|---|---|
| 2026-05-26 | `docs/prd/README.md` | Reframed the PRD around a CLI-first AI cite and added the future native app and changelog to the document map. | TBD | worker | gpt-5.4-mini | Initial CLI-first rewrite. |
| 2026-05-26 | `docs/prd/01-product-brief.md` | Rewrote the product thesis around a private semantic document engine exposed via CLI for agent use. | TBD | worker | gpt-5.4-mini | Initial CLI-first rewrite. |
| 2026-05-26 | `docs/prd/02-users-and-problems.md` | Replaced UI/reviewer framing with corpus owner, integrator, and consumer-agent personas. | TBD | worker | gpt-5.4-mini | Initial CLI-first rewrite. |
| 2026-05-26 | `docs/prd/03-mvp-scope.md` | Moved the MVP center to the CLI/engine and pushed the native app into a separate future/V2 document. | TBD | worker | gpt-5.4-mini | Initial CLI-first rewrite. |
| 2026-05-26 | `docs/prd/04-functional-requirements.md` | Recast requirements around CLI ingestion, query, trace, and machine-readable outputs. | TBD | worker | gpt-5.4-mini | Initial CLI-first rewrite. |
| 2026-05-26 | `docs/prd/05-non-functional-requirements.md` | Reworked quality, security, observability, and CLI ergonomics for the cite. | TBD | worker | gpt-5.4-mini | Initial CLI-first rewrite. |
| 2026-05-26 | `docs/prd/06-ux-flows.md` | Replaced screen-based flows with CLI and agent journeys for ingest, query, verify, and retry. | TBD | worker | gpt-5.4-mini | Initial CLI-first rewrite. |
| 2026-05-26 | `docs/prd/07-system-architecture.md` | Re-centered the architecture on the Rust engine, CLI surface, semantic graph, and future adapters. | TBD | worker | gpt-5.4-mini | Initial CLI-first rewrite. |
| 2026-05-26 | `docs/prd/08-ai-retrieval-design.md` | Added a semantic hierarchy and graph-expanding retrieval design for grounded answers. | TBD | worker | gpt-5.4-mini | Initial CLI-first rewrite. |
| 2026-05-26 | `docs/prd/09-api-contract.md` | Rewrote the contract as CLI commands with JSON output, traces, errors, and exit codes. | TBD | worker | gpt-5.4-mini | Initial CLI-first rewrite. |
| 2026-05-26 | `docs/prd/10-acceptance-criteria.md` | Updated acceptance criteria to match the CLI-first engine, trace flow, and demo modes. | TBD | worker | gpt-5.4-mini | Initial CLI-first rewrite. |
| 2026-05-26 | `docs/prd/11-risks-open-questions.md` | Updated risks, trade-offs, and decision log for the CLI-first cite and future native app. | TBD | worker | gpt-5.4-mini | Initial CLI-first rewrite. |
| 2026-05-26 | `docs/prd/12-legal-privacy-compliance.md` | Reframed privacy and legal guidance around the private corpus, traces, and CLI output. | TBD | worker | gpt-5.4-mini | Initial CLI-first rewrite. |
| 2026-05-26 | `docs/prd/13-ai-ethics-governance.md` | Reworked governance for CLI traces, model registry, and human accountability. | TBD | worker | gpt-5.4-mini | Initial CLI-first rewrite. |
| 2026-05-26 | `docs/prd/14-future-native-app.md` | Added a dedicated V2 companion-app document that depends on the CLI/engine. | TBD | worker | gpt-5.4-mini | Initial CLI-first rewrite. |
| 2026-05-26 | `docs/prd/15-market-landscape.md` | Added a market and positioning note covering competitor clusters, differentiation, and strategic principles. | TBD | worker | gpt-5.4-mini | Initial market-positioning appendix. |
| 2026-05-26 | `docs/prd/*` | Judgment Day Round 1 fix: pivoted PRD from app-hosted answerer to CLI context cite; added context-pack commands, single-shot durable process model, config contract, vector-first positioning, and deferred MCP/hybrid/native/answer-adapter scope. | TBD | worker | gpt-5.4-mini | Surgical documentation fix after dual adversarial review. |
| 2026-05-26 | `docs/prd/*` | Round 2 polish: clarified explicit ingest backlog processing, added `refresh`, mapped errors to exit codes, scoped `responsible_owner`, moved rate-limit state to durable data, and limited MVP graph scope to minimal source metadata. | TBD | worker | gpt-5.4-mini | Polish pass for single-judge suspect findings. |
| 2026-05-26 | `docs/prd/*` | Verification polish: made ingest lock/backlog upsert atomic/idempotent, defined refresh snapshot semantics and state errors, added two-threshold result-kind rules, `chunk_not_found`, lock-conflict details, canonical rate-limit key, and post-MVP graph wording. | TBD | worker | gpt-5.4-mini | Surgical fix for polish verification warnings. |
