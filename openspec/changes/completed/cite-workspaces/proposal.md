# Proposal: Cite Workspaces + Check-docs

**Status:** Draft
**Author:** el Gentleman + rikar
**Created:** 2026-06-06
**RFCs:** rfc-auto-docs-sync.md, EVALUACION_CITE.md

---

## Problem

### 1. Single global database limits portability and structure

Cite currently stores everything in a single `cite.db` at `~/.local/share/cite/` (or `CITE_DATA_DIR`). This creates two problems:

- **No portability:** You can't move project-specific knowledge between machines or share it with a team. Everything is mixed together.
- **No structure:** All documents live in the same flat space. No way to separate "project A docs" from "project B docs" from "general knowledge."

### 2. Documentation drifts from binary behavior

The EVALUACION_CITE.md found concrete examples:
- `agent-usage-guide.md` says compact mode is a proposal → it's been the default since v0.2.0
- `--topic` and `--concept` filters are documented but don't work
- No mechanism to detect these desyncs automatically

---

## Proposed Solution

### Feature 1: Workspaces

Introduce a two-tier storage model:

```
┌─────────────────────────────────────────────────────────┐
│                    cite workspace                        │
├─────────────────────────────────────────────────────────┤
│  ~/.local/share/cite/cite.db     → Global (shared)      │
│  ./cite.db                        → Project (local)      │
│                                                         │
│  Engine reads both, project takes priority on conflict  │
└─────────────────────────────────────────────────────────┘
```

**Key behaviors:**
- Global DB: the existing `cite.db` — general knowledge, shared across all projects
- Project DB: a new `cite.db` in the project root — project-specific, transportable
- Search/retrieve queries both DBs, deduplicates, project results take priority
- `cite workspace init` creates the project DB
- `cite workspace status` shows which DBs are active and their stats

### Feature 2: Check-docs

A command that verifies documentation against the current binary:

```
cite check-docs openspec/guides/agent-usage-guide.md
```

**Key behaviors:**
- Parses markdown for fenced code blocks (bash commands)
- Executes each command against the current `cite` binary
- Compares actual output vs documented expected output
- Reports: ✅ OK / ❌ OUTDATED / ⚠️ WARNING
- Batch mode: `cite check-docs openspec/ --recursive`

---

## Scope

### In scope

- [ ] Workspace infrastructure (dual-DB resolution, priority merging)
- [ ] CLI commands: `cite workspace init`, `cite workspace status`
- [ ] Project DB creation and detection
- [ ] Search/retrieve across both DBs with deduplication
- [ ] Check-docs: markdown parser + command executor + comparator
- [ ] Check-docs: human-readable report output
- [ ] Metadata headers for behavioral docs (verified_with, last_verified)

### Out of scope

- [ ] Auto-fix outdated docs (suggest only, don't modify)
- [ ] CI integration (GitHub Action) — future phase
- [ ] Auto-generate docs from binary
- [ ] Streaming for workspace queries
- [ ] Demo precargado with app docs — separate work
- [ ] Workspace merge strategy (manual conflict resolution) — future
- [ ] Remote/shared workspaces (cloud sync) — future

---

## Benefits

1. **Portability:** Project DB is a single file you can commit, share, or move
2. **Structure:** Clear separation between general knowledge and project-specific
3. **Doc trust:** Automated verification catches desyncs before users/agents do
4. **Agent safety:** Agents won't follow outdated instructions
5. **Demo-ready:** Project DB pattern enables the "precargado demo" later

---

## Risks

| Risk | Mitigation |
|------|------------|
| Dual-DB adds complexity to queries | Keep resolution simple: project > global, deduplicate by document_id |
| Performance: two DB reads per query | SQLite is fast; measure and optimize if needed |
| Check-docs: dynamic output (latency, UUIDs) | Skip/regex-match variable values |
| Check-docs: false positives | Start with exact match, add semantic comparison in Phase 2 |
| Project DB in root clutters repo | Allow `.cite/` directory as alternative location |

---

## Decisions (2026-06-06)

1. **Project DB location:** `.cite/cite.db` directory pattern. Support `.cite.db` in root as fallback for simple projects.

2. **Workspace detection:** Auto-detect. If `.cite/cite.db` or `.cite.db` exists in cwd, use it as project workspace. `--global` flag forces global-only mode.

3. **Priority on conflict:** Project always wins. If same document_id exists in both DBs, project version takes precedence.

4. **Check-docs scope:** Only `cite` commands for MVP. Less false positives, covers the main use case.

---

## Open Questions

1. **Should `--workspace` flag exist for explicit path override?** Not MVP, but consider for future.

2. **What happens to ingest when workspace is active?** Default to project DB? Flag to target global? Recommendation: ingest goes to project by default, `--global` flag for global DB.

---

## Dependencies

- Existing `cite` binary (v0.3.0+)
- SQLite (already used)
- No new external dependencies expected

---

## Success Criteria

1. `cite workspace init` creates a project DB that works alongside global
2. `cite search` returns results from both DBs, deduplicated
3. `cite check-docs` correctly identifies the known desync in agent-usage-guide.md
4. All existing tests still pass
5. No performance regression > 20% on standard queries
