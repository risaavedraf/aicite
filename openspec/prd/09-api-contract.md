# API Contract

The MVP is CLI-first. The primary contract is a set of stable CLI commands with JSON output for agents/tools and human-readable output for operators. A native app, MCP wrapper, hosted API, or built-in answer adapter can be added later, but each must preserve the same retrieval/context schemas.

## Base assumptions

| Area | Decision |
|---|---|
| Protocol | CLI commands; JSON mode is the machine contract; adapters are post-MVP |
| Request/response format | Rust-serializable structs, JSON-compatible output, stable schema |
| Error format | Machine-readable error enum with human-readable message |
| Versioning | Command and schema versions tracked through CLI semver and migration versioning |
| Runtime mode | Commands receive CLI runtime config for `public_packaged_demo`, `local_private_demo`, or `production` and enforce mode restrictions engine-side |
| Process model | Single-shot durable: each command starts, reads local state/config, persists updates, releases locks, and exits; no daemon is required in MVP |
| State identity | Durable request, trace, document, backlog, lock, and rate-limit records live under configured local data/state paths, not in process memory or disposable cache only |
| Path trust | The CLI can accept a file path only when the path policy and runtime mode allow it; agents should use documented flags and must not bypass validation with raw paths alone |
| Corpus scope | The MVP does not mix corpora: `public_packaged_demo` uses sample documents only; `local_private_demo` uses imported documents only |
| Transport security | All external provider API calls must use HTTPS; non-TLS endpoints are rejected |
| Output modes | Default human-readable output plus `--json` for tool use |
| Traceability | Each retrieval/context pack can be resolved to a stable trace ID |

## Configuration contract

Configuration is part of the public contract because agents need reproducible automation.

| Layer | Examples | Rule |
|---|---|---|
| CLI flags | `--config`, `--data-dir`, `--cache-dir`, `--runtime-mode`, `--top-k`, `--json` | Highest precedence |
| Environment | `CITE_CONFIG`, `CITE_DATA_DIR`, `CITE_CACHE_DIR`, `CITE_RUNTIME_MODE`, `CITE_EMBEDDING_PROVIDER`, `CITE_EMBEDDING_MODEL`, provider secret vars | Overrides config files |
| Config file | `$XDG_CONFIG_HOME/cite/config.toml`, `%APPDATA%\\cite\\config.toml`, `~/Library/Application Support/cite/config.toml`, or `CITE_CONFIG` | Overrides defaults |
| Defaults | public packaged demo defaults or local/private dev defaults | Lowest precedence |

Required config topics:

- runtime mode;
- local data directory for documents, SQLite, minimal graph/source metadata, indexes, locks, backlog records, traces, and rate-limit state;
- cache directory for temporary extraction data and provider caches only;
- embedding provider/model and provider timeout;
- rate-limit window and max requests;
- optional import roots/workspace roots;
- redaction/debug level.

Diagnostics must redact secrets and must not print raw file paths unless debug mode explicitly allows them.

## Single-shot durable process model

The CLI must not rely on process-local sessions for correctness.

- Each command opens durable state and exits after bounded work.
- Ingestion and retry commands use durable locks and persisted job records.
- If the ingestion/refresh lock is held, `ingest <path>` atomically validates and upserts a durable same-path/same-corpus backlog/loadable-document record, then returns `operation_in_progress` with `retry_after_seconds` and the existing or new document/backlog ID.
- Durable backlog records describe loadable or pending documents, but they are processed only by explicit CLI invocations such as `ingest --next` or `ingest --queued`; no hidden daemon drains them.
- Polling `get` or `list` observes persisted state; it does not imply a daemon is running.
- Rate-limit counters are stored under the configured local data/state path and keyed exactly by `runtime_mode + corpus_id + provider_id + retrieval_scope`.
- A future daemon may wrap commands, but MVP behavior must remain correct without it.

## Health

### `cite health`

Returns runtime and local-state status.

```json
{
  "status": "ok",
  "version": "0.1.0",
  "schema_version": "context-v1",
  "runtime_mode": "local_private_demo",
  "data_dir_configured": true,
  "cache_dir_configured": true
}
```

## Document lifecycle enum

Documents use this canonical lifecycle:

| Status | Meaning |
|---|---|
| `pending` | Import accepted; ingestion has not started and can be processed by an explicit invocation such as `ingest --next` or `ingest --queued` |
| `processing` | Extraction, chunking, metadata building, or embedding is running in the current command invocation |
| `ready` | Document is indexed and available for retrieval/context commands |
| `failed` | Ingestion stopped after a non-retryable error or after the retry cap, with a human-readable reason |

