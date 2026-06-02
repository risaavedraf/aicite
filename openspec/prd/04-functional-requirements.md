# Functional Requirements

These requirements define what the MVP must do from the perspective of the CLI, the engine, and the machine contract.

## Requirement levels

| Level | Meaning |
|---|---|
| Must | Required for MVP |
| Should | Strongly recommended if it does not delay MVP |
| Could | Post-MVP candidate |

## Corpus ingestion

| ID | Level | Requirement |
|---|---|---|
| FR-001 | Must | The CLI can ingest `.pdf`, `.txt`, and `.md` documents in local/private mode; the public packaged demo uses sample documents only. |
| FR-002 | Must | The engine validates file type and returns a clear error for unsupported files. |
| FR-003 | Must | The engine extracts text from ingested or sample files. |
| FR-004 | Must | The engine chunks extracted text and builds a semantic structure for retrieval and navigation. |
| FR-005 | Must | The engine creates embeddings for chunks and stores them in a local index. |
| FR-006 | Must | Users and agents can see whether a document is `pending`, `processing`, `ready`, or `failed`. |
| FR-007 | Should | Failed ingestion includes a human-readable reason and a machine-readable error code. |
| FR-008 | Must | README documents local storage paths and manual reset/delete steps for imported documents, extracted text, chunks, embeddings, indexes, minimal graph/source metadata, durable locks, rate-limit state, and derived metadata. |
| FR-009 | Must | Logs use the safe-field allowlist from the non-functional requirements. Logs must not contain full document text, full prompts, secrets, API keys, raw filenames/display names, provider error payloads, user query text, citation text, chunk text, or raw personal data. |
| FR-010 | Must | Ingestion uses durable local locks plus explicit backlog/status records. A command that holds the ingestion lock processes one document at a time. If another ingestion/refresh holds the lock, `ingest <path>` atomically validates and upserts a durable backlog/loadable-document record for the same path/corpus, returns the existing or new document/backlog ID, and exits with `operation_in_progress` plus `retry_after_seconds`. Backlog records are processed only by explicit CLI invocations such as `ingest --next` or `ingest --queued`; no daemon or hidden async queue is required for MVP progress. |
| FR-011 | Must | Failed ingestion must roll back or mark partial data (extracted text, partial chunks, minimal metadata records) for cleanup so the local database remains consistent. Partial data from failed ingestion must not be returned by retrieval. |
| FR-012 | Should | When `display_name` is not provided during ingest, the system derives a sanitized display label from the selected file. User-facing command output uses `display_name`; raw filenames remain internal/debug metadata and must not be logged. In production mode, PII-obvious names default to a generic label like `document_<id>.<ext>`. |
| FR-013 | Must | On command startup, interrupted `processing` documents must be recovered with bounded retries: reset to `pending` only while retry attempts remain, apply backoff before re-processing, and mark the document `failed` after the configured cap. Documents stuck in `processing` from a previous invocation must not remain in zombie state or retry forever. |
| FR-014 | Must | Failed ingestion provides a user-visible recovery path to retry or reprocess the failed document after partial cleanup. |
| FR-015 | Must | The CLI provides `refresh` to rebuild index data and minimal source/section/chunk metadata for `ready` already-ingested sources using atomic snapshot semantics: reads keep using the last ready snapshot while staging rebuilds, then switch atomically when refresh completes. Pending/processing documents return `document_not_ready`; failed documents must be recovered with `retry` before refresh. |

## Retrieval and context packs

