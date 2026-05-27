# Apply Progress — phase-5-durability

## Completed tasks

- [x] Slice 1: added durable ingest migration foundation (`durable_locks`, `ingest_backlog`).
- [x] Slice 1: implemented storage APIs for lock acquire/release and backlog upsert.
- [x] Slice 1: wired engine ingest lock conflict path to upsert backlog and return `operation_in_progress`.
- [x] Slice 1: kept ingest happy-path behavior when lock is acquired.
- [x] Slice 1: added tests for lock serialization, owner-bound release, backlog idempotent upsert/update, and ingest conflict behavior.
- [x] Slice 1: wired JSON error details for `operation_in_progress` (`retry_after_seconds`, `lock_name`).

## Files changed

- `crates/storage/src/migrations/003_durable_ingest.sql` (new)
- `crates/storage/src/migrations/mod.rs`
- `crates/storage/src/locks.rs` (new)
- `crates/storage/src/backlog.rs` (new)
- `crates/storage/src/lib.rs`
- `crates/engine/src/ingest.rs`
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

- [ ] Slice 2: queue processing commands (`ingest --queued`, `ingest --next`) and atomic claim flow.
- [ ] Slice 3: durable rate-limits for `search` / `retrieve` / `context`.
- [ ] Slice 4: retry alignment + interrupted `processing` recovery.
- [ ] Slice 5: `refresh` + atomic snapshot swap.

## Workload / PR boundary

- Boundary covered: **Phase 5 Slice 1 only**.
- Estimated review size impact: within target (~<400 changed lines for this slice).
