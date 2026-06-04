# Phase 12 Proposal — Agent UX (Compact/Full Mode + Evaluation)

## Problem

Every `cite context --json` response dumps 645-1500 tokens of metadata into the agent's context window. Most of that metadata (context_pack_id, query_id, instructions, 15 metadata fields, document_id, chunk_id, page, offset, confidence_label) is rarely needed per-query. Agents doing 5-10 queries per conversation burn 3000-15000 tokens on metadata alone.

Additionally, Phase 11's breadcrumb fields (topic_name, concept_name, breadcrumb) are present in the engine types but the CLI's search/retrieve output structs discard them.

The evaluation system has duplicate providers and no hierarchical fixtures.

## Goal

1. **Compact/Full response mode**: Default compact (~200-250 tokens, 60-70% reduction), `--full` flag for current behavior
2. **Fix breadcrumb passthrough**: Search and Retrieve CLI output should include Phase 11 breadcrumb fields
3. **Evaluation improvements**: Consolidate duplicate providers, add hierarchical fixtures

## Scope

### In scope
- Compact response types + transform layer in CLI
- `--full` flag on context, search, retrieve commands
- Fix search/retrieve breadcrumb passthrough
- Consolidate EvalProvider/GoldenProvider
- Add 2-3 hierarchical evaluation fixtures
- Tests

### Out of scope
- `--max-snippet-chars` (future enhancement)
- Streaming output
- Multi-query batching
- Field selection (`--fields`)

## Approach

**Compact/full via CLI post-processing** (not serde attributes):
- Engine always returns full data
- CLI adds `--full` flag (default: compact when `--json` is used)
- Compact mode maps to reduced structs with only essential fields
- Non-JSON (human-readable) output unaffected

## Estimated scope: ~530 lines

| Area | Est. Lines |
|------|------------|
| Compact response types + transform | ~80 |
| --full flag on 3 commands | ~140 |
| Fix search/retrieve breadcrumb | ~20 |
| Consolidate eval providers | ~60 |
| Hierarchical fixtures | ~80 |
| Tests | ~150 |
