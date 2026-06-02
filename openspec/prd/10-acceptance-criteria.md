# Acceptance Criteria

This checklist defines when the MVP is ready to be shown as a working CLI-first context cite for AI agents.

## Product acceptance

- [ ] [FR-301] User can open the CLI with one default corpus.
- [ ] [FR-001] Public packaged demo uses preloaded sample documents and disables uploads.
- [ ] [FR-001] Public packaged demo contains no embedded real provider API key; it uses mock/local/offline providers or requires user-supplied configuration outside the package.
- [ ] [FR-001, FR-302] Public packaged demo retrieves from sample documents only; local/private demo retrieves from imported documents only.
- [ ] [FR-001, FR-002] Local/private CLI demo can import a supported document after clear no-sensitive-data warnings.
- [ ] [FR-006] User can see ingestion status.
- [ ] [FR-101] User or agent can submit a retrieval query after ingestion completes.
- [ ] [FR-103, FR-104] User or agent receives a context pack with ranked chunks, citations, source metadata, and trace ID.
- [ ] [FR-201] User or agent receives citations with source snippets.
- [ ] [FR-202] User or agent can inspect cited snippets from `context`, `read`, or `trace` output.
- [ ] [FR-105] Unsupported or unknown requests return `no_results` or `insufficient_context` instead of fabricated answers.
- [ ] CLI output shows a disclaimer that downstream AI answers must be verified against cited sources.
- [ ] Production uploads remain blocked until auth, deletion, retention, privacy notice, and provider/legal review are complete.

## AI/retrieval acceptance

- [ ] [FR-102] Retrieval returns relevant chunks for known queries; relevant supporting chunk appears in top 5 for at least 80% of golden retrieval queries.
- [ ] [FR-102, FR-105] Configured `evidence_floor` and `confidence_threshold` default to calibrated values for the active embedding model/index; candidates below `evidence_floor` return `result_kind: 'no_results'` with no citations, while weak/partial evidence between thresholds returns `insufficient_context` with cautious metadata.
- [ ] [FR-103] MVP retrieval/context commands do not require a built-in answer-generation LLM.
- [ ] [FR-104] Context packs include agent instructions that require cited context and no unsupported claims.
- [ ] Embedding provider/model used for retrieval is visible in metadata or trace output when a provider call occurs.
- [ ] Real-provider runs use configurable provider/model IDs that satisfy the white-box registry capabilities; named commercial models are not required for MVP acceptance.
- [ ] Context trace records `trace_id`, schema version, provider/model registry ID when applicable, document IDs, citation IDs, retrieval top-k, ranking method, `user_visible_disclaimer_shown`, and timestamp visible to the CLI; `responsible_owner` is required only in production/team mode and may be omitted or `null` in local/private mode.
- [ ] Golden dataset includes at least 3 direct-fact cases, 2 no-results cases, 1 ambiguous query, 1 multi-chunk query, and 1 prompt-injection fixture.
- [ ] Direct-fact golden cases pass 3/3 with supporting chunks in top 5.
- [ ] No-results golden cases pass 2/2 without fabricated citations or unsupported source claims.
- [ ] Prompt-injection fixture confirms document instructions are treated as source text, not executable instructions.
- [ ] Ambiguous golden case returns `result_kind: 'insufficient_context'` or clearly cautious metadata, with no unsupported claims.
- [ ] Multi-chunk fixture includes at least two relevant citations.

## Runtime acceptance

- [ ] [FR-401] `health` command returns healthy status and local config/state summary.
- [ ] [FR-001] `ingest` supports file import in local/private mode.
- [ ] [FR-001, FR-402] `ingest` requires a valid file-selection capability or documented path policy and rejects raw path-only imports when policy requires a capability.
- [ ] [FR-302] `list` lists documents for the active runtime corpus; public packaged demo returns sample documents only, and local/private demo returns imported documents only.
- [ ] [FR-006] `get` returns ingestion state.
- [ ] [FR-010] Durable ingestion locks prevent unsafe concurrent mutation; `ingest <path>` under an active ingestion/refresh lock atomically upserts a same-path/same-corpus backlog record, returns `operation_in_progress` with `retry_after_seconds` plus the document/backlog ID, and pending/backlog documents are processed only by explicit CLI commands.
- [ ] [FR-011, FR-013] Failed or interrupted ingestion rolls back/excludes partial data, applies bounded retry/backoff, and marks the document `failed` after the retry cap instead of retrying forever.
- [ ] [FR-014] `retry` requeues a failed document from a clean state and returns updated document metadata.
- [ ] [FR-015] `refresh` rebuilds index data and minimal source/section/chunk metadata only for ready already-ingested sources, uses staging plus atomic snapshot swap, keeps reads on the last ready snapshot during refresh, and returns documented state-specific errors.
- [ ] `search` returns ranked snippets, citation IDs, scores, and trace ID.
- [ ] `retrieve` returns chunk-level records for agent/tool use.
- [ ] `context` returns `context_pack_id`, `result_kind`, citations, instructions, metadata, and trace ID.
- [ ] `read` returns source text for either a scoped citation ID or a scoped chunk ID from the current ready snapshot, rejects ambiguous or mutually conflicting selectors, and returns `chunk_not_found` for stale/missing chunks.
- [ ] `trace` returns retrieval/context trace metadata including citation IDs and ranking metadata.
- [ ] Retrieval/context commands return `document_not_ready` when no documents are ready in the active corpus.
- [ ] [FR-108] When some active-corpus documents are non-ready, retrieval uses only ready documents and returns partial-corpus metadata plus excluded non-ready counts/IDs.
- [ ] [FR-402] Errors follow the documented error format.
- [ ] [FR-001, FR-402] `ingest` returns `runtime_mode_forbidden` when uploads are disabled by public packaged demo mode or blocked by pre-compliance production mode.
- [ ] Public packaged demo upload attempts are rejected engine-side with explanatory copy.
- [ ] Engine validates file type and size.
- [ ] Engine handles provider failures without crashing.
- [ ] Retrieval API remains stateless for MVP; conversation memory is not required.
- [ ] [FR-109] Retrieval/context rate limiting enforces the configured durable limit keyed by `runtime_mode + corpus_id + provider_id + retrieval_scope`, returns `rate_limit_exceeded` with `retry_after_seconds`, stores counters in durable local state/data, and does not reset merely because a process exits or restarts.
- [ ] Sample corpus/index initialization is deterministic: packaged sample build either ships with a ready sample index or builds it on first run with clear recovery if initialization fails.

