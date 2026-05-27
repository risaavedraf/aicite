# SDD Design — Phase 5: Durability

## Architecture

```text
CLI
  -> commands::{ingest,retry,search,retrieve,context,refresh}
    -> engine::{ingest,retrieve,context,refresh,recovery}
      -> storage::Database::{documents,locks,backlog,rate_limits,snapshots}
        -> SQLite (WAL)
```

Durability is introduced as storage-first primitives with engine-level orchestration. CLI remains a thin contract layer.

---

## 1) Schema additions and migration strategy

## Migration `003_durable_ingest.sql`

Add ingestion lock + backlog tables.

### `durable_locks`
- `lock_name TEXT PRIMARY KEY`
- `owner_id TEXT NOT NULL`
- `acquired_at TEXT NOT NULL`
- `expires_at TEXT` (nullable lease for stale-lock takeover)
- `updated_at TEXT NOT NULL`

Indexes:
- PK on `lock_name` is enough for point lookups.

### `ingest_backlog`
- `queue_id TEXT PRIMARY KEY`
- `idempotency_key TEXT NOT NULL UNIQUE`
- `source_path TEXT NOT NULL`
- `display_name_override TEXT`
- `status TEXT NOT NULL` (`queued|claimed|done|failed`)
- `claimed_by TEXT`
- `claimed_at TEXT`
- `attempt_count INTEGER NOT NULL DEFAULT 0`
- `last_error_code TEXT`
- `last_error_message TEXT`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`

Indexes:
- `idx_ingest_backlog_status_created(status, created_at)` for FIFO claim.

## Migration `004_rate_limits.sql`

### `rate_limit_counters`
- `key TEXT NOT NULL`
- `route TEXT NOT NULL` (`search|retrieve|context`)
- `window_start_epoch INTEGER NOT NULL`
- `request_count INTEGER NOT NULL`
- `updated_at TEXT NOT NULL`
- `PRIMARY KEY (key, route, window_start_epoch)`

Indexes:
- PK supports atomic upsert and lookup.

## Migration `005_snapshots.sql`

### `corpus_snapshots`
- `snapshot_id TEXT PRIMARY KEY`
- `state TEXT NOT NULL` (`building|active|superseded|failed`)
- `created_at TEXT NOT NULL`
- `activated_at TEXT`
- `superseded_at TEXT`
- `error_code TEXT`
- `error_message TEXT`

### `snapshot_members`
- `snapshot_id TEXT NOT NULL`
- `document_id TEXT NOT NULL`
- `PRIMARY KEY (snapshot_id, document_id)`

### `snapshot_pointer`
- single-row pointer table (`id INTEGER PRIMARY KEY CHECK (id=1)`, `active_snapshot_id TEXT NOT NULL`)

Atomic swap = single transaction updating `snapshot_pointer` + snapshot states.

## Migration ordering

- Extend `crates/storage/src/migrations/mod.rs` with versions 3, 4, 5.
- Migrations are additive and backward compatible with existing Phase 4 tables.

---

## 2) Storage API design

Add new storage modules:
- `storage/src/locks.rs`
- `storage/src/backlog.rs`
- `storage/src/rate_limits.rs`
- `storage/src/snapshots.rs`

### Lock APIs
- `try_acquire_lock(lock_name, owner_id, now, lease_secs) -> Result<AcquireLockResult>`
- `release_lock(lock_name, owner_id) -> Result<()>`

`try_acquire_lock` uses transactional insert/update-if-expired semantics.

### Backlog APIs
- `upsert_backlog_item(input) -> Result<BacklogItem>`
- `list_backlog(status_filter) -> Result<Vec<BacklogItem>>`
- `claim_next_backlog_item(claimed_by, now) -> Result<Option<BacklogItem>>`
- `mark_backlog_done(queue_id)` / `mark_backlog_failed(queue_id, error)`

`idempotency_key` derived from normalized source path + display override.

### Rate-limit APIs
- `check_and_increment_rate_limit(route, key, now, max, window_secs) -> Result<RateLimitDecision>`

Decision:
- `Allowed { remaining }`
- `Blocked { retry_after_seconds }`

Atomic behavior via transaction + upsert.

### Snapshot APIs
- `begin_snapshot_build(snapshot_id)`
- `attach_document_to_snapshot(snapshot_id, document_id)`
- `activate_snapshot(snapshot_id)` (atomic swap)
- `get_active_snapshot_id()`

### Recovery helper APIs
- `list_processing_documents()`
- `mark_processing_as_failed(document_id, error_info)`
- optional: `requeue_document(document_id)` if policy uses queue recovery.

---

## 3) Engine orchestration flow

## 3.1 Ingest flow (`engine::ingest`)

Split current `ingest(...)` into:
- `enqueue_ingest(...)`
- `ingest_next_queued(...)`
- `ingest_with_lock(...)` (used by direct ingest and queue worker)

`ingest_with_lock`:
1. `try_acquire_lock("ingest_pipeline", owner_id, ...)`
2. if acquired: run existing pipeline (`validate -> create doc -> processing -> extract/chunk/embed -> ready|failed`)
3. release lock in `finally` path
4. if lock not acquired:
   - `upsert_backlog_item(...)`
   - return `HarnessError::OperationInProgress { retry_after_seconds, lock_name: Some("ingest_pipeline") }`

## 3.2 Rate-limited retrieval/context

Add shared guard function (new `engine::durability` helper):
- `enforce_rate_limit(route, key, db, config.rate_limit)`

Call guard at entry of:
- `retrieve::search`
- `retrieve::retrieve`
- `context::build_context`

If blocked, return `HarnessError::RateLimitExceeded { retry_after_seconds }` before embedding/ranking.

Keying decision for MVP:
- `key = runtime_mode + ":" + provider_id` (stable across restarts and deterministic in single-user CLI mode).

## 3.3 Retry alignment

Keep manual retry contract:
- only `failed` docs
- file must exist
- cleanup partial data
- set `pending`
- `retry_count` reset to `0`

Fix code comment drift to match implemented behavior.

## 3.4 Startup recovery

Introduce `engine::recovery::recover_interrupted_processing(db)`:
- find all `documents.status = processing`
- mark each as `failed` with durable reason code (`interrupted_processing_recovered`)
- idempotent: rerun does not re-mutate already recovered docs

Run recovery from CLI command entry after DB open (shared helper used by all command handlers).

## 3.5 Refresh orchestration

New `engine::refresh::refresh_corpus(...)`:
1. create `building` snapshot
2. ingest/attach candidate documents into staging snapshot
3. validate build completeness
4. transactionally call `activate_snapshot(snapshot_id)`
5. on failure, mark snapshot `failed`, keep prior pointer intact

Retrieval/context read paths resolve through active snapshot membership only.

---

## 4) CLI command changes

### `harness ingest`
- Extend args:
  - positional `path` remains for direct mode
  - `--queued <path>` enqueue-only mode
  - `--next` process next queued
- Mutual exclusion validation in clap group.

### `harness refresh`
- New command module `crates/cli/src/commands/refresh.rs`
- Add subcommand wiring in:
  - `crates/cli/src/main.rs`
  - `crates/cli/src/commands/mod.rs`

### Shared bootstrap
- Add command-local helper (`open_db_and_recover`) to avoid duplicated recovery invocation logic.

---

## 5) Idempotency and concurrency considerations

- Lock acquisition is transactional; only one owner may process ingest at a time.
- Backlog enqueue is idempotent via unique `idempotency_key`.
- `claim_next_backlog_item` uses atomic status transition (`queued -> claimed`) to prevent double-claim.
- Rate-limit check+increment is atomic per `(key, route, window)`.
- Snapshot activation is atomic; no mixed visibility states.

---

## 6) Failure recovery paths and invariants

## Invariants
1. At most one active ingest lock for `ingest_pipeline`.
2. A backlog item has exactly one status in `{queued, claimed, done, failed}`.
3. `claimed` backlog items always have `claimed_by` and `claimed_at`.
4. Exactly one active snapshot pointer row.
5. Retrieval/context only read active snapshot documents.

## Recovery paths
- Crash during ingest lock ownership: lease expiry allows future takeover; stale backlog stays queued.
- Crash after backlog claim: claimed item can be reaped back to queued if claim timeout exceeded.
- Crash during refresh: active snapshot pointer remains old snapshot unless activation transaction commits.
- Startup recovery converts orphan `processing` docs into deterministic failed state.

---

## 7) Performance notes and tradeoffs

- Additional writes per ingest conflict (lock attempt + backlog upsert) are small versus embedding latency.
- Durable rate-limit adds one transactional read/write per guarded command; acceptable for MVP CLI throughput.
- Snapshot indirection adds joins/filtering on retrieval paths; mitigated with snapshot membership indexes.
- Chosen deterministic rate-limit key (`runtime_mode:provider_id`) is simple but coarse; future multi-user mode can extend to user/API principal keys.
- Marking interrupted `processing` as failed favors safety/auditability over automatic hidden reprocessing.

---

## 8) Slice alignment (review budget)

- Slice 1: migration 003 + lock/backlog storage + ingest conflict path
- Slice 2: queue CLI modes + claim/process
- Slice 3: migration 004 + rate-limit guard wiring
- Slice 4: retry alignment + startup recovery
- Slice 5: migration 005 + refresh + snapshot-gated reads
