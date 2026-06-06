# openspec/ — Document Index

Last updated: 2026-06-06

---

## Structure

```
openspec/
├── architecture/          # Architecture decision records
├── changes/               # SDD change artifacts
│   ├── active/            # Currently in progress (EMPTY)
│   ├── completed/         # Implemented and verified
│   └── archive/           # Old phases (1-9)
├── guides/                # User-facing guides
├── improvements/          # Ideas for future work
├── prd/                   # Product requirements
├── reports/               # Review and audit reports
├── rfc/                   # Requests for Comments
│   ├── active/            # Proposals under discussion
│   ├── completed/         # Implemented RFCs
│   └── ideas/             # Future explorations
└── specs/                 # Formal specifications
```

---

## Active Work

### RFCs (under discussion)

| RFC | Status | Description |
|-----|--------|-------------|
| `release-scope-v0.4-line.md` | **Draft** | Release slicing across the v0.4.x line; v0.5 reserved for agent interface/v1 direction |
| `rfc-cite-v1-skill-lsp.md` | **Draft** | Cite skill/LSP-like bridge and v1 agent-interface direction |
| `review-comments-v0.5-rfcs.md` | Review notes | Cross-RFC comments for v0.4.x/v0.5 scope decisions |
| `rfc-embedding-providers.md` | **Draft** | Pluggable embedding providers, reembed, doctor, resumable ingest |
| `rfc-tags-and-note-add.md` | **Draft** | Tags, note add, and retrieval quality roadmap |
| `rfc-auto-docs-sync.md` | **Implemented** | Auto-verify docs against binary; Phase 1 complete |
| `EVALUACION_CITE.md` | Evidence | Full evaluation of Cite CLI v0.3.x |
| `SESSION_CONTEXT_2026-06-06.md` | Handoff | Session state and known issues |

### Changes (in progress)

**None** — all changes are completed.

---

## Completed Changes

| Change | Date | Description |
|--------|------|-------------|
| error-remediation-v3 | 2026-06-05 | UTF-8 fixes, FK enforcement, API key validation |
| error-remediation-v2 | 2026-06-04 | Second pass error fixes |
| error-remediation | 2026-06-02 | Initial error remediation (308 tests) |
| phase-12-agent-ux | 2026-05-28 | Agent UX improvements |
| phase-11-hierarchical-retrieval | 2026-05-28 | Hierarchical retrieval |
| phase-10-hierarchical-graph-foundation | 2026-05-28 | Graph hierarchy foundation |

---

## Guides

| File | Description | Status |
|------|-------------|--------|
| `agent-usage-guide.md` | How agents use CITE | ⚠️ OUTDATED (compact mode) |
| `installation.md` | Installation instructions | ✅ Current |
| `demo.md` | Demo walkthrough | ✅ Current |

---

## RFCs (ideas — future)

| RFC | Description |
|-----|-------------|
| `rfc-cite-pi-integration.md` | Local embedding model (reduce latency) |
| `rfc-rag-benchmark-framework.md` | RAG evaluation methodology |
| `rfc-landing-page.md` | GitHub Pages landing |

---

## Priority for Next Session

1. **Review `release-scope-v0.4-line.md`** — confirm the v0.4.x release train and whether v0.5 is unnecessary for now
2. **Close/move implemented RFCs** — `rfc-auto-docs-sync.md` Phase 1 should leave active scope
3. **Plan v0.4.0 work units** — tags plus Ollama provider MVP
4. **Shape v0.5 as Cite Agent Interface** — skill/LSP-like bridge, stable tool contract, and v1 direction after v0.4.x foundations are stable

---

## Notes

- `changes/active/` is now empty — ready for new work
- All error-remediation work is completed and verified
- The evaluation (EVALUACION_CITE.md) identified documentation sync as key issue