## CLI/config acceptance

- [ ] Commands have stable help text and documented examples.
- [ ] `--json` output is machine-readable and stable across runs.
- [ ] Human-readable default output stays consistent enough for agents and operators.
- [ ] stdout contains the primary result; stderr contains diagnostics and errors.
- [ ] Exit codes are documented and consistent.
- [ ] [FR-405, FR-406] README documents config files, environment variables, precedence, runtime mode, data/cache/index paths, and local reset/delete steps.
- [ ] [FR-407] Provider secrets are loaded through documented env/config mechanisms and redacted from errors, logs, and traces.

## Engineering acceptance

- [ ] Project has a clear README in English.
- [ ] `.env.example` documents required environment variables.
- [ ] CLI can be built and launched locally from documented commands.
- [ ] Packaging support exists for the CLI binary.
- [ ] Tests run from a single command.
- [ ] CI runs tests and linting.
- [ ] No API keys or secrets are committed, and packaged demo artifacts do not contain embedded real provider keys.
- [ ] Logs include ingestion, retrieval, context, rate-limit, and durable lock events.
- [ ] Logs use the documented safe-field allowlist and avoid full document text, full prompts, secrets, API keys, raw filenames/display names, provider error payloads, user query text, citation text, chunk text, and raw personal data.
- [ ] README includes Chile privacy-law and AI governance caveats.
- [ ] [FR-008] README documents local storage paths and manual reset/delete steps for imported documents, extracted text, chunks, embeddings, indexes, minimal graph/source metadata, durable locks, rate-limit state, and derived metadata.
- [ ] Local/private README and CLI output state that MVP local storage relies on operator-controlled device/OS/filesystem protections and does not promise encryption at rest unless separately configured.

## Demo acceptance

A reviewer should be able to complete one of these demo modes in under 5 minutes.

| Runtime mode | Upload behavior | Acceptance rule |
|---|---|---|
| Public packaged demo | Disabled | Uses preloaded sample documents only. |
| Local/private CLI demo | Enabled | Requires no-sensitive-data warning and provider disclosure. |
| Production | Blocked | Not enabled until compliance checklist is complete. |

### Demo prerequisites

**Packaged sample build:**

- Download the latest release binary for your OS (Windows, macOS, or Linux).
- No additional setup required; the CLI ships with sample documents and bundled dependencies.
- API key: the packaged demo must not contain a real bundled provider key. It uses a mock/local/offline provider with no user configuration, or it requires a user-supplied key outside the packaged artifact for private mode only.

**Local/private CLI demo:**

- Rust toolchain (stable, 1.75+).
- Embedding provider configured through documented environment variables or config file, unless local/mock embeddings are used.
- Build and run from the project root using the documented CLI commands.

### Packaged sample build

1. Open the packaged CLI demo.
2. See that uploads are disabled, ingest attempts are rejected with explanatory copy, and the demo uses preloaded sample documents.
3. Retrieve context for a query answered by the sample documents.
4. Inspect citations through `read` or `trace` output.
5. Retrieve context for a query not answered by the sample documents.
6. See safe `no_results` or `insufficient_context` output.
7. See provider disclosure and the verification disclaimer.

### Local/private CLI demo

1. Run the CLI locally or in a private environment.
2. Import a document after seeing the no-sensitive-data warning.
3. Wait for ready status or receive a terminal ingestion result from `ingest`.
4. Retrieve context supported by the document.
5. Inspect citations through `read` or `trace` output.
6. Retrieve context for a query not answered by the document.
7. See safe `no_results` or `insufficient_context` output.
8. See provider disclosure and the verification disclaimer.
9. Confirm README documents config, durable local state, manual reset/delete steps, and local/private storage protections/non-protections.

## Capability evidence

The project is ready when it clearly demonstrates:

| Capability | Evidence |
|---|---|
| Grounded retrieval | Vector-first retrieval, context packs, citations, source-read, no-results behavior |
| CLI/engine engineering | Rust engine, CLI commands, validation, durable locks/state, error handling |
| Production thinking | env/config contract, packaging, CI, tests, logs, rate limiting |
| Product thinking | PRD, scoped MVP, user flows, acceptance criteria |
| Privacy/compliance thinking | Ley 19.628 / Ley 21.719 baseline, demo safety policy, data minimization |
| AI governance | Provider registry, context trace, human accountability |
| Communication | English README, diagrams, clear demo flow |

## Related docs

- [MVP Scope](./03-mvp-scope.md)
- [Functional Requirements](./04-functional-requirements.md)
- [Risks and Open Questions](./11-risks-open-questions.md)
- [Legal and Privacy Compliance](./12-legal-privacy-compliance.md)
- [AI Ethics and Governance](./13-ai-ethics-governance.md)