## Commands

### `cite ingest`

Imports a document and processes it when the current runtime mode permits uploads. In `public_packaged_demo`, the command rejects uploads because the CLI uses preloaded sample documents. In `production`, the command rejects uploads until the compliance checklist is complete.

**Path validation rules:**

- The CLI must validate the path according to the configured policy before opening the file.
- A raw `file_path` string is not enough to prove trusted origin when the implementation uses a capability layer or scoped path policy.
- Symlinks must be resolved before validation; symlinked paths outside allowed roots are rejected.
- Network paths, device files, and paths with traversal sequences (`../`) are rejected.
- Accepted roots are the configured import roots, configured workspace roots, or OS/user document roots when explicitly enabled.
- Post-MVP or optional import sources may include stdin, manifests, or globs, but each must be explicitly documented before use by agents.

Request example:

```bash
cite ingest ./docs/handbook.pdf --display-name "handbook.pdf" --json
```

Response:

```json
{
  "document_id": "doc_123",
  "display_name": "handbook.pdf",
  "status": "ready",
  "ingestion_job_id": "job_123",
  "chunk_count": 42,
  "trace_id": "trace_ingest_123"
}
```

`display_name` is optional. When omitted, the system derives a sanitized display label from the selected file. When provided, it overrides the display name shown in `list`, `get`, `search`, `retrieve`, `context`, `read`, and `trace` output. Raw filenames are internal/debug metadata and must not be returned to normal consumers or logs.

**Backlog behavior:** `cite ingest <path>` validates the file, atomically upserts a durable backlog/loadable-document record for the same path/corpus, and attempts immediate processing when the ingestion lock is free. If the same path is already present in the active corpus, the command returns the existing document/backlog ID and must not create duplicate rows. `cite ingest <path> --queue-only --json` performs the same validation/upsert but leaves the record `pending` without processing it. `cite ingest --next --json` or `cite ingest --queued --json` processes pending backlog records in explicit command invocations. If another ingestion or refresh holds the lock, the upsert remains committed and the command returns `operation_in_progress` with exit code `6`, `retry_after_seconds`, and the document/backlog ID; no daemon or hidden async queue drains backlog records.

### `cite list`

Lists documents available in the active runtime corpus. Public packaged demo returns sample documents only; local/private demo returns imported documents only.

```json
{
  "documents": [
    {
      "document_id": "doc_123",
      "display_name": "handbook.pdf",
      "status": "ready",
      "chunk_count": 42,
      "retry_count": 0,
      "next_retry_at": null,
      "created_at": "2026-05-26T18:00:00Z"
    }
  ]
}
```

### `cite get`

Returns document metadata and ingestion state.

```json
{
  "document_id": "doc_123",
  "display_name": "handbook.pdf",
  "status": "ready",
  "chunk_count": 42,
  "retry_count": 0,
  "max_retry_count": 3,
  "next_retry_at": null,
  "error": null
}
```

## Ingestion retry and recovery

Automatic ingestion retries are bounded. MVP default is 3 total attempts per document with exponential backoff before re-processing (`next_retry_at`). On command startup, interrupted `processing` documents are recovered using the same retry policy: reset to `pending` only while attempts remain, otherwise mark `failed`. Each retry starts from a clean ingestion state; partial text, chunks, minimal metadata records, and embeddings from failed attempts must be rolled back or excluded from retrieval before retrying.

A failed document does not block later ingestion after it reaches terminal `failed`. The CLI exposes a manual `retry` command for `failed` documents; the retry requeues the document from a clean state and returns the updated document metadata.

### `cite retry`

Requeues a `failed` document for manual retry after the user fixes the file/provider issue. The retry clears or ignores partial data from previous failed attempts before reprocessing. It returns `invalid_parameter` if the document is not `failed`, `document_not_found` if the document does not exist, and `file_not_found` if the original local file is no longer accessible.

```bash
cite retry doc_123 --json
```

```json
{
  "document_id": "doc_123",
  "display_name": "handbook.pdf",
  "status": "pending",
  "retry_count": 0,
  "max_retry_count": 3,
  "next_retry_at": null
}
```

### `cite refresh`