| ID | Level | Requirement |
|---|---|---|
| FR-101 | Must | Users and agents can submit a natural-language retrieval query over the active corpus through the CLI. Query length is limited to 2000 characters. |
| FR-102 | Must | The engine retrieves relevant chunks using vector-first ranking. MVP graph scope is minimal metadata for source, section, chunk, citation, and trace relationships; graph expansion/traversal ranking is post-MVP. |
| FR-103 | Must | The engine returns ranked chunks, citations, source metadata, and retrieval metadata without calling a built-in LLM answer generator. |
| FR-104 | Must | The `context` command emits an agent-consumable context pack that includes cited chunks, source labels, chunk IDs, scores, and trace IDs. |
| FR-105 | Must | If retrieved context is insufficient, the engine returns explicit `no_results` or `insufficient_context` output instead of inventing an answer. Below the configured `evidence_floor`, `no_results` requires empty citations. Between `evidence_floor` and `confidence_threshold`, or when coverage is partial, `insufficient_context` may include clearly marked low-confidence or partial citations/metadata. |
| FR-106 | Should | The engine exposes retrieval scores, ranking method, `evidence_floor`, `confidence_threshold`, and minimal source/section/chunk metadata for transparency. |
| FR-107 | Should | Follow-up-style interactions are treated as stateless new retrieval requests for MVP; persistent conversation memory is post-MVP. |
| FR-108 | Must | If some documents in the active corpus are `pending` or `processing`, retrieval uses the ready subset only and returns metadata so the CLI can warn that non-ready documents were excluded. If no documents are `ready`, retrieval commands return `document_not_ready`. |
| FR-109 | Must | Retrieval/context commands enforce a configurable durable rate limit to prevent runaway provider and compute costs. The MVP default is 20 retrieval/context requests per minute per `runtime_mode + corpus_id + provider_id + retrieval_scope` key; exceeded requests return `rate_limit_exceeded` with `retry_after_seconds`. The counter is stored in durable local state/data, not disposable cache, and must not reset merely because a CLI process exits. |

## Citation and verification

| ID | Level | Requirement |
|---|---|---|
| FR-201 | Must | Each returned citation includes a sanitized document display name, chunk ID, source snippet text, and page/offset metadata when available. |
| FR-202 | Must | The CLI lets the user or agent inspect citations through `context`, `read`, and `trace` output. |
| FR-203 | Should | Citations include page number for PDFs when available. |
| FR-204 | Could | The CLI can open or export the original uploaded document. |
| FR-205 | Must | The engine can expose a trace object for each retrieval/context-pack request, including trace ID, request ID, ranking metadata, citation IDs, provider metadata for embedding calls when relevant, and timing information. |

## Corpus management

| ID | Level | Requirement |
|---|---|---|
| FR-301 | Must | The MVP provides one default corpus. |
| FR-302 | Must | The CLI can list documents available in the active runtime corpus; public packaged demo returns sample documents only, and local/private demo returns imported documents only. |
| FR-303 | Could | The CLI can delete a document and remove its chunks from retrieval. |
| FR-304 | Could | The CLI can create multiple named corpora. |

## CLI, configuration, and developer requirements

| ID | Level | Requirement |
|---|---|---|
| FR-401 | Must | The CLI exposes a health/status command for local smoke tests. |
| FR-402 | Must | Command output has machine-readable error responses. |
| FR-403 | Must | The CLI supports both human-readable output and a `--json` mode for agent/tool use. |
| FR-404 | Should | The CLI logs retrieval latency, retrieval count, embedding provider/model when used, and errors. |
| FR-405 | Must | README includes local setup, env vars, config file paths, precedence rules, demo flow, runtime mode rules, storage/cache/index paths, durable state paths, and manual reset/delete steps. |
| FR-406 | Must | Configuration precedence is documented and deterministic: CLI flags override environment variables, environment variables override config files, and config files override runtime defaults. |
| FR-407 | Must | Provider secrets are passed through documented environment variables or local config, never committed, never printed, and redacted in diagnostics. |
| FR-408 | Should | Agent-safe overrides exist for runtime mode, corpus path, config path, cache/data directory, top-k, and output mode without requiring interactive prompts. |

## Related docs

- [UX Flows](./06-ux-flows.md)
- [API Contract](./09-api-contract.md)
- [Acceptance Criteria](./10-acceptance-criteria.md)
