# SDD Explore Notes — Phase 5: Durability

## Goal

Define the implementation path for Phase 5 durability features on top of completed Phase 4 contracts, with delivery slices that stay under the 400-line review budget.

## Boundary with completed Phase 4

Phase 4 is already implemented and active across:
- `crates/engine/src/context.rs` (`build_context`, `read_context`, `get_trace`)
- `crates/storage/src/traces.rs` + migration `002_trace_citations.sql`
- CLI commands:
  - `crates/cli/src/commands/context.rs`
  - `crates/cli/src/commands/read.rs`
  - `crates/cli/src/commands/trace.rs`

Phase 5 must **add durability guarantees** without changing the context/citation output contracts introduced in Phase 4.

---

## Current-state findings (by deliverable)

## 1) Durable ingestion locks (sequential processing)

Current state:
- Ingest path is direct and non-serialized:
  - `crates/cli/src/commands/ingest.rs` → `engine::ingest::ingest`
  - `crates/engine/src/ingest.rs` creates document, sets `processing`, runs pipeline.
- SQLite WAL + busy timeout exists (`crates/storage/src/lib.rs`) but this is DB-level contention handling, not a logical durable ingest lock.
- No lock table or lock lease metadata in schema (`001_initial.sql`, `002_trace_citations.sql`).

Gap:
- No explicit durable lock ownership, expiry, or conflict response payload.

## 2) Backlog/upsert on lock conflict (`operation_in_progress`)

Current state:
- Error enum already supports `OperationInProgress` with `retry_after_seconds` and `lock_name`:
  - `crates/common/src/error.rs`
- Exit code mapping already supports this (`ExitCode::OperationInProgress = 6` in `crates/common/src/exit.rs`).
- No backlog queue schema or upsert API in storage.
- Ingest currently returns immediate success/failure only; no enqueue path.

Gap:
- Missing queue persistence and conflict path that returns `operation_in_progress` while upserting backlog item.

## 3) `cite ingest --next` / `--queued`

Current state:
- CLI ingest args currently only: `path`, optional `--display-name` (`crates/cli/src/commands/ingest.rs`).
- No `--next` or `--queued` modes.
- No storage APIs to fetch next queued item or list queued backlog.

Gap:
- Missing command flags, engine orchestration for queue processing, and queue read models.

## 4) Durable retrieval/context rate limiting (20 req/min per key)

Current state:
- Config includes `rate_limit.max_requests` and `rate_limit.window_seconds` defaults (`crates/config/src/lib.rs`).
- `search`/`retrieve`/`context` flows do not consult rate limits:
  - `crates/engine/src/retrieve.rs`
  - `crates/engine/src/context.rs`
- No persistent rate-limit table/counters in migrations.
- `RateLimitExceeded` error exists in common errors, but unused in retrieval/context execution paths.

Gap:
- Missing durable counters, keying strategy, atomic check+increment, and enforcement hooks in engine entrypoints.

## 5) `cite retry` behavior alignment for failed docs

Current state:
- CLI retry command exists: `crates/cli/src/commands/retry.rs`.
- Engine retry exists: `engine::ingest::retry_document`.
- Behavior today:
  - requires document status `failed`,
  - requires source file still exists,
  - cleanup partial data,
  - set status to `pending`,
  - reset retry count to 0 (`db.reset_retry_count`).
- Comment in engine says “increment retry count” but implementation resets to 0 (code/comment mismatch).
- `next_retry_at` field exists in schema/types but is not used to schedule retries.

Gap:
- Clarify/align retry semantics for Phase 5 durability (manual retry vs queued retry metadata), and fix code/comment contract drift.

## 6) `cite refresh` with atomic snapshot swap

Current state:
- No `refresh` command in CLI:
  - absent from `crates/cli/src/main.rs` command enum.
- No snapshot/version tables in schema.
- Retrieval reads “ready docs/chunks” directly from live tables (`list_ready_chunk_embeddings` path).

Gap:
- Missing snapshot model, staged refresh workflow, and atomic promotion/swap operation.

## 7) Recovery of interrupted `processing` docs on startup

Current state:
- No startup recovery hook in CLI bootstrap (`crates/cli/src/main.rs`).
- Commands open DB ad hoc per command; no centralized bootstrap guard.
- `processing` is a valid status in `DocumentStatus`, but no recovery routine currently scans/repairs stale processing docs.

Gap:
- Missing startup/command-entry recovery routine to reclassify interrupted work and/or enqueue continuation.

---

## Risks

1. **State-machine drift**  
   If lock/backlog/retry/recovery transitions are added incrementally without a single transition contract, document states can become inconsistent.

2. **Partial durability implementation**  
   Implementing lock errors without persistent queue upsert can create UX dead-ends (`operation_in_progress` but no resumable path).

3. **Rate-limit key ambiguity**  
   Without a deterministic key strategy (runtime/user/API-key/process scope), durable counters may throttle incorrectly.

4. **Refresh atomicity complexity**  
   Snapshot swap touches ingest/retrieval assumptions; done poorly, it can expose mixed-state reads.

5. **Review budget overrun**  
   Phase 5 crosses migrations + engine + CLI; unsliced delivery likely exceeds 400-line review budget.

---

## Non-goals (for this phase slice planning)

- No ANN/reranking retrieval upgrades.
- No evaluation/golden dataset work (Phase 6).
- No packaging/release workflow work (Phase 7).
- No answer-generation layer beyond context retrieval contracts.
- No CLI binary rename/system-wide command rename.

---

## Recommended delivery slices (<= 400 changed lines each)

### Slice 1 — Durable ingest lock + backlog upsert + conflict contract
Scope:
- New migration for lock + backlog tables.
- Storage APIs:
  - acquire/release ingest lock,
  - upsert backlog entry.
- Engine ingest conflict path:
  - when lock held, upsert backlog and return `CiteError::OperationInProgress { retry_after_seconds, lock_name }`.
- CLI ingest surfaces stable error JSON/human output.
Target files:
- `crates/storage/src/migrations/003_*.sql`
- `crates/storage/src/migrations/mod.rs`
- `crates/storage/src/*` (new lock/backlog module)
- `crates/engine/src/ingest.rs`
- `crates/cli/src/commands/ingest.rs`
- focused unit tests in storage/engine

### Slice 2 — Queue operations: `ingest --queued` and `ingest --next`
Scope:
- Extend CLI args for queue modes.
- Engine paths:
  - enqueue-only mode,
  - process-next mode with lock acquisition.
- Storage APIs:
  - list queued,
  - claim next queued atomically.
Target files:
- `crates/cli/src/commands/ingest.rs`
- `crates/engine/src/ingest.rs`
- queue storage module + tests

### Slice 3 — Durable rate limits for search/retrieve/context
Scope:
- Migration for rate-limit counters.
- Storage API: check+increment per key/window (durable).
- Engine guards in:
  - `retrieve::search`
  - `retrieve::retrieve`
  - `context::build_context`
- Return `RateLimitExceeded { retry_after_seconds }` consistently.
Target files:
- `crates/storage/src/migrations/004_*.sql`
- `crates/storage/src/*rate_limit*.rs`
- `crates/engine/src/retrieve.rs`
- `crates/engine/src/context.rs`
- tests for window rollover + persistence across DB reopen

### Slice 4 — Retry alignment + interrupted processing recovery
Scope:
- Align retry contract/comment/behavior.
- Decide and codify `retry_count` semantics for manual retry in Phase 5.
- Add recovery routine for stale `processing` docs at command startup path (or shared bootstrap helper).
Target files:
- `crates/engine/src/ingest.rs`
- `crates/storage/src/documents.rs` (query/update helpers)
- CLI bootstrap/helper wiring (likely `crates/cli/src/main.rs` + command helpers)
- tests for interrupted processing transitions

### Slice 5 — `cite refresh` + atomic snapshot swap
Scope:
- Add refresh command and engine orchestration.
- Add snapshot metadata/state tables.
- Implement atomic promote/swap semantics.
- Ensure retrieval/context read from active snapshot only.
Target files:
- `crates/cli/src/main.rs` + `crates/cli/src/commands/refresh.rs`
- `crates/engine/src/*refresh*.rs` (or ingest/context integration)
- storage migrations + snapshot storage module
- end-to-end tests for atomic visibility

---

## Suggested acceptance checks for Slice 1 (first implementation slice)

- On concurrent ingest attempt:
  - command returns `operation_in_progress` (exit code 6),
  - backlog row exists/upserts deterministically.
- Lock holder can complete and release lock.
- Duplicate enqueue attempts for same source do not create unbounded duplicates (upsert verified).
- Existing happy-path ingest remains unchanged when no lock conflict occurs.
