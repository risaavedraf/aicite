# SDD Proposal — Phase 5: Durability

## Change name

`phase-5-durability` — Durable locks, rate limits, backlog processing, refresh, and recovery

## Problem

Phase 4 delivered context packs and citations, but operational durability is still missing. Ingest can run concurrently without a logical lock, lock conflicts are not persisted into a backlog, retrieval/context has no enforced durable rate limiting, and there is no startup recovery for interrupted `processing` documents or `refresh` snapshot workflow. This creates reliability gaps and inconsistent behavior across CLI restarts.

## Proposed change

Implement Phase 5 durability end-to-end across storage, engine, and CLI while preserving existing Phase 4 output contracts.

Key additions:
- Durable ingest lock with explicit conflict handling
- Backlog persistence/upsert on lock conflict
- Queue processing modes (`cite ingest --queued`, `cite ingest --next`)
- Durable rate limiting for `search`, `retrieve`, and `context` (20 req/min per key)
- Retry contract alignment for failed documents
- `cite refresh` command with atomic snapshot swap
- Recovery routine for interrupted `processing` documents on startup/command entry

## In scope

- New migrations for lock/backlog, durable rate-limit counters, and refresh snapshot state
- Storage APIs for lock acquire/release, backlog upsert/claim/list, rate-limit check+increment, snapshot swap, and interrupted-processing recovery helpers
- Engine orchestration for ingest conflict path, queued processing, rate-limit enforcement, refresh, and recovery transitions
- CLI wiring for `ingest --queued`, `ingest --next`, and `refresh`
- Error/exit-code consistency for `operation_in_progress` and `rate_limit_exceeded`
- Tests for lock conflict, backlog idempotency, durable counters, queue progression, recovery, and refresh atomicity

## Out of scope

- Retrieval quality upgrades (ANN/reranking)
- Golden dataset/evaluation work (Phase 6)
- Packaging/release work (Phase 7)
- CLI binary rename
- Answer-generation layer beyond retrieval/context contracts

## Acceptance criteria

1. Concurrent ingest attempts are serialized by a durable ingest lock.
2. On lock conflict, ingest upserts a backlog record and returns `operation_in_progress` (exit code `6`) with stable error details.
3. `cite ingest --queued` enqueues without immediate processing; `cite ingest --next` claims and processes the next queued item deterministically.
4. `search`, `retrieve`, and `context` enforce durable rate limits at 20 requests/minute per key and return `rate_limit_exceeded` (exit code `7`) with retry-after information.
5. Rate-limit counters survive CLI restart/database reopen.
6. `cite retry` behavior is contract-aligned and implementation/comment drift is removed.
7. `cite refresh` performs an atomic snapshot promotion/swap; readers never observe mixed pre/post-refresh state.
8. Interrupted `processing` documents are recovered on startup/command entry via deterministic state transition policy.
9. Existing happy-path behavior from Phases 2–4 remains valid when no lock/rate-limit/recovery conditions are triggered.
10. `cargo test`, `cargo clippy -- -D warnings`, and `cargo fmt --check` pass.

## Estimated size and review workload strategy

Estimated total diff: ~900–1300 lines across migrations, storage, engine, CLI, and tests.

Given the 400-line review budget, deliver in chained slices:

- **Slice 1 (~300–380 lines):** durable ingest lock + backlog upsert + conflict contract
- **Slice 2 (~280–360 lines):** queue modes (`--queued`, `--next`) + claim/process flow
- **Slice 3 (~280–360 lines):** durable rate limits in retrieval/context
- **Slice 4 (~220–320 lines):** retry alignment + interrupted-processing recovery
- **Slice 5 (~300–400 lines):** `refresh` + atomic snapshot swap

If any slice exceeds budget, split again before review.
