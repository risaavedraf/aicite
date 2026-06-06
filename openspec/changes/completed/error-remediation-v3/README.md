# Error Remediation V3 — Deferred Items

**Status:** 🔲 NOT STARTED
**Depends on:** error-remediation-v2 (second pass) merged to main

## Scope

| Item | Tier | Est. Files | Risk | Priority |
|------|------|-----------|------|----------|
| C9 Newtype migration | 🔴 | ~50 | High | 1 |
| H7 Snapshot rollback | 🟠 | 2-3 | Medium | 2 |
| created_at type consistency | 🟠 | 5-10 | Medium | 3 |
| H19 ScoredChunk dedup | 🟠 | 1-2 | Low-Med | 4 |
| Snapshot updated_at migration | 🟡 | 2-3 | Low | 5 |

## Quick Start

Lee `third-pass-prompt.md` para el contexto completo y los requisitos SDD.

## Dependencies

- C9 (newtypes) es independiente de los demás
- H7 y snapshot_updated_at tocan los mismos archivos (snapshots)
- created_at consistency y H19 pueden agruparse en un PR de type cleanup
