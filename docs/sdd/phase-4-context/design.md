# SDD Design — Phase 4: Context Packs + Citations

## Architecture

```
CLI (context/read/trace)
  -> engine::context::{context,read,trace}
      -> providers::EmbeddingProvider::embed(query)   [context]
      -> storage::Database::{retrieval + trace/citation lookups}
      -> retrieval::{rank_by_similarity}
      -> context assembler (result_kind + citation shaping + metadata)
      -> trace recorder + read resolvers
  -> CLI formatter (json/human + disclaimer)
```

## Module changes

### `crates/storage`

- Extend schema for deterministic trace/citation resolution.
- Add APIs for:
  - persisting context trace envelopes,
  - persisting citation records linked to traces/context pack,
  - lookup by `(trace_id, citation_id)`,
  - lookup by `(document_id, chunk_id)` constrained to current ready snapshot,
  - loading trace details for `harness trace`.
- Keep compatibility with existing retrieval storage primitives.

### `crates/engine`

- Add `context` module to orchestrate:
  - query and `top_k` validation,
  - query embedding + ranking reuse,
  - threshold decision (`result_kind`),
  - citation shaping,
  - context metadata assembly,
  - trace persistence and retrieval.
- Add deterministic `read` resolver for both selector modes.
- Keep result-kind logic centralized to avoid CLI drift.

### `crates/cli`

- Add commands:
  - `commands/context.rs`
  - `commands/read.rs`
  - `commands/trace.rs`
- Wire command enum + dispatcher in `main.rs` and `commands/mod.rs`.
- Ensure stable JSON contracts and human-readable output with disclaimer.

### `crates/common`

- Add/align DTOs for context pack and trace output contracts.
- Ensure error mapping covers contract-required codes and safe detail fields.

## Persistence model decision

Two options were identified:
1. **Normalized tables** for trace headers + trace citations (recommended for deterministic lookup and safer evolution).
2. Store citation IDs only as JSON in trace rows and reconstruct from chunk data (higher ambiguity risk for `read`).

MVP recommendation: normalized or quasi-normalized storage that guarantees unambiguous `(trace_id, citation_id)` read behavior and stable `trace` outputs.

## Result-kind + readiness behavior

Decision boundaries:
- Threshold table controls `result_kind` for evidence quality.
- Corpus readiness controls whether retrieval can run at all.

Proposed rule:
- If active corpus has zero ready docs: return `document_not_ready`.
- If active corpus has mixed readiness: run on ready subset + emit excluded counts.
- Then apply result-kind table on retrieved candidates.

## Safety controls

- Keep citation snippet text in command outputs where contract requires.
- Keep logs/errors on strict safe-field allowlist.
- Never include raw provider payloads or full prompt/query internals in error details.

## Performance (MVP)

- Reuse Phase 3 O(N*d) ranking path.
- Additional overhead is context/trace shaping and persistence writes.
- Acceptable for MVP corpus sizes; ANN/reranking remains future work.

## Delivery strategy and review workload

Given estimated 900–1300 changed lines and 400-line review budget:
- deliver in **3 chained slices**,
- gate each slice with tests/lint/fmt,
- review before moving to next slice.
