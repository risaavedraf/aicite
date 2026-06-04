# Phase 11 Archive — Hierarchical Retrieval

## Status: COMPLETE ✅

## Summary

Implemented hierarchical retrieval for the CITE CLI, consuming the hierarchy infrastructure built in Phase 10. The retrieval pipeline now routes between hierarchical and flat paths based on data availability and configuration.

## Deliverables

| Deliverable | Status |
|-------------|--------|
| Storage: hierarchical chunk query with JOIN | ✅ |
| Storage: has_hierarchy_data() detection | ✅ |
| Config: RetrievalConfig.use_hierarchy flag | ✅ |
| Response types: breadcrumb fields on Citation, SearchHit, RetrieveHit | ✅ |
| Engine: fetch_candidates() routing hierarchical/flat | ✅ |
| Engine: enrich_with_hierarchy() post-rank enrichment | ✅ |
| Engine: build_breadcrumb() helper | ✅ |
| CLI: --flat, --topic, --concept flags | ✅ |
| CLI: flag validation (mutual exclusivity) | ✅ |
| Tests: 8 new tests covering hierarchical retrieval | ✅ |

## Test Results

- **223 tests pass** (was 215, +8 new)
- clippy clean
- fmt clean

## Files Changed

| File | Change |
|------|--------|
| `crates/storage/src/embeddings.rs` | +HierarchicalChunkEmbedding, +list_chunk_embeddings_hierarchical(), +has_hierarchy_data() |
| `crates/retrieval/src/lib.rs` | +hierarchy fields on ScoredChunk |
| `crates/engine/src/retrieve.rs` | +build_breadcrumb(), +enrich_with_hierarchy(), +fetch_candidates(), modified search()/retrieve() |
| `crates/engine/src/context.rs` | Modified build_context() for hierarchical routing + breadcrumb in Citations |
| `crates/cli/src/commands/context.rs` | +--flat, --topic, --concept flags |
| `crates/cli/src/commands/search.rs` | +--flat, --topic, --concept flags |
| `crates/cli/src/commands/retrieve.rs` | +--flat, --topic, --concept flags |
| `crates/engine/src/evaluate.rs` | Updated build_context() caller |
| `crates/engine/tests/golden_test.rs` | Updated build_context() caller |

## Architecture Decisions

1. **fetch_candidates() pattern**: Single function handles hierarchical vs flat routing for all engine functions
2. **Post-rank enrichment**: Hierarchy metadata via HashMap lookup applied after ranking
3. **Progressive breadcrumb**: "doc > topic > concept" with fallback when concept/topic missing
4. **Auto-fallback**: When no hierarchy data exists, automatically uses flat retrieval

## Known Limitations

- Topic/concept name → ID resolution not implemented (--topic/--concept accept IDs only)
- semantic_links table still empty (Phase 12 territory)
- Compact/full response mode deferred to Phase 12

## SDD Artifacts

- `openspec/changes/phase-11-hierarchical-retrieval/proposal.md`
- `openspec/changes/phase-11-hierarchical-retrieval/specs/` (3 domain specs)
- `openspec/changes/phase-11-hierarchical-retrieval/design.md`
- `openspec/changes/phase-11-hierarchical-retrieval/tasks.md`
- `openspec/changes/phase-11-hierarchical-retrieval/verify-report.md`