Rebuilds index data and minimal source/section/chunk metadata for `ready` already-ingested sources. It does not import new files. It accepts one document ID or `--all`, acquires the ingestion/refresh lock, and returns `operation_in_progress` if another mutating command is active. Refresh uses atomic snapshot semantics: new chunks, index rows, embeddings, and metadata are built in staging, then swapped into the current ready snapshot only after the refresh succeeds.

```bash
cite refresh doc_123 --json
cite refresh --all --json
```

```json
{
  "status": "completed",
  "refreshed_document_ids": ["doc_123"],
  "rebuilt_chunk_count": 42,
  "trace_id": "trace_refresh_123",
  "errors": []
}
```

During refresh, `search`, `retrieve`, `context`, and `read` continue to resolve against the last ready snapshot. If no prior ready snapshot exists, reads and retrieval return `document_not_ready`. `refresh` accepts only `ready` documents: unknown IDs return `document_not_found`; `pending` or `processing` documents return `document_not_ready`; `failed` documents must use `retry` first and return `invalid_parameter` if passed to `refresh`; lock conflicts return `operation_in_progress` with `retry_after_seconds`. Provider/storage errors use the shared error format.

## Retrieval commands

Retrieval commands are stateless from the caller's perspective. Follow-up-style requests are sent as new retrieval queries, and persistent conversation memory is post-MVP. When no documents are `ready` in the active corpus, retrieval commands return `document_not_ready`. If some documents are `ready` and others are `pending` or `processing`, retrieval runs against the ready subset only and returns partial-corpus metadata.

### `cite search`

Returns a concise ranked overview for humans and agents.

```bash
cite search "refund policy" --top-k 5 --json
```

```json
{
  "result_kind": "context",
  "query_id": "qry_123",
  "trace_id": "trace_123",
  "results": [
    {
      "citation_id": "c1",
      "document_id": "doc_123",
      "display_name": "policy.md",
      "chunk_id": "chunk_009",
      "page": null,
      "snippet": "Refunds may be requested within 30 days if the item is unused...",
      "score": 0.84
    }
  ],
  "metadata": {
    "retrieved_chunks": 5,
    "evidence_floor": 0.50,
    "confidence_threshold": 0.70,
    "ranking_method": "vector_cosine_v1",
    "source_metadata_state": "minimal_hierarchy_v1",
    "corpus_index_state": "partial",
    "ready_document_count": 1,
    "excluded_non_ready_document_count": 2,
    "excluded_non_ready_document_ids": ["doc_456", "doc_789"],
    "latency_ms": 420
  }
}
```

### `cite retrieve`

Returns chunk-level retrieval records for agent/tool use.

```bash
cite retrieve "refund policy" --top-k 5 --json
```

```json
{
  "result_kind": "context",
  "query_id": "qry_123",
  "trace_id": "trace_123",
  "chunks": [
    {
      "citation_id": "c1",
      "document_id": "doc_123",
      "display_name": "policy.md",
      "chunk_id": "chunk_009",
      "node_id": "node_009",
      "page": null,
      "offset": { "start": 1200, "end": 1540 },
      "text": "Refunds may be requested within 30 days if the item is unused...",
      "score": 0.84,
      "source_path": ["doc_123", "section:return-policy", "chunk_009"]
    }
  ],
  "metadata": {
    "top_k": 5,
    "evidence_floor": 0.50,
    "confidence_threshold": 0.70,
    "ranking_method": "vector_cosine_v1",
    "embedding_model_registry_id": "embedding-configured-default",
    "provider": "openai-compatible",
    "corpus_index_state": "ready",
    "latency_ms": 520
  }
}
```

### `cite context`

Builds the MVP's primary agent-facing artifact: a cited context pack. This command does not call a built-in LLM answer generator.

```bash
cite context "What does the refund policy say?" --top-k 5 --json
```

```json
{
  "context_pack_id": "ctx_123",
  "result_kind": "context",
  "query_id": "qry_123",
  "trace_id": "trace_123",
  "instructions": "Use only the cited context for claims about the user's documents. If the context is insufficient, say the documents do not contain enough information. Cite citation IDs for important claims.",
  "citations": [
    {
      "citation_id": "c1",
      "document_id": "doc_123",
      "display_name": "policy.md",
      "chunk_id": "chunk_009",
      "page": null,
      "text": "Refunds may be requested within 30 days if the item is unused...",
      "score": 0.84
    }
  ],
  "metadata": {
    "schema_version": "context-v1",
    "created_at": "2026-05-26T18:00:00Z",
    "retrieved_chunks": 5,
    "evidence_floor": 0.50,
    "confidence_threshold": 0.70,
    "ranking_method": "vector_cosine_v1",
    "top_score": 0.84,
    "corpus_index_state": "ready",
    "ready_document_count": 1,
    "excluded_non_ready_document_count": 0,
    "latency_ms": 530,
    "disclaimer": "Verify downstream AI answers against the cited sources before acting on them."
  }
}
```

