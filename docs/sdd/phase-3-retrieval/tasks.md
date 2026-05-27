# SDD Tasks — Phase 3: Retrieval Pipeline

## Overview

| # | Task | Depends on | Est. lines |
|---|---|---|---|
| 1 | Add storage ready-corpus embedding lookup | — | ~180 |
| 2 | Implement cosine ranking in retrieval crate | 1 | ~180 |
| 3 | Add engine retrieval orchestration APIs | 1,2 | ~220 |
| 4 | Add CLI commands: search/retrieve | 3 | ~220 |
| 5 | Add tests + run verify commands | 1-4 | ~160 |

## Task details

### 1) Storage lookup
- Add `ChunkEmbeddingRecord` struct
- Add `Database::list_ready_chunk_embeddings()`
- Join `embeddings/chunks/documents` and decode vectors
- Unit tests for ready-only filtering and vector decode

### 2) Retrieval ranking
- Implement cosine similarity util
- Implement ranking + top-k selection
- Skip invalid candidates (dim mismatch / zero norm)
- Unit tests

### 3) Engine API
- Add `engine::retrieval` module
- Add `search()` and `retrieve()` functions
- Add query and `k` validation
- Map ranked candidates to output DTOs

### 4) CLI commands
- Add `harness search` and `harness retrieve`
- Support `--k` override
- JSON + human-friendly outputs
- Wire in `commands/mod.rs` and `main.rs`

### 5) Verify
- Run `cargo test`
- Run `cargo clippy -- -D warnings`
- Run `cargo fmt --check`
- Fix failing checks

## Review workload

Estimated diff: ~900 lines. This likely exceeds the 400-line guardrail for a single review slice, so recommend either:
1) commit in 2 work units (storage+retrieval core, then engine+cli+tests), or
2) single PR with clearly separated commits.
