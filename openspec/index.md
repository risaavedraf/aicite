# Documentation Index

Welcome to the AI Cite CLI documentation. This index provides a map to all project documentation.

## Product Requirements (PRD)

The PRD defines the complete product specification, from vision to acceptance criteria.

| Document | Purpose |
|---|---|
| [PRD Overview](prd/README.md) | PRD document map and MVP north star |
| [01 — Product Brief](prd/01-product-brief.md) | Product thesis, value proposition, and non-goals |
| [02 — Users and Problems](prd/02-users-and-problems.md) | Personas, jobs, and user pain points |
| [03 — MVP Scope](prd/03-mvp-scope.md) | MVP boundary, runtime modes, and deferred scope |
| [04 — Functional Requirements](prd/04-functional-requirements.md) | CLI, engine, retrieval, and context behavior |
| [05 — Non-Functional Requirements](prd/05-non-functional-requirements.md) | Quality, security, config, observability |
| [06 — UX Flows](prd/06-ux-flows.md) | CLI and agent flows |
| [07 — System Architecture](prd/07-system-architecture.md) | Architecture, process model, module boundaries |
| [08 — AI Retrieval Design](prd/08-ai-retrieval-design.md) | Vector-first retrieval, context packs, citations |
| [09 — API Contract](prd/09-api-contract.md) | CLI commands, JSON schemas, errors, exit codes |
| [10 — Acceptance Criteria](prd/10-acceptance-criteria.md) | Definition of done |
| [11 — Risks and Open Questions](prd/11-risks-open-questions.md) | Risks, trade-offs, cut lines |
| [12 — Legal and Privacy](prd/12-legal-privacy-compliance.md) | Privacy surfaces, runtime policies |
| [13 — AI Ethics](prd/13-ai-ethics-governance.md) | Accountability, provider registry, traces |
| [14 — Future Native App](prd/14-future-native-app.md) | V2 companion UI |
| [15 — Market Landscape](prd/15-market-landscape.md) | Market clusters, differentiation |

## SDD (Spec-Driven Development)

Phase-by-phase design artifacts, specs, and task breakdowns. All SDD artifacts live in `openspec/changes/`, organized by status.

| Document | Purpose |
|---|---|
| [SDD Overview](changes/README.md) | SDD documentation index |
| [Roadmap](changes/roadmap.md) | Phase roadmap and status |
| [v0.2 Phase Map](changes/v0.2-phase-map.md) | v0.2 phase plan |

### Active — current work in progress

| Directory | Purpose |
|---|---|
| [error-remediation](changes/active/error-remediation/) | First-pass error remediation |
| [error-remediation-v2](changes/active/error-remediation-v2/) | Second-pass error remediation |
| [error-remediation-v3](changes/active/error-remediation-v3/) | Verification pass (active) |

### Completed — verified and done

| Directory | Purpose |
|---|---|
| [phase-10](changes/completed/phase-10-hierarchical-graph-foundation/) | Hierarchical graph foundation |
| [phase-11](changes/completed/phase-11-hierarchical-retrieval/) | Hierarchical retrieval |
| [phase-12](changes/completed/phase-12-agent-ux/) | Agent UX improvements |

### Archive — historical phases

Phases 1–9 and install-setup-ux are in `changes/archive/`. See the [roadmap](changes/roadmap.md) for the full phase history.

## Guides

User-facing guides for installation, usage, and demos.

| Document | Purpose |
|---|---|
| [Installation Guide](guides/installation.md) | All install methods: manual, script, package managers |
| [Agent Usage Guide](guides/agent-usage-guide.md) | How AI agents consume the CLI |
| [Demo Guide](guides/demo.md) | 5-minute demo tracks: packaged and local |

## Architecture

Technical architecture documents and design proposals.

| Document | Purpose |
|---|---|
| [Rename to Cite](architecture/rename-to-cite.md) | CLI identity and runtime naming policy |
| [v0.2.0 Hierarchical Graph](architecture/v0.2.0-hierarchical-graph.md) | Proposed hierarchical graph architecture |
| [Hybrid Notes Ingestion](architecture/cite-notes-hybrid.md) | Notes ingestion design (CLI + front‑matter) |
| [Front‑Lobe Engine](architecture/front-lobe-engine.md) | Orchestrator that uses Cite as evidence store |

## RFCs

Requests for comment, organized by status.

### Active — pending proposals for this project

| Document | Purpose |
|---|---|
| [Hybrid Notes Ingestion](rfc/active/rfc-notes-hybrid.md) | Notes ingestion via CLI + front‑matter |
| [Front‑Lobe Engine](rfc/active/rfc-front-lobe-engine.md) | Orchestrator that uses Cite as evidence store |

### Completed — implemented RFCs

| Document | Purpose |
|---|---|
| [Install & Setup UX](rfc/completed/rfc-install-setup-ux.md) | Installation script and setup wizard (implemented in v0.2.0) |

### Ideas — related projects and future explorations

| Document | Purpose |
|---|---|
| [CITE-Pi Integration](rfc/ideas/rfc-cite-pi-integration.md) | Pi extension with local embedding model |
| [Landing Page](rfc/ideas/rfc-landing-page.md) | Leptos + GitHub Pages landing |
| [RAG Benchmark Framework](rfc/ideas/rfc-rag-benchmark-framework.md) | Systematic RAG evaluation methodology |

## Improvements

Inbox for external documents and ideas brought into the project.

### Ideas — future explorations

| Document | Purpose |
|---|---|
| [CITE + Pi Integration Guide](improvements/ideas/CITE_Pi_Integration.md) | Integration guide (Spanish) |
| [RAG Benchmark Guide](improvements/ideas/RAG_Benchmark_Guide.md) | Benchmark methodology (Spanish) |

## Reports

Code quality, structure, and compliance review reports.

| Document | Purpose |
|---|---|
| [Error Tracking](reports/error-tracking.md) | Error tracking log |

### Archive — historical reviews

Past code reviews and analysis reports are in `reports/archive/revision-repo/`.