`result_kind` follows the canonical threshold table:

| Retrieval state | Result kind | Citation rule |
|---|---|---|
| No candidate reaches `evidence_floor` | `no_results` | `citations: []` |
| At least one candidate reaches `evidence_floor` but top evidence is below `confidence_threshold` | `insufficient_context` | Low-confidence citations allowed and clearly marked |
| Top evidence reaches `confidence_threshold` but required facets are only partially covered | `insufficient_context` | Partial citations allowed and clearly marked |
| Evidence reaches `confidence_threshold` with enough coverage for the query | `context` | At least one supporting citation required |

`evidence_floor` and `confidence_threshold` are config/model calibration values and must be reported in retrieval metadata when available. `no_results` always has empty citations; `insufficient_context` may include citations, but they must be marked by low score/metadata and human-readable caution.

### `cite read`

Reads a citation or chunk so an agent can inspect source text without re-running retrieval.

Selectors are mutually exclusive:

- `--citation-id <id> --trace-id <id>` reads a citation from a known retrieval/context trace. `--trace-id` scopes short citation IDs such as `c1`.
- `--chunk-id <id> --document-id <id>` reads a chunk from a known document. `--document-id` scopes chunk IDs when they are not globally unique.

Providing both `--citation-id` and `--chunk-id`, omitting the required scope, or matching multiple records returns `invalid_parameter`. Missing citation records return `citation_not_found`; missing documents return `document_not_found`; missing chunk IDs in an existing document return `chunk_not_found`. `read --chunk-id` resolves only against the current ready snapshot for that document. Stale chunk IDs from superseded refresh snapshots, chunks from failed attempts, or chunks belonging to non-ready documents return `chunk_not_found` or `document_not_ready` as applicable; implementations must not expose partial/staging chunks.

```bash
cite read --citation-id c1 --trace-id trace_123 --json
cite read --chunk-id chunk_009 --document-id doc_123 --json
```

```json
{
  "citation_id": "c1",
  "document_id": "doc_123",
  "display_name": "policy.md",
  "chunk_id": "chunk_009",
  "page": null,
  "offset": { "start": 1200, "end": 1540 },
  "text": "Refunds may be requested within 30 days if the item is unused...",
  "trace_id": "trace_123"
}
```

### `cite trace`

Returns the trace for a completed retrieval/context request.

```bash
cite trace trace_123 --json
```

```json
{
  "trace_id": "trace_123",
  "query_id": "qry_123",
  "context_pack_id": "ctx_123",
  "timestamp": "2026-05-26T18:00:00Z",
  "schema_version": "context-v1",
  "embedding_model_registry_id": "embedding-configured-default",
  "provider": "openai-compatible",
  "document_ids": ["doc_123"],
  "citation_ids": ["c1"],
  "retrieval_top_k": 5,
  "evidence_floor": 0.50,
  "confidence_threshold": 0.70,
  "ranking_method": "vector_cosine_v1",
  "source_metadata_state": "minimal_hierarchy_v1",
  "responsible_owner": null,
  "user_visible_disclaimer_shown": true
}
```

`responsible_owner` is required in production/team mode. In local/private mode it may be omitted or `null`; clients must not invent fake owners to satisfy local traces.

## Filename handling

`display_name` is the only normal user-facing source label in command responses, citations, CLI panels, and error messages. Raw original filenames may be stored internally for file access/debugging, but must be hidden from normal consumers and excluded from logs. Filenames containing personal data should be sanitized to a generic display name in production contexts.

## Error/log redaction

Error `details` may include safe machine fields such as `document_id`, `error_code`, `retry_after_seconds`, or sanitized `display_name`. They must not include raw file paths, raw filenames, full provider payloads, query text, prompts, or document/citation text. Logs must use the stricter log-safe allowlist in the non-functional requirements.

## Error format

All errors should follow this shape:

```json
{
  "error": {
    "code": "unsupported_file_type",
    "message": "Only PDF, TXT, and MD files are supported.",
    "details": {
      "display_name": "Unsupported spreadsheet file"
    }
  }
}
```

