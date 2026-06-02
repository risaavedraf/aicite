# Documentation Changelog

This changelog tracks documentation changes separately from code releases. Use it to understand what docs were added, updated, or improved over time.

## 2026-06-02

### Moved
- Consolidated all documentation from `docs/` into `openspec/` — single documentation root
- `docs/architecture/` → `openspec/architecture/`
- `docs/rfc/` → `openspec/rfc/` (reorganized into `active/`, `completed/`, `ideas/`)
- `docs/Improvements/` → `openspec/Improvements/`
- `docs/guides/` → `openspec/guides/`
- `docs/prd/` → `openspec/prd/`
- `docs/reports/` → `openspec/reports/`
- `docs/sdd/` → merged into `openspec/changes/`
- Eliminated `docs/` directory entirely

### Reorganized
- RFCs split into three categories:
  - `openspec/rfc/active/` — pending proposals for this project
  - `openspec/rfc/completed/` — implemented RFCs
  - `openspec/rfc/ideas/` — related projects and future explorations
- SDD artifacts consolidated: `openspec/sdd/` merged into `openspec/changes/`

---

## 2026-06-01

### Added
- `docs/rfc/rfc-cite-pi-integration.md` — RFC for Pi extension integration with local embedding model
- `docs/rfc/rfc-rag-benchmark-framework.md` — RFC for systematic RAG evaluation methodology
- `docs/guides/Clean_Code_Principles.pdf` — Reference material (moved from docs/Improvements/)
- `docs/guides/GitHub_Repo_Structure_Best_Practices.pdf` — Reference material (moved from docs/Improvements/)
- `docs/guides/Rust_Clean_Code_Best_Practices.pdf` — Reference material (moved from docs/Improvements/)
- `docs/CHANGELOG-docs.md` — This file

### Updated
- `README.md` — Added v0.2.2 to version history, documented hierarchical retrieval features, added retrieval flags table (--flat, --topic, --concept, --full, --k)

---

## 2026-05-29

### Added
- `docs/rfc/rfc-front-lobe-engine.md` — RFC for orchestration layer using CITE as evidence store
- `docs/rfc/rfc-notes-hybrid.md` — RFC for hybrid notes ingestion (`cite note add`)
- `docs/rfc/rfc-landing-page.md` — RFC for Leptos landing page on GitHub Pages
- `docs/architecture/v0.2.0-hierarchical-graph.md` — Architecture doc for hierarchical graph design
- `docs/architecture/cite-notes-hybrid.md` — Architecture doc for hybrid notes
- `docs/architecture/front-lobe-engine.md` — Architecture doc for front-lobe engine
- `docs/architecture/rename-to-cite.md` — Decision doc for Harness → CITE rename

### Updated
- `README.md` — Updated for v0.2.0 release (setup wizard, TOML config, install.sh, enhanced health)
- `CHANGELOG.md` — Added v0.2.0, v0.2.1, v0.2.2 entries

---

## 2026-05-28

### Added
- `docs/sdd/` — SDD phase artifacts for phases 1-9
- `docs/prd/` — Product requirements documents
- `docs/guides/agent-usage-guide.md` — Guide for AI agents using CITE
- `docs/guides/demo.md` — Demo walkthrough
- `docs/guides/installation.md` — Installation instructions

### Updated
- `README.md` — Initial release documentation (v0.1.0)
- `CHANGELOG.md` — Created with v0.1.0 entry

---

## Legend

- **Added**: New files or sections
- **Updated**: Modified existing content
- **Removed**: Deprecated or deleted content
- **Moved**: Relocated to different path
