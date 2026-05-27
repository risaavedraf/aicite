# Apply Progress — phase-5-durability

## Completed tasks

- [x] Slice 1: added durable ingest migration foundation (`durable_locks`, `ingest_backlog`).
- [x] Slice 1: implemented storage APIs for lock acquire/release and backlog upsert.
- [x] Slice 1: wired engine ingest lock conflict path to upsert backlog and return `operation_in_progress`.
- [x] Slice 1: kept ingest happy-path behavior when lock is acquired.
- [x] Slice 1: added tests for lock serialization, owner-bound release, backlog idempotent upsert/update, and ingest conflict behavior.
- [x] Slice 1: wired JSON error details for `operation_in_progress` (`retry_after_seconds`, `lock_name`).
- [x] Slice 2: added queue entrypoints (`harness ingest --queued <path>`, `harness ingest --next`) with clap mode validation.
- [x] Slice 2: implemented backlog claim/requeue/done/failed transitions and engine next-item processing flow.
- [x] Slice 2: added tests for enqueue, FIFO claim, next-item processing, and empty-queue behavior.
- [x] Slice 3: added persistent durable rate-limit counters migration (`rate_limit_counters`).
- [x] Slice 3: implemented storage check+increment APIs with window rollover and DB-reopen persistence tests.
- [x] Slice 3: enforced rate-limit guards for `search`, `retrieve`, and `context` returning `rate_limit_exceeded` with retry-after.

## Files changed

- `crates/storage/src/migrations/003_durable_ingest.sql` (new)
- `crates/storage/src/migrations/mod.rs`
- `crates/storage/src/migrations/004_rate_limits.sql` (new)
- `crates/storage/src/locks.rs` (new)
- `crates/storage/src/backlog.rs` (new)
- `crates/storage/src/rate_limits.rs` (new)
- `crates/storage/src/lib.rs`
- `crates/engine/src/ingest.rs`
- `crates/engine/src/retrieve.rs`
- `crates/engine/src/context.rs`
- `crates/cli/src/commands/search.rs`
- `crates/cli/src/commands/retrieve.rs`
- `crates/cli/src/commands/context.rs`
- `crates/common/src/error.rs`

## Test / verify commands run

Focused:
- `cargo test -p storage lock --quiet`
- `cargo test -p storage backlog --quiet`
- `cargo test -p engine ingest_lock_conflict --quiet`
- `cargo test -p common operation_in_progress_json_contains_retry_and_lock --quiet`

Full gates:
- `cargo test`
- `cargo fmt --check`
- `cargo clippy -- -D warnings`

## Deviations from design

- Lock lease/expiry semantics were intentionally deferred (not needed for Slice 1 acceptance checks).
- Backlog idempotency key is currently path-based (normalized source path), allowing display-name override updates on the same queued item.

## Remaining tasks

- [ ] Slice 4: retry alignment + interrupted `processing` recovery.
- [ ] Slice 5: `refresh` + atomic snapshot swap.

## Workload / PR boundary

- Boundary covered: **Phase 5 Slices 1–3** (lock/backlog foundation, queue modes, durable rate limits).
- Slice 3 commit size: ~377 insertions / 17 deletions (within 400-line review target).
- Next boundary: Slice 4 only (retry alignment + interrupted processing recovery).
