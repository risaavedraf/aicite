# SDD Tasks — Phase 5: Durability

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~900–1300 |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 → PR 2 → PR 3 → PR 4 → PR 5 |
| Delivery strategy | ask-on-risk |
| Chain strategy | pending |

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: pending
400-line budget risk: High

## Dependency order

1. PR 1 must land first (lock + backlog foundation).
2. PR 2 depends on PR 1 (queue commands and claim/process flow).
3. PR 3 depends on PR 1 (durable rate-limits).
4. PR 4 depends on PR 1 (retry alignment + recovery).
5. PR 5 depends on PR 1 and should start after PR 2–4 are stable (refresh + snapshots).

## PR 1 — Durable ingest lock + backlog upsert + conflict contract

**Status:** ✅ Completed (apply session 2026-05-27)

**Target size:** 300–380 lines  
**Primary files:**
- `crates/storage/src/migrations/003_durable_ingest.sql` (new)
- `crates/storage/src/migrations/mod.rs`
- `crates/storage/src/locks.rs` (new)
- `crates/storage/src/backlog.rs` (new)
- `crates/storage/src/lib.rs`
- `crates/engine/src/ingest.rs`
- `crates/cli/src/commands/ingest.rs`

**Tasks:**
1. Add migration 003 for `durable_locks` and `ingest_backlog` tables + indexes.
2. Implement storage APIs: `try_acquire_lock`, `release_lock`, `upsert_backlog_item`.
3. Wire ingest flow to acquire lock before pipeline; on conflict upsert backlog and return `operation_in_progress` with `retry_after_seconds` and `lock_name`.
4. Keep existing ingest happy path unchanged when lock is acquired.
5. Add unit tests in storage/engine for lock conflict and backlog idempotent upsert.

**Acceptance checks:**
- Concurrent ingest conflict returns code `operation_in_progress` and exit `6`.
- Conflict path persists/upserts backlog row.
- No conflict: ingest still reaches `ready` normally.

**Verify gate:**
- `cargo test`
- `cargo clippy -- -D warnings`
- `cargo fmt --check`

## PR 2 — Queue operations: `ingest --queued` and `ingest --next`

**Status:** ⏳ Pending

**Target size:** 280–360 lines  
**Primary files:**
- `crates/cli/src/commands/ingest.rs`
- `crates/engine/src/ingest.rs`
- `crates/storage/src/backlog.rs`
- `crates/cli/src/main.rs` (if subcommand arg wiring needed)

**Tasks:**
1. Extend ingest args with queue modes (`--queued <path>`, `--next`) and mutual-exclusion validation.
2. Add engine entrypoints for enqueue-only and process-next flows.
3. Add storage APIs for `claim_next_backlog_item`, `list_backlog` as needed by command output.
4. Ensure `--next` claims exactly one item atomically and processes under ingest lock.
5. Add tests for enqueue idempotency, empty queue behavior, and single-claim semantics.

**Acceptance checks:**
- `ingest --queued` enqueues/upserts without processing.
- `ingest --next` claims one queued item and advances status deterministically.
- Empty queue returns stable success response (no crash/no false failure).

**Verify gate:**
- `cargo test`
- `cargo clippy -- -D warnings`
- `cargo fmt --check`

## PR 3 — Durable rate limiting for `search` / `retrieve` / `context`

**Status:** ⏳ Pending

**Target size:** 280–360 lines  
**Primary files:**
- `crates/storage/src/migrations/004_rate_limits.sql` (new)
- `crates/storage/src/migrations/mod.rs`
- `crates/storage/src/rate_limits.rs` (new)
- `crates/engine/src/retrieve.rs`
- `crates/engine/src/context.rs`

**Tasks:**
1. Add migration 004 for persistent `rate_limit_counters`.
2. Implement atomic storage `check_and_increment_rate_limit` API.
3. Add engine guard helper and call it at start of `search`, `retrieve`, and `build_context`.
4. Return `RateLimitExceeded { retry_after_seconds }` before retrieval work when blocked.
5. Add tests for limit hit, retry-after, window rollover, and persistence after DB reopen.

**Acceptance checks:**
- 20 req/min policy enforced for all three routes.
- Exceeded requests return `rate_limit_exceeded` and exit `7`.
- Counters survive process/database reopen.

**Verify gate:**
- `cargo test`
- `cargo clippy -- -D warnings`
- `cargo fmt --check`

## PR 4 — Retry alignment + interrupted `processing` recovery

**Status:** ⏳ Pending

**Target size:** 220–320 lines  
**Primary files:**
- `crates/engine/src/ingest.rs`
- `crates/engine/src/recovery.rs` (new)
- `crates/storage/src/documents.rs`
- `crates/cli/src/main.rs` and/or shared DB bootstrap helper(s)

**Tasks:**
1. Align retry code/comments/contract (explicit `retry_count` behavior, no ambiguity).
2. Add storage query helpers for interrupted `processing` docs.
3. Implement `recover_interrupted_processing` with idempotent transition policy.
4. Call recovery once at command startup path after DB open.
5. Add tests for recovery idempotency and retry contract alignment.

**Acceptance checks:**
- Restart recovery deterministically reclassifies stale `processing` docs.
- Re-running recovery does not double-mutate already recovered rows.
- Retry behavior and comments match output contract.

**Verify gate:**
- `cargo test`
- `cargo clippy -- -D warnings`
- `cargo fmt --check`

## PR 5 — `refresh` + atomic snapshot swap

**Status:** ✅ Completed (apply session 2026-05-27)

**Target size:** 300–400 lines  
**Primary files:**
- `crates/storage/src/migrations/005_snapshots.sql` (new)
- `crates/storage/src/migrations/mod.rs`
- `crates/storage/src/snapshots.rs` (new)
- `crates/engine/src/refresh.rs` (new)
- `crates/cli/src/commands/refresh.rs` (new)
- `crates/cli/src/commands/mod.rs`
- `crates/cli/src/main.rs`

**Tasks:**
1. Add migration 005 for snapshot metadata, membership, and active pointer tables.
2. Implement snapshot storage APIs including atomic `activate_snapshot` transaction.
3. Add engine refresh orchestration for staged build + promote/swap.
4. Add CLI `harness refresh` command and output contract.
5. Add tests proving no mixed pre/post-refresh visibility and rollback on refresh failure.

**Acceptance checks:**
- Refresh promotes snapshot atomically.
- Retrieval/context reads only active snapshot membership.
- Failed refresh keeps previous active snapshot intact.

**Verify gate:**
- `cargo test`
- `cargo clippy -- -D warnings`
- `cargo fmt --check`

## Workload risk note and split recommendation

Phase 5 is over the 400-line budget if delivered as one PR. Keep the five-PR chain above. If any PR drifts above ~400 changed lines, split again by module boundary (migration/storage first, then engine/CLI/tests).