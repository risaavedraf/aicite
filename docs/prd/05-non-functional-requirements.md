# Non-Functional Requirements

The MVP should feel production-ready by being reliable, observable, secure enough for a demo, and easy for agents to run from the shell.

## Quality targets

| Area | Requirement |
|---|---|
| Reliability | Core ingest/retrieve/read flow works repeatedly across separate CLI invocations without manual resets. |
| Transparency | Context packs expose citations, scores, trace IDs, and no-results reasons. |
| Reproducibility | The same corpus, configuration, and index should produce comparable retrieval outputs and traces across runs. |
| Maintainability | Code should be modular: ingestion, retrieval, minimal source metadata, provider integration, storage, CLI, config, and trace layers are separable. |
| Deployability | The engine can be packaged and run locally from documented commands. |
| Testability | Critical logic and command surfaces have automated tests. |
| Compliance readiness | Privacy, legal, and AI governance decisions are documented before production use. |
| CLI ergonomics | Commands have predictable names, help text, exit codes, output modes, and durable state behavior. |

## Performance

| Metric | MVP target |
|---|---|
| Import response | CLI validates a file and starts ingestion work within 2 seconds for typical docs. |
| Ingestion time | Small documents under 5 MB process within 60 seconds when provider latency is normal. |
| Retrieval latency | `search`/`retrieve`/`context` return within 5 seconds for an indexed small corpus under normal provider/index latency. |
| Retrieval size | Default top-k retrieval: 5 chunks (valid range: 1–10, configurable by command flag/config). |
| File size | MVP supports files up to 10 MB by default. |
| Command startup | Common CLI commands should start quickly enough for agent use; avoid unnecessary boot work. |

## Configuration contract

Configuration must be deterministic and safe for automation.

| Area | Requirement |
|---|---|
| Precedence | CLI flags > environment variables > config file > runtime defaults. |
| Config file | Support `HARNESS_CONFIG` override; otherwise use OS-appropriate config paths such as `$XDG_CONFIG_HOME/harness/config.toml`, `%APPDATA%\\harness\\config.toml`, or `~/Library/Application Support/harness/config.toml`. |
| Data directory | Support `HARNESS_DATA_DIR`; otherwise use OS-appropriate local app data for documents, SQLite, minimal graph/source metadata, indexes, durable locks, backlog records, traces, and rate-limit state. |
| Cache directory | Support `HARNESS_CACHE_DIR`; otherwise use OS-appropriate cache paths for temporary extraction data and provider caches only. Rate-limit state is durable local state, not disposable cache. |
| Runtime mode | `HARNESS_RUNTIME_MODE` or config selects `public_packaged_demo`, `local_private_demo`, or `production`. |
| Provider configuration | Embedding provider/model IDs and API keys use documented keys such as `HARNESS_EMBEDDING_PROVIDER`, `HARNESS_EMBEDDING_MODEL`, and provider-specific secret variables. |
| Agent-safe overrides | Flags can override config path, data/cache directories, runtime mode, corpus ID/path, `top_k`, JSON output, and tracing without interactive prompts. |
| Secret handling | Secrets are never logged, echoed in errors, written into traces, or embedded in packaged demos. Diagnostics show redacted placeholders only. |

## Security and privacy

| Requirement | Notes |
|---|---|
| No secrets in repo | API keys must be provided via environment variables or user-supplied local configuration. Public packaged builds must not embed real provider keys; they must use mock/local/offline providers or require user configuration outside the package. |
| File validation | Reject unsupported extensions and oversized files. |
| Safe storage | Uploaded files, extracted chunks, minimal graph/source metadata, indexes, embeddings, locks, backlog records, and rate-limit state should be stored in CLI-managed local directories or database tables, with documented local reset/delete steps. Local/private MVP does not promise encryption at rest beyond the operator's OS/filesystem controls. |
| Prompt/data boundary | Documents are treated as data for retrieval. If a future answer adapter is added, prompts must instruct the model to use retrieved context only and ignore document instructions. |
| Demo data warning | README and CLI output must state that public packaged uploads are disabled and local/private imports must not use personal, sensitive, or confidential documents unless the operator controls the environment. |
| Chile privacy baseline | Product docs must account for Ley 19.628 and Ley 21.719 at a product/engineering level. |
| Provider disclosure | CLI output and README must disclose when document snippets or embeddings may be sent to configured AI providers. |
| Data minimization | Retrieval/context flows must return and, when applicable, send only the chunks needed for the requested context, not full documents. |
| HTTPS required | All external API calls to AI or embedding providers must use HTTPS; reject non-TLS endpoints. |
| Durable rate limiting | Retrieval/context commands must enforce the FR-109 configurable rate limit with durable local state/data keyed exactly by `runtime_mode + corpus_id + provider_id + retrieval_scope`, return `rate_limit_exceeded` with `retry_after_seconds`, and not reset merely because a CLI process restarts. |
| Output redaction | Logs and traces must avoid raw filenames/display names, query text, citation text, chunk text, secrets, and provider payloads unless explicitly allowed by the contract. |

## Observability

The system should log:

- Import accepted/rejected.
- Ingestion moved through `pending`, `processing`, `ready`, or `failed` states.
- Durable lock acquisition/release and explicit backlog/manual-processing decisions.
- Chunk count per document.
- Embedding provider/model registry IDs used.
- Retrieval count, chunk count, top score, and ranking method.
- Citation IDs returned to the user or agent.
- Context-pack trace IDs.
- Configured responsible owner/team ID in production mode.
- Retrieval/context latency.
- Rate-limit decisions.
- Error type and request ID.

Logs should be structured enough to debug production behavior without storing full document text, full prompts, secrets, or raw personal data.

**Log-safe fields (allowlist):** request_id, trace_id, document_id, chunk_id, citation_id, model_registry_id, provider_id, responsible_owner_id, latency_ms, error_code, ingestion_status, retrieval_count, chunk_count, top_score, ranking_method, runtime_mode, rate_limit_key_hash.

**Prohibited log fields:** full document text, full prompts, secrets, API keys, raw filenames or display names (use document_id instead), user query text, citation text content, chunk text content, raw personal data, and unredacted provider error payloads.

## Reliability behavior

| Scenario | Expected behavior |
|---|---|
| Embedding provider fails | Mark ingestion or retrieval as failed with a clear provider error; do not crash the engine. |
| Local index/state/cache unavailable | Return a clear storage error with reset guidance; do not silently rebuild destructive state. |
| No relevant chunks found | Return `no_results` with empty citations and a next action. |
| Empty document | Reject or mark failed with clear reason. |
| Unsupported file | Reject before ingestion. |
| Trace lookup fails | Keep source/citation output usable and surface a machine-readable trace error separately. |
| Concurrent mutation | Use durable locks. If the ingestion/refresh lock is held, `ingest <path>` atomically upserts the same-path/same-corpus backlog record, returns `operation_in_progress` with `retry_after_seconds` and a non-zero exit code, and does not start hidden work. Durable backlog entries are processed only by explicit CLI commands, not by hidden queue draining. |

## CLI usability

- The CLI must provide help text for all supported commands.
- Errors must be human-readable on stderr and machine-readable in JSON mode.
- The CLI must distinguish stdout content from stderr diagnostics.
- Common commands should have stable names and flags so agents can rely on them.
- Commands must document whether they mutate local state, read local state, or perform provider calls.

## Related docs

- [System Architecture](./07-system-architecture.md)
- [AI and Retrieval Design](./08-ai-retrieval-design.md)
- [Acceptance Criteria](./10-acceptance-criteria.md)
- [Legal and Privacy Compliance](./12-legal-privacy-compliance.md)
- [AI Ethics and Governance](./13-ai-ethics-governance.md)
