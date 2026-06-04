# SDD Proposal — Phase 3: Retrieval Pipeline

## Change name

`phase-3-retrieval` — Vector-first semantic retrieval over ready documents

## Problem

After Phase 2, documents can be ingested but cannot be semantically queried. There is no runtime path to rank chunks by relevance, so downstream context assembly (Phase 4) is blocked.

## Proposed change

Implement the retrieval pipeline: query embedding, cosine ranking over ready corpus chunks, top-k selection, and two CLI entry points (`search`, `retrieve`) with source metadata.

## In scope

- Ready-corpus embedding lookup (`documents` + `chunks` + `embeddings` join)
- Cosine similarity scoring
- Top-k clipping and validation (`1..10`, default from config)
- Engine retrieval orchestration
- CLI commands `cite search` and `cite retrieve`
- Metadata in outputs: `document_id`, `display_name`, `chunk_id`, `section_id`, `chunk_index`, `page`, offsets
- Query guardrails (non-empty, max length)

## Out of scope

- Context pack IDs, citation IDs, traces persistence (Phase 4)
- Durable rate limiting/locks (Phase 5)
- ANN index / approximate nearest neighbors
- Answer generation

## Acceptance criteria

1. `cite search "<query>" --json` returns ranked hits from ready documents only.
2. `cite retrieve "<query>" --json` returns ranked hits with chunk text + metadata.
3. `--k` accepts only `1..10`; invalid values return `invalid_parameter`.
4. Empty or whitespace-only query returns `invalid_parameter`.
5. Query embedding and chunk embeddings must match dimensions; mismatches are skipped safely.
6. No ready documents (or no valid vectors) returns success with empty results.
7. `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` pass.

## Estimated size

~700-900 lines including tests and command wiring.