## Initial error codes

| Code | Meaning |
|---|---|
| `unsupported_file_type` | File extension is not supported |
| `file_too_large` | File exceeds configured max size |
| `file_not_found` | Selected file path does not exist or is inaccessible |
| `document_not_found` | Requested document does not exist |
| `document_not_ready` | Retrieval requested when no documents are `ready` in the active corpus |
| `trace_not_found` | Requested trace record does not exist |
| `citation_not_found` | Requested citation record does not exist |
| `chunk_not_found` | Requested chunk does not exist in the current ready snapshot for the scoped document |
| `embedding_provider_error` | Embedding provider failed |
| `runtime_mode_forbidden` | Command is not allowed in the current runtime mode, such as uploads in public packaged demo or pre-compliance production |
| `internal_error` | Unexpected engine or CLI error |
| `query_too_long` | Retrieval query exceeds maximum length |
| `invalid_parameter` | A request parameter is outside its valid range |
| `path_rejected` | File path policy failed validation |
| `retrieval_timeout` | Retrieval or embedding provider call exceeded the configured timeout |
| `rate_limit_exceeded` | Too many retrieval/context requests in the configured time window |
| `operation_in_progress` | A durable lock is held by another command invocation |
| `config_error` | Config is missing, invalid, or unsafe |
| `storage_error` | Local data, index, cache, or migration state is unavailable or unsafe |

## Exit codes

| Exit code | Meaning |
|---|---|
| `0` | Success, including `context`, `no_results`, and `insufficient_context` responses |
| `1` | Validation, config, or contract error |
| `2` | Not found or not ready |
| `3` | Provider or external dependency failure |
| `4` | Runtime mode forbidden |
| `5` | Internal error |
| `6` | Operation in progress / durable lock conflict |
| `7` | Rate limit exceeded |

### Error-to-exit-code mapping

| Error code | Exit code |
|---|---:|
| `unsupported_file_type` | `1` |
| `file_too_large` | `1` |
| `file_not_found` | `2` |
| `document_not_found` | `2` |
| `document_not_ready` | `2` |
| `trace_not_found` | `2` |
| `citation_not_found` | `2` |
| `chunk_not_found` | `2` |
| `embedding_provider_error` | `3` |
| `runtime_mode_forbidden` | `4` |
| `internal_error` | `5` |
| `query_too_long` | `1` |
| `invalid_parameter` | `1` |
| `path_rejected` | `1` |
| `retrieval_timeout` | `3` |
| `rate_limit_exceeded` | `7` |
| `operation_in_progress` | `6` |
| `config_error` | `1` |
| `storage_error` | `1` |

## Lock-conflict behavior

When `operation_in_progress` is returned for an ingestion/refresh lock conflict, the error `details` object must include `retry_after_seconds` (integer), `lock_name`, and, when a backlog/document upsert occurred, the `document_id` or `backlog_id`. The client may retry after that delay or explicitly process backlog later with `ingest --next` / `ingest --queued`.

```json
{
  "error": {
    "code": "operation_in_progress",
    "message": "Another ingestion or refresh command is active. The document was queued for explicit processing.",
    "details": {
      "retry_after_seconds": 10,
      "lock_name": "ingestion",
      "document_id": "doc_123",
      "backlog_id": "backlog_123"
    }
  }
}
```

## Rate limit behavior

Retrieval/context commands must enforce the FR-109 configurable durable rate limit. The canonical durable rate-limit key is exactly `runtime_mode + corpus_id + provider_id + retrieval_scope`. When `rate_limit_exceeded` is returned, the error `details` object includes `retry_after_seconds` (integer), and the client must wait at least that many seconds before retrying. Default rate limit: 20 retrieval/context requests per minute per canonical key. The counter is stored in durable local state/data, not disposable cache, and must not reset merely because a CLI process exits or is relaunched.

```json
{
  "error": {
    "code": "rate_limit_exceeded",
    "message": "Rate limit exceeded. Try again in a few seconds.",
    "details": {
      "retry_after_seconds": 12
    }
  }
}
```

## Related docs

- [Functional Requirements](./04-functional-requirements.md)
- [System Architecture](./07-system-architecture.md)
- [Acceptance Criteria](./10-acceptance-criteria.md)
- [Legal and Privacy Compliance](./12-legal-privacy-compliance.md)
- [AI Ethics and Governance](./13-ai-ethics-governance.md)
