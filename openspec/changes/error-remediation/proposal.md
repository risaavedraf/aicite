# Proposal: Error Remediation — 113 Errors Across 9 Crates

**Change ID:** error-remediation
**Date:** 2026-06-02
**Status:** proposal
**Source:** `openspec/reports/revision-repo/analisis-final-v2.md` (canonical)

---

## Problem Statement

The aiharness codebase has 113 catalogued errors across 9 crates, discovered during a comprehensive crate-by-crate review. Among them:

- **11 Critical (T1):** Runtime crashes, silent data corruption, security bypasses, broken onboarding
- **19 High (T2):** Config dead-ends, silenced errors, unsafe casts, inconsistent error handling
- **37 Medium (T3):** DRY violations, dead code, test infrastructure gaps
- **38 Low (T4):** Minor cleanup, naming, documentation

The most dangerous systemic issue is **UTF-8 bytes-vs-characters confusion** across 4 crates (6 locations), which silently corrupts hierarchy assignment, chunk boundaries, and metadata for any non-ASCII text — and causes a runtime panic in display name sanitization. All existing tests use ASCII-only data, making this invisible to CI.

## Proposed Solution

Fix all 11 Critical + 12 High-impact errors in a single SDD pass, organized into **3 chained PRs** grouped by theme (not by crate). Defer T3/T4 errors to a second pass.

## Scope

### In Scope (First Pass — 35 errors, ~285 lines)

| Theme | Errors | Tier | Crates | Est. Lines |
|-------|--------|------|--------|------------|
| 1. UTF-8 bytes/chars | C2,C3,C4,C5,H10,H11,H12 | T1+T2 | common, graph, ingest | ~45 |
| 2. FK enforcement | C1 | T1 | storage | ~5 |
| 3. Production mode guard | C6,C10 | T1 | cli, engine | ~15 |
| 4. Empty API key | C7 | T1 | cli, providers | ~15 |
| 5. Rate limit composite key | C8 | T1 | engine, storage | ~10 |
| 6. Config-disconnect | H13,H14,H15 | T2 | config, providers, cli | ~50 |
| 7. Silenced errors | H6,H17 | T2 | storage, engine | ~15 |
| 8. Integer cast safety | H18 | T2 | storage | ~20 |
| 9. Provider unwrap | H3 | T2 | cli | ~15 |
| 10. Graph robustness | H8,H9 | T2 | graph | ~25 |
| 11. Misc high-tier | H1,H2,H5,H16,H19,C11 | T1+T2 | 7 crates | ~70 |

### Out of Scope (Deferred — 78 errors)

- DRY refactoring (3 themes)
- Dead code cleanup (6+ items)
- Test infrastructure (14 errors)
- Newtype migration (~50 files)
- Type consistency (3 themes)
- All M-tier (37) and L-tier (38) errors

## PR Strategy

Chained PRs with `ask-always` strategy. Review budget: < 400 lines per PR.

| PR | Theme(s) | Focus | Est. Lines | Crates |
|----|----------|-------|:----------:|--------|
| **PR-1** | 1+2 | Data integrity (UTF-8 + FK) | ~50 | common, graph, ingest, storage |
| **PR-2** | 3+4+5 | Security + Onboarding + Compliance | ~40 | cli, engine, providers, storage |
| **PR-3** | 6-11 | Config + Defensive + Robustness | ~195 | config, providers, cli, engine, graph, retrieval, common |

**Why chained:**
- PR-1 is pure data integrity — no behavioral change for ASCII, just correctness for non-ASCII + FK
- PR-2 is security/compliance — blocks production deployment if not fixed
- PR-3 is config + defensive + robustness — largest but lowest risk per change
- Each PR is independently reviewable. PR-1 has zero dependencies on PR-2/3.

## Dependencies Between Themes

```
Theme 1 (UTF-8) ──→ Theme 10 (Graph robustness)
                     [cursor positions must be char-based first]

Theme 6 (Config) ──→ Theme 1 (UTF-8)
                     [consolidate chunk field names before fixing chunker]

Theme 3 (Guard) ──→ rename production_mode first
```

Recommended execution order within each PR:
- **PR-1:** Theme 1 first (UTF-8), then Theme 2 (FK)
- **PR-2:** Theme 4 first (API key), then Theme 3 (guard), then Theme 5 (rate limit)
- **PR-3:** Theme 6 first (config), then 7→8→9→10→11

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|:----------:|:------:|-----------|
| FK enforcement surfaces orphan data in existing DBs | Low | Medium | SQLite only enforces on new writes; existing orphans persist silently |
| Config field rename breaks env vars | Medium | Low | Document migration in CHANGELOG |
| Previously silenced errors now surface | Medium | Low | Consider warn-then-fail in PR-3 |
| UTF-8 fix misses some `len()` calls | Low | High | CI grep check for suspicious `len()` usage |

## Non-Goals

1. Newtype migration (`DocumentId`/`ChunkId`/`TraceId`) — ~50-file separate effort
2. `Database` as god object — architectural refactor, not bug fix
3. Snapshot rollback completeness (H7) — deferred to second pass
4. Provider retry/backoff — feature addition, not bug fix
5. Full test coverage — deferred to test infrastructure pass

## Acceptance Criteria

- [ ] All 11 Critical errors fixed and verified
- [ ] All 12 High-impact errors fixed and verified
- [ ] UTF-8 tests pass with non-ASCII fixtures (emoji, CJK, accented)
- [ ] FK violations rejected on new writes
- [ ] Empty API key produces clear error message
- [ ] `check_ingest_allowed` is wired into ingest path
- [ ] Rate limit key is composite per FR-109
- [ ] Config fields are consumed by their intended targets
- [ ] No `.ok()`, `let _ =`, or `.unwrap_or_default()` on fallible operations
- [ ] All integer casts use `try_from`
- [ ] Each PR < 400 changed lines
- [ ] `cargo test` passes after each PR
- [ ] `cargo clippy -- -D warnings` passes after each PR

## Recommended Next Phase

→ **spec** (detailed specification per theme with exact file:line references)
