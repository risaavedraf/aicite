# SDD Spec — Phase 5: Durability

## 1) Scope and compatibility

Phase 5 adds durability behavior on top of completed Phases 2–4 without changing existing successful output contracts for `search`, `retrieve`, `context`, `read`, or `trace`.

Durability scope:
- durable ingest lock + conflict contract
- durable ingest backlog and queue operations
- durable rate limits for retrieval/context entrypoints
- retry contract alignment
- refresh contract with atomic snapshot swap
- startup recovery for interrupted `processing` documents

## 2) Storage contracts

## 2.1 Durable ingest lock

Storage MUST provide a durable lock record for ingestion serialization.

Required lock behavior:
- lock is uniquely identified by `lock_name` (ingest lock name MUST be `ingest_pipeline`)
- lock acquisition is atomic
- lock release is atomic
- stale/expired lock handling (if configured) MUST be deterministic and testable
- lock state MUST survive CLI process restart

## 2.2 Backlog queue

Storage MUST provide a durable backlog for pending ingest work.

Each backlog item MUST include at least:
- queue identity (`queue_id`)
- source path
- display-name override (nullable)
- enqueue/update timestamps
- status (`queued`, `claimed`, `done`, `failed`)
- optional failure metadata

Upsert semantics:
- enqueue on lock conflict MUST be idempotent for equivalent ingest intent
- repeated enqueue of the same source MUST update existing queued item rather than create unbounded duplicates

Claim semantics:
- next-item claim MUST be atomic
- only one worker invocation may claim a queued item at a time

## 2.3 Durable rate-limit counters

Storage MUST persist rate-limit counters that survive process/database reopen.

Rate-limit scope:
- applies to `search`, `retrieve`, and `context`
- default policy: `20 requests / 60 seconds / key`
- keying MUST be deterministic for the same caller scope across invocations
- check+increment MUST be atomic

## 2.4 Refresh snapshot state

Storage MUST support refresh snapshot promotion with atomic active-snapshot swap.

Visibility rule:
- readers MUST observe either the pre-refresh active snapshot OR the post-refresh active snapshot, never a mixed state

## 3) CLI contracts

## 3.1 `harness ingest <path>`

Behavior:
- tries to acquire ingest lock before processing
- if lock acquired: runs normal ingest pipeline
- if lock not acquired: MUST upsert backlog item and return `operation_in_progress`

Conflict error contract (`operation_in_progress`):
- machine code: `operation_in_progress`
- exit code: `6`
- JSON details MUST include:
  - `retry_after_seconds` (u32 > 0)
  - `lock_name` (for ingest: `ingest_pipeline`)

## 3.2 `harness ingest --queued <path>`

Behavior:
- MUST enqueue/upsert ingest work without immediate processing
- MUST return success when enqueue/upsert succeeds
- MUST be idempotent for equivalent ingest intent

## 3.3 `harness ingest --next`

Behavior:
- MUST claim exactly one next queued item atomically
- MUST attempt processing under ingest lock
- if queue is empty: MUST return success with explicit empty-queue result
- if lock is held by another process: MUST return `operation_in_progress` with retry metadata

## 3.4 `harness retry <document_id>`

Behavior:
- only valid for documents in `failed`
- MUST verify source file exists
- MUST clear stale partial ingest artifacts before retry scheduling
- MUST reset document status to `pending`
- retry count semantics MUST be explicit and consistent between code, comments, and output contract

Failure mapping remains:
- missing doc => `document_not_found`
- not failed => `invalid_parameter`
- missing source file => `file_not_found`

## 3.5 `harness refresh`

Behavior:
- MUST execute refresh as staged build + atomic snapshot promotion
- MUST not expose mixed snapshot reads
- MUST return refresh status including success/failure and active snapshot identity/version metadata

## 4) Engine contracts

Engine MUST enforce:
- ingest serialization through durable lock APIs
- backlog upsert on ingest lock conflict
- deterministic queue transitions (`queued -> claimed -> done|failed`)
- durable rate-limit checks before retrieval/context execution
- startup recovery routine before command execution path (or equivalent shared bootstrap)

Startup recovery contract:
- engine MUST scan for interrupted `processing` documents from prior aborted runs
- each interrupted item MUST transition via deterministic policy to a recoverable state (`failed` with reason and/or re-queued)
- recovery action MUST be idempotent across repeated startups

## 5) Error and exit-code contracts

Durability-critical mappings:
- `operation_in_progress` => exit `6`
- `rate_limit_exceeded` => exit `7`

`rate_limit_exceeded` payload MUST include `retry_after_seconds`.

Error responses MAY include safe machine details (IDs, lock names, retry-after), but MUST NOT include unsafe sensitive fields (see Safety section).

## 6) Rate-limit policy contract

Default policy values:
- `max_requests = 20`
- `window_seconds = 60`

Applies to command entrypoints:
- `harness search`
- `harness retrieve`
- `harness context`

When limit is exceeded:
- command MUST not execute retrieval/context computation
- MUST return `rate_limit_exceeded` with deterministic retry-after estimate

## 7) Safety and redaction constraints

Durability-related errors/logs MUST NOT expose:
- raw absolute file paths
- raw provider payloads
- full query text
- full chunk/source text
- secrets/tokens/credentials

Allowed safe fields include:
- document/trace/context IDs
- lock name
- retry-after seconds
- queue state labels
- sanitized error class/code

## 8) Non-goals

This phase does NOT include:
- retrieval quality/ranking upgrades
- golden dataset/evaluation harness (Phase 6)
- packaging/release flows (Phase 7)
- CLI binary rename
- answer generation over retrieved context

## 9) Required tests

## 9.1 Lock + backlog
- lock acquisition/release success path
- concurrent ingest conflict returns `operation_in_progress` with `retry_after_seconds` + `lock_name`
- lock conflict persists backlog upsert
- duplicate enqueue is idempotent (no unbounded duplicates)

## 9.2 Queue processing
- `ingest --queued` enqueues/upserts without processing
- `ingest --next` claims exactly one item atomically
- empty queue behavior is explicit and stable
- claim conflict behavior under concurrency

## 9.3 Rate limiting
- search/retrieve/context are each guarded by durable rate limit checks
- 20 requests/min policy enforcement per key
- retry-after computation correctness
- counters persist across DB reopen
- window rollover resets allowance correctly

## 9.4 Retry + recovery
- retry allowed only for failed docs
- retry contract fields/semantics align with implementation comments
- startup recovery reclassifies interrupted `processing` docs deterministically
- startup recovery is idempotent

## 9.5 Refresh atomicity
- refresh performs atomic snapshot promotion
- readers never observe mixed pre/post-refresh data
- refresh failure leaves prior active snapshot intact

## 9.6 Regression guard
- existing Phase 2–4 happy paths remain valid when no durability conflict/limit/recovery condition is triggered
