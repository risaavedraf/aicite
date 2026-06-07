# Apply Progress: v0.4.0-tags-lifecycle-ollama

## Structured status consumed

- Active change: `v0.4.0-tags-lifecycle-ollama`
- Apply state: ready
- Artifact store: `openspec+engram` (Engram tools unavailable in this subagent toolset; persisted progress written to OpenSpec only)
- Action context: implementation
- Workspace root: `E:/Proyectos/Intento_de_conseguir_pega/aiharness`
- Allowed edit roots: `crates/storage/src`, `crates/storage/src/migrations`, `crates/common/src`, `crates/engine/src`, `crates/ingest/src`, `crates/cli/src`, `openspec/changes/active/v0.4.0-tags-lifecycle-ollama`
- Warnings consumed: engine/ingest/cli edits limited to mechanical `Document { ... }` constructor updates; PR 1 only; high workload resolved as stacked-to-main PR 1 slice.
- GitHub issue: https://github.com/risaavedraf/aicite/issues/30
- Branch: `feat/v0.4-tags-schema-core`

## PR boundary

PR 1 only: schema + core tag APIs.

Implemented:
- migration 009 for `tags` table and document lifecycle columns;
- migration registration;
- `Document` lifecycle fields and storage row mapping/insert persistence;
- core storage tag API in `crates/storage/src/tags.rs`;
- storage tests for PR 1 semantics;
- persisted PR 1 task checkboxes in `tasks.md`.

Not implemented by design:
- CLI tag commands;
- `list --tag`;
- retrieval tag filters;
- ingest lifecycle behavior/auto-tags;
- changed re-ingest replacement;
- check-docs tag parsing;
- provider trait/factory/Ollama work.

## Completed tasks and persisted checkbox updates

PR 1 checkboxes marked `- [x]` in `openspec/changes/active/v0.4.0-tags-lifecycle-ollama/tasks.md`:

- RED storage tests for migration 9, lifecycle columns, tag idempotency/scoping/validation, reserved key behavior, and document `status:changed` rejection.
- GREEN migration 009.
- GREEN migration registration.
- GREEN `Document` lifecycle fields and document row mapping/insert updates.
- GREEN tag domain/storage helpers.
- GREEN `pub mod tags` export.
- VERIFY focused storage tests, `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.
- VERIFY refactor note: no CLI behavior added.

## Files changed

- `crates/storage/src/migrations/009_tags_lifecycle.sql` — new additive migration for tags and lifecycle columns/indexes.
- `crates/storage/src/migrations/mod.rs` — registers migration 009.
- `crates/storage/src/tags.rs` — new tag domain/storage helpers and tests.
- `crates/storage/src/lib.rs` — exports `tags` module and adds migration version/column tests.
- `crates/storage/src/documents.rs` — persists/maps lifecycle fields and tests lifecycle persistence.
- `crates/common/src/types.rs` — adds lifecycle fields to `Document`.
- `crates/storage/src/snapshots.rs` — updates old-schema migration test fixture so it can migrate through version 9.
- Mechanical `Document { ... }` constructor updates for new lifecycle fields:
  - `crates/cli/src/commands/evaluate.rs`
  - `crates/engine/src/context.rs`
  - `crates/engine/src/evaluate.rs`
  - `crates/engine/src/ingest.rs`
  - `crates/engine/src/recovery.rs`
  - `crates/engine/src/refresh.rs`
  - `crates/engine/src/retrieve.rs`
  - `crates/ingest/src/lib.rs`
  - `crates/storage/src/chunks.rs`
  - `crates/storage/src/concepts.rs`
  - `crates/storage/src/embeddings.rs`
  - `crates/storage/src/semantic_links.rs`
  - `crates/storage/src/topics.rs`
  - `crates/storage/src/traces.rs`
- `openspec/changes/active/v0.4.0-tags-lifecycle-ollama/tasks.md` — PR 1 checkboxes marked complete.
- `openspec/changes/active/v0.4.0-tags-lifecycle-ollama/apply-progress.md` — this cumulative progress update.

## Test commands run

- `cargo test -p storage tags:: -- --nocapture` — passed (8 tag tests).
- `cargo test -p storage test_migration_version_9_adds_tags_and_lifecycle_columns -- --nocapture` — passed.
- `cargo test -p storage test_insert_and_get_document_lifecycle_fields -- --nocapture` — passed.
- `cargo test -p storage --lib` — passed (124 tests).
- `cargo fmt --check` — passed after formatting.
- `cargo clippy -- -D warnings` — passed.
- `cargo test` — passed.

## Deviations from design/tasks

- No product-scope deviation: PR 1 only was implemented.
- The existing `snapshots` old-schema migration regression test fixture was expanded with a minimal `documents` table so the test can migrate through new version 9; this preserves the original test intent while accounting for the new migration dependency.
- Review-size risk: PR 1 required a new tested `tags.rs` module plus mechanical `Document` constructor updates. This likely exceeds the nominal 400 changed-line budget once untracked new files are counted, even though it remains one coherent storage/schema work unit.

## Remaining tasks

Exact unchecked task lines remaining in `tasks.md`:

```text
- [ ] Add CLI/parser tests in `crates/cli/src/commands/mod.rs`, new `crates/cli/src/commands/tag.rs`, and `crates/cli/src/commands/list.rs` for `key:value` mutation parsing, key-only mutation rejection, entity inference from `doc_*`/`chunk_*`, `--entity-type document|chunk`, reserved-key rejection, exact remove semantics, AND list filters, and `list --tag status:changed` not matching documents with only changed chunks.
- [ ] Add `Commands::Tag(commands::tag::TagArgs)` in `crates/cli/src/main.rs` and `pub mod tag;` in `crates/cli/src/commands/mod.rs`.
- [ ] Implement `crates/cli/src/commands/tag.rs` with `set/get/rm`, entity inference, optional `--entity-type`, and calls to user tag storage APIs.
- [ ] Convert `List` to `List(commands::list::ListArgs)` in `crates/cli/src/main.rs` and implement `--tag` parsing in `crates/cli/src/commands/list.rs`.
- [ ] Add document-local tag-filtered list storage helper in `crates/storage/src/documents.rs` using one bound `EXISTS` clause per filter.
- [ ] Run focused CLI/storage tests, then `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.
- [ ] Add tests in `crates/storage/src/embeddings.rs`, `crates/engine/src/retrieve.rs`, `crates/engine/src/context.rs`, and CLI command modules `search.rs`, `retrieve.rs`, `context.rs` for exact tag filtering, multiple-filter AND semantics, `status` chunk-local behavior, sibling non-inheritance, pre-ranking exclusion, and legacy `--topic`/`--concept` regression behavior.
- [ ] Add shared retrieval tag filter parsing/validation in `crates/cli/src/commands/mod.rs` or a concrete helper module under `crates/cli/src/commands/`.
- [ ] Add `--tag` args to `crates/cli/src/commands/search.rs`, `retrieve.rs`, and `context.rs` and pass parsed filters into engine requests.
- [ ] Extend retrieval request/data flow in `crates/engine/src/retrieve.rs` and `crates/engine/src/context.rs` with `tag_filters`.
- [ ] Update candidate SQL in `crates/storage/src/embeddings.rs` to append bound chunk-local tag `EXISTS` clauses without string-interpolating user values.
- [ ] Run focused retrieval tests, then `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.
- [ ] Add tests in `crates/engine/src/ingest.rs`, `crates/storage/src/documents.rs`, and `crates/storage/src/tags.rs` for `source_hash`, `ingested_at`, `file_modified_at`, source-path lookup, unchanged hash skip, no duplicate active source path, and OpenSpec path auto-tags on both documents and chunks.
- [ ] Implement `get_document_by_file_path` and lifecycle update helpers in `crates/storage/src/documents.rs` using the same canonical path format that ingest stores.
- [ ] Compute source hash and file modified time in `crates/engine/src/ingest.rs` before expensive extraction/embedding.
- [ ] Recheck existing document hash under the existing ingest pipeline lock and return the existing document result without re-chunking/re-embedding when unchanged.
- [ ] Store lifecycle fields on initial ingest and successful changed ingest.
- [ ] Add engine-owned auto-tag assignment in `crates/engine/src/ingest.rs` for `source_kind:document`, `workspace:<name>`, and OpenSpec path mappings to `type:prd|spec|architecture|guide|rfc` on documents and chunks; do not propagate `status`.
- [ ] Run focused ingest/storage tests, then `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.
- [ ] Add tests in `crates/engine/src/ingest.rs`, `crates/storage/src/chunks.rs`, `crates/storage/src/embeddings.rs`, and `crates/storage/src/tags.rs` for changed source processing, document ID reuse, no duplicate active documents, content-hash changed chunk detection including duplicate text counts, stale `status:changed` cleanup, document `status:changed` ban, and failure preserving last ready representation where practical.
- [ ] Add storage helpers for transactional replacement of a document's chunks, embeddings, related chunk tags, and hierarchy rows in `crates/storage/src/chunks.rs`, `crates/storage/src/embeddings.rs`, and `crates/storage/src/tags.rs`.
- [ ] Implement changed-source branch in `crates/engine/src/ingest.rs` that compares previous/new chunk text hashes, marks only known changed/new chunks with engine-owned `status:changed`, and never writes document-local `status:changed`.
- [ ] Clear stale chunk-local `status:changed` for the document during successful replacement before recreating current changed tags.
- [ ] Update lifecycle metadata, chunk count, and pipeline status only after successful extract/chunk/embed/replace.
- [ ] Run focused ingest replacement tests, then `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.
- [ ] Add tests in `crates/cli/src/commands/check_docs.rs` and `crates/check-docs/src/parser.rs` if that parser crate owns markdown parsing for adjacent `<!-- tag:status=planned -->`, `<!-- tag:status=implemented -->`, unknown tags, blank-line adjacency, and untagged default verification.
- [ ] Extend markdown parsing to capture `<!-- tag:key=value -->` comments with line numbers and associate recognized status tags to the next adjacent Cite command block.
- [ ] Map `status=planned` to planned/skipped/warning output such as `Planned command; verification skipped` instead of outdated failure.
- [ ] Ensure `status=implemented`, unknown tags, and untagged commands use existing verification paths.
- [ ] Run focused check-docs tests, then `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.
- [ ] Add tests in `crates/providers/src/lib.rs`, `crates/providers/src/gemini.rs`, `crates/providers/src/openai.rs`, `crates/cli/src/commands/mod.rs`, `crates/cli/src/commands/health.rs`, and `crates/config/src/lib.rs` for sequential `embed_batch` order, `BatchStrategy` defaults/overrides, Gemini/OpenAI-compatible key enforcement, placeholder Ollama no-key branch behavior if introduced here, endpoint fallback compatibility, and health strategy output.
- [ ] Add `BatchStrategy` and default `EmbeddingProvider::embed_batch` / `batch_strategy` methods in `crates/providers/src/lib.rs`.
- [ ] Update `crates/providers/src/gemini.rs` and `crates/providers/src/openai.rs` to compile with the extended trait and report the selected strategy.
- [ ] Refactor `create_provider` in `crates/cli/src/commands/mod.rs` so provider selection happens before API-key validation.
- [ ] Extend `crates/config/src/lib.rs` with optional provider fields: `endpoint`, `dimensions`, `device`, `batch_size`, and `workspace`, preserving `ingest.embedding_endpoint` as compatibility fallback where needed.
- [ ] Update `crates/cli/src/commands/health.rs` to include provider id/model and batch strategy without requiring live Ollama support yet.
- [ ] Run focused provider/config/health tests, then `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.
- [ ] Add unit tests in new `crates/providers/src/ollama.rs`, `crates/providers/src/lib.rs`, `crates/cli/src/commands/mod.rs`, and `crates/cli/src/commands/health.rs` for no-key creation, default endpoint `http://localhost:11434`, request body to `/api/embed`, response parsing order/count, `embed` delegating to batch, native `BatchStrategy`, unreachable endpoint health message, and existing provider regression behavior.
- [ ] Add `crates/providers/src/ollama.rs` with HTTP client, endpoint/model validation, `embed_batch` native request, `embed` single-text wrapper, response parsing, and actionable errors.
- [ ] Export `pub mod ollama;` from `crates/providers/src/lib.rs` and add the `ollama` factory branch in `crates/cli/src/commands/mod.rs` with no API-key requirement.
- [ ] Resolve Ollama endpoint from `embedding.endpoint` first, compatibility fallback if appropriate, then default local endpoint.
- [ ] Extend `crates/cli/src/commands/health.rs` to report Ollama provider id, model, endpoint, connectivity status, latency when measurable, error text, and native batch strategy.
- [ ] Add an ignored live-test target or documented manual check only if it stays outside normal `cargo test` requirements.
- [ ] Run focused Ollama/provider/health tests, then `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.
- [ ] Run `cargo fmt --check`.
- [ ] Run `cargo clippy -- -D warnings`.
- [ ] Run `cargo test`.
- [ ] Manually smoke-test representative commands after implementation: `cite tag set/get/rm`, `cite list --tag type:rfc`, `cite retrieve "query" --tag status:changed`, unchanged re-ingest, changed re-ingest, `cite check-docs`, and `cite health` for existing and Ollama provider configs.
```

## Workload / review notes

- Delivery path remains stacked-to-main.
- Current PR boundary should be reviewed as storage/schema only.
- Because PR 1 includes a fully tested new module and mechanical constructor updates, parent should review changed-line count before PR creation and decide whether to keep PR 1 as a size exception or split tests/API into smaller stacked commits/PRs.


## PR 2 update — Tag CLI + document-local list filters

### Structured status consumed

- Active change: `v0.4.0-tags-lifecycle-ollama`
- Apply state: `ready_for_pr2`
- Artifact store: `openspec+engram` (Engram unavailable to this subagent; OpenSpec artifacts updated)
- Action context: implementation
- Workspace root: `E:/Proyectos/Intento_de_conseguir_pega/aiharness`
- Allowed edit roots used: `crates/cli/src`, `crates/storage/src`, `openspec/changes/active/v0.4.0-tags-lifecycle-ollama`
- Warnings consumed: PR 2 only; `status` is local-only/non-inheritable; `list --tag status:changed` is document-local only; no search/retrieve/context tag filters implemented.
- GitHub issue: https://github.com/risaavedraf/aicite/issues/30
- Branch: `feat/v0.4-tag-cli-list`

### PR boundary

PR 2 only: tag CLI + document-local `list --tag` filters.

Implemented:
- `cite tag set/get/rm` command with entity inference for `doc_*` / `chunk_*` and optional `--entity-type document|chunk`.
- User mutation path calls storage user APIs, so reserved keys are rejected and `status:changed` remains chunk-only.
- Mutation inputs require `key:value`; key-only mutations are rejected.
- `cite list --tag` parsing accepts key:value and deliberate key-only filters.
- Document-local tag-filtered list storage helper with AND semantics via one bound `EXISTS` clause per filter.
- Tests for tag parsing, entity inference, explicit entity type, reserved-key rejection, exact remove semantics, list AND semantics, key-only list filters, and no chunk-to-document status inference.

Not implemented by design:
- `--tag` on `search`, `retrieve`, or `context`.
- ingest lifecycle behavior, auto-tags, changed chunk recalculation, check-docs markdown tags, provider trait/factory, or Ollama.

### Completed tasks and persisted checkbox updates

PR 2 checkboxes marked `- [x]` in `openspec/changes/active/v0.4.0-tags-lifecycle-ollama/tasks.md`:

- RED CLI/parser/storage tests for tag mutation parsing, key-only mutation rejection, entity inference, explicit entity type, reserved key rejection, exact remove semantics, list AND filters, and document-local `status` behavior.
- GREEN `Commands::Tag` and `pub mod tag` wiring.
- GREEN `crates/cli/src/commands/tag.rs` implementation.
- GREEN `List(commands::list::ListArgs)` and `--tag` parsing.
- GREEN document-local `list_documents_by_tags` storage helper.
- VERIFY focused CLI/storage tests, `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.

### Files changed

- `crates/cli/src/main.rs` — wires `List(ListArgs)` and `Tag(TagArgs)`.
- `crates/cli/src/commands/mod.rs` — exports `tag` module.
- `crates/cli/src/commands/tag.rs` — new `cite tag set/get/rm` command, parser helpers, output, and tests.
- `crates/cli/src/commands/list.rs` — adds `ListArgs`, `--tag` filter parsing, filtered list execution, and tests.
- `crates/storage/src/documents.rs` — adds document-local `list_documents_by_tags` and tests.
- `crates/storage/src/tags.rs` — tightens `TagFilter::parse` to reject leading/trailing whitespace.
- `openspec/changes/active/v0.4.0-tags-lifecycle-ollama/tasks.md` — PR 2 checkboxes marked complete.
- `openspec/changes/active/v0.4.0-tags-lifecycle-ollama/apply-progress.md` — this cumulative update.

### Test commands run

- `cargo test -p cli -- --nocapture` — passed (32 tests).
- `cargo test -p storage tags:: -- --nocapture` — passed (8 tag tests).
- `cargo test -p storage test_list_documents_by -- --nocapture` — passed (4 matching storage tests).
- `cargo fmt --check` — passed.
- `cargo clippy -- -D warnings` — passed.
- `cargo test` — passed.

Failed/incorrect focused command attempts before correction:
- `cargo test -p cli tag:: list:: commands:: -- --nocapture` — invalid Cargo test invocation.
- `cargo test -p cli --lib -- --nocapture` — failed because `cli` has no library target.
- `cargo test -p storage tags:: documents::tests::test_list_documents_by -- --nocapture` — invalid Cargo test invocation.

### Deviations from design/tasks

- No product-scope deviation: PR 2 only was implemented.
- `TagFilter::parse` now rejects leading/trailing whitespace for filters, matching mutation validation and preventing malformed `--tag` inputs from being silently normalized.
- Review-size risk: PR 2 likely exceeds the nominal 400-line budget due to a new CLI command module plus storage/list tests, but it remains a coherent tag CLI/list work unit.

### Remaining tasks

Exact unchecked task lines remaining in `tasks.md` after PR 2:

```text
- [ ] Add tests in `crates/storage/src/embeddings.rs`, `crates/engine/src/retrieve.rs`, `crates/engine/src/context.rs`, and CLI command modules `search.rs`, `retrieve.rs`, `context.rs` for exact tag filtering, multiple-filter AND semantics, `status` chunk-local behavior, sibling non-inheritance, pre-ranking exclusion, and legacy `--topic`/`--concept` regression behavior.
- [ ] Add shared retrieval tag filter parsing/validation in `crates/cli/src/commands/mod.rs` or a concrete helper module under `crates/cli/src/commands/`.
- [ ] Add `--tag` args to `crates/cli/src/commands/search.rs`, `retrieve.rs`, and `context.rs` and pass parsed filters into engine requests.
- [ ] Extend retrieval request/data flow in `crates/engine/src/retrieve.rs` and `crates/engine/src/context.rs` with `tag_filters`.
- [ ] Update candidate SQL in `crates/storage/src/embeddings.rs` to append bound chunk-local tag `EXISTS` clauses without string-interpolating user values.
- [ ] Run focused retrieval tests, then `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.
- [ ] Add tests in `crates/engine/src/ingest.rs`, `crates/storage/src/documents.rs`, and `crates/storage/src/tags.rs` for `source_hash`, `ingested_at`, `file_modified_at`, source-path lookup, unchanged hash skip, no duplicate active source path, and OpenSpec path auto-tags on both documents and chunks.
- [ ] Implement `get_document_by_file_path` and lifecycle update helpers in `crates/storage/src/documents.rs` using the same canonical path format that ingest stores.
- [ ] Compute source hash and file modified time in `crates/engine/src/ingest.rs` before expensive extraction/embedding.
- [ ] Recheck existing document hash under the existing ingest pipeline lock and return the existing document result without re-chunking/re-embedding when unchanged.
- [ ] Store lifecycle fields on initial ingest and successful changed ingest.
- [ ] Add engine-owned auto-tag assignment in `crates/engine/src/ingest.rs` for `source_kind:document`, `workspace:<name>`, and OpenSpec path mappings to `type:prd|spec|architecture|guide|rfc` on documents and chunks; do not propagate `status`.
- [ ] Run focused ingest/storage tests, then `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.
- [ ] Add tests in `crates/engine/src/ingest.rs`, `crates/storage/src/chunks.rs`, `crates/storage/src/embeddings.rs`, and `crates/storage/src/tags.rs` for changed source processing, document ID reuse, no duplicate active documents, content-hash changed chunk detection including duplicate text counts, stale `status:changed` cleanup, document `status:changed` ban, and failure preserving last ready representation where practical.
- [ ] Add storage helpers for transactional replacement of a document's chunks, embeddings, related chunk tags, and hierarchy rows in `crates/storage/src/chunks.rs`, `crates/storage/src/embeddings.rs`, and `crates/storage/src/tags.rs`.
- [ ] Implement changed-source branch in `crates/engine/src/ingest.rs` that compares previous/new chunk text hashes, marks only known changed/new chunks with engine-owned `status:changed`, and never writes document-local `status:changed`.
- [ ] Clear stale chunk-local `status:changed` for the document during successful replacement before recreating current changed tags.
- [ ] Update lifecycle metadata, chunk count, and pipeline status only after successful extract/chunk/embed/replace.
- [ ] Run focused ingest replacement tests, then `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.
- [ ] Add tests in `crates/cli/src/commands/check_docs.rs` and `crates/check-docs/src/parser.rs` if that parser crate owns markdown parsing for adjacent `<!-- tag:status=planned -->`, `<!-- tag:status=implemented -->`, unknown tags, blank-line adjacency, and untagged default verification.
- [ ] Extend markdown parsing to capture `<!-- tag:key=value -->` comments with line numbers and associate recognized status tags to the next adjacent Cite command block.
- [ ] Map `status=planned` to planned/skipped/warning output such as `Planned command; verification skipped` instead of outdated failure.
- [ ] Ensure `status=implemented`, unknown tags, and untagged commands use existing verification paths.
- [ ] Run focused check-docs tests, then `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.
- [ ] Add tests in `crates/providers/src/lib.rs`, `crates/providers/src/gemini.rs`, `crates/providers/src/openai.rs`, `crates/cli/src/commands/mod.rs`, `crates/cli/src/commands/health.rs`, and `crates/config/src/lib.rs` for sequential `embed_batch` order, `BatchStrategy` defaults/overrides, Gemini/OpenAI-compatible key enforcement, placeholder Ollama no-key branch behavior if introduced here, endpoint fallback compatibility, and health strategy output.
- [ ] Add `BatchStrategy` and default `EmbeddingProvider::embed_batch` / `batch_strategy` methods in `crates/providers/src/lib.rs`.
- [ ] Update `crates/providers/src/gemini.rs` and `crates/providers/src/openai.rs` to compile with the extended trait and report the selected strategy.
- [ ] Refactor `create_provider` in `crates/cli/src/commands/mod.rs` so provider selection happens before API-key validation.
- [ ] Extend `crates/config/src/lib.rs` with optional provider fields: `endpoint`, `dimensions`, `device`, `batch_size`, and `workspace`, preserving `ingest.embedding_endpoint` as compatibility fallback where needed.
- [ ] Update `crates/cli/src/commands/health.rs` to include provider id/model and batch strategy without requiring live Ollama support yet.
- [ ] Run focused provider/config/health tests, then `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.
- [ ] Add unit tests in new `crates/providers/src/ollama.rs`, `crates/providers/src/lib.rs`, `crates/cli/src/commands/mod.rs`, and `crates/cli/src/commands/health.rs` for no-key creation, default endpoint `http://localhost:11434`, request body to `/api/embed`, response parsing order/count, `embed` delegating to batch, native `BatchStrategy`, unreachable endpoint health message, and existing provider regression behavior.
- [ ] Add `crates/providers/src/ollama.rs` with HTTP client, endpoint/model validation, `embed_batch` native request, `embed` single-text wrapper, response parsing, and actionable errors.
- [ ] Export `pub mod ollama;` from `crates/providers/src/lib.rs` and add the `ollama` factory branch in `crates/cli/src/commands/mod.rs` with no API-key requirement.
- [ ] Resolve Ollama endpoint from `embedding.endpoint` first, compatibility fallback if appropriate, then default local endpoint.
- [ ] Extend `crates/cli/src/commands/health.rs` to report Ollama provider id, model, endpoint, connectivity status, latency when measurable, error text, and native batch strategy.
- [ ] Add an ignored live-test target or documented manual check only if it stays outside normal `cargo test` requirements.
- [ ] Run focused Ollama/provider/health tests, then `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.
- [ ] Run `cargo fmt --check`.
- [ ] Run `cargo clippy -- -D warnings`.
- [ ] Run `cargo test`.
- [ ] Manually smoke-test representative commands after implementation: `cite tag set/get/rm`, `cite list --tag type:rfc`, `cite retrieve "query" --tag status:changed`, unchanged re-ingest, changed re-ingest, `cite check-docs`, and `cite health` for existing and Ollama provider configs.
```

### Workload / review notes

- Delivery path remains stacked-to-main.
- Current PR boundary should be reviewed as tag CLI + document-local list filtering only.
- PR 3 should start from this branch after parent review/commit and explicit approval.

## PR 3 update — Retrieval tag filters

### Structured status consumed

- Active change: `v0.4.0-tags-lifecycle-ollama`
- Apply state: PR 3 explicitly approved by user/parent prompt.
- Artifact store: `openspec+engram` (Engram memory tools unavailable in this subagent toolset; OpenSpec artifacts updated)
- Action context: repo-local implementation in `E:/Proyectos/Intento_de_conseguir_pega/aiharness`
- Allowed edit roots used: `crates/storage/src/embeddings.rs`, `crates/engine/src/retrieve.rs`, `crates/engine/src/context.rs`, `crates/cli/src/commands/search.rs`, `crates/cli/src/commands/retrieve.rs`, `crates/cli/src/commands/context.rs`, `crates/cli/src/commands/mod.rs`, and this OpenSpec change directory.
- GitHub issue: https://github.com/risaavedraf/aicite/issues/30
- Branch: `feat/v0.4-tag-cli-list`

### PR boundary

PR 3 only: chunk-local retrieval tag filters for `search`, `retrieve`, and `context`.

Implemented:
- Shared retrieval tag filter parsing via `parse_retrieval_tag_filters`.
- `--tag` args for `cite search`, `cite retrieve`, and `cite context`.
- Engine retrieval/context request data flow carrying `tag_filters`.
- Storage candidate filtering with one chunk-local `EXISTS` clause per tag filter and bound SQL parameters.
- Tests for exact tag filtering, multiple-filter AND semantics, chunk-local `status`, no document/sibling inheritance, pre-ranking exclusion, and legacy topic/concept compatibility with tag filters.

Not implemented by design:
- PR 4+ ingest lifecycle, auto-tags, changed re-ingest replacement, check-docs tags, provider trait/factory, or Ollama.
- Document tag aggregation for retrieval; retrieval filters remain chunk-local only.

### Completed tasks and persisted checkbox updates

PR 3 checkboxes marked `- [x]` in `openspec/changes/active/v0.4.0-tags-lifecycle-ollama/tasks.md`:

- RED storage/engine/CLI tests for exact tag filtering, AND semantics, status chunk-local behavior, sibling/document non-inheritance, pre-ranking exclusion, and legacy topic/concept regression behavior.
- GREEN shared retrieval tag filter parsing/validation.
- GREEN `--tag` args for `search`, `retrieve`, and `context` and engine request wiring.
- GREEN retrieval/context `tag_filters` data flow.
- GREEN storage candidate SQL with bound chunk-local tag `EXISTS` clauses.
- VERIFY focused retrieval tests, `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.

### Files changed

- `crates/storage/src/embeddings.rs` — adds tag-filtered ready/hierarchical candidate helpers and storage tests.
- `crates/engine/src/retrieve.rs` — adds `tag_filters` request field, tag-aware search/retrieve entry points, candidate flow, and retrieval tests.
- `crates/engine/src/context.rs` — adds tag-aware context entry point and context citation test.
- `crates/cli/src/commands/mod.rs` — adds shared retrieval tag filter parser and parser tests.
- `crates/cli/src/commands/search.rs` — adds `--tag` arg, parser call, tag-aware engine call, and CLI module test.
- `crates/cli/src/commands/retrieve.rs` — adds `--tag` arg, parser call, tag-aware engine call, and CLI module test.
- `crates/cli/src/commands/context.rs` — adds `--tag` arg, parser call, tag-aware engine call, and CLI module test.
- `crates/cli/src/main.rs` — adds parent hardening tests that parse real repeated `--tag` argv for `search`, `retrieve`, and `context`.
- `openspec/changes/active/v0.4.0-tags-lifecycle-ollama/tasks.md` — PR 3 checkboxes marked complete.
- `openspec/changes/active/v0.4.0-tags-lifecycle-ollama/apply-progress.md` — this cumulative PR 3 update.

### Test commands run

- `cargo test -p storage embeddings::tests::test_list_ready_chunk_embeddings_by_tags -- --nocapture` — passed (2 matching storage tests).
- `cargo test -p engine tag -- --nocapture` — passed (3 matching engine tests; included retrieval tag filters and context tag filtering).
- `cargo test -p engine retrieve::tests::test_retrieve_status_changed_is_chunk_local_only -- --nocapture` — passed.
- `cargo test -p cli tag_filters -- --nocapture` — passed (5 matching CLI parser/module tests).
- `cargo test -p cli parse_retrieval_tag_filters -- --nocapture` — passed (2 matching shared parser tests).
- `cargo test -p cli parses_repeated_tag_filters_in_order -- --nocapture` — passed (3 parent-added Clap argv parsing tests).
- `cargo fmt --check` — passed after formatting.
- `cargo clippy -- -D warnings` — passed.
- `cargo test` — passed.

Failed/incorrect focused command attempts before correction:
- `cargo test -p engine retrieve::tests::test_search_with_tags_filters_before_ranking_and_uses_and_semantics retrieve::tests::test_retrieve_status_changed_is_chunk_local_only retrieve::tests::test_search_combines_legacy_topic_filter_with_tag_filter context::tests::test_context_with_tags_returns_only_matching_chunk_citations -- --nocapture` — invalid Cargo test invocation with multiple test name arguments.
- Initial `cargo fmt --check` failed only on formatting; `cargo fmt` was run and the final `cargo fmt --check` passed.

### Deviations from design/tasks

- No product-scope deviation: PR 3 only was implemented.
- Existing public `search`, `retrieve`, and `build_context` APIs remain backward-compatible and delegate to new tag-aware variants with empty filters; CLI uses the new tag-aware variants.
- Key-only tag filters remain accepted through the existing `TagFilter::parse` design, but PR 3 tests emphasize exact `key:value` retrieval behavior.
- Parent fresh review found no blockers and suggested real Clap argv parsing coverage; parent added those parser-level tests before final validation.

### Remaining tasks

PR 3 is complete. Exact unchecked task lines remaining in `tasks.md` are PR 4+ and final cross-slice verification tasks, starting with:

```text
- [ ] Add tests in `crates/engine/src/ingest.rs`, `crates/storage/src/documents.rs`, and `crates/storage/src/tags.rs` for `source_hash`, `ingested_at`, `file_modified_at`, source-path lookup, unchanged hash skip, no duplicate active source path, and OpenSpec path auto-tags on both documents and chunks.
- [ ] Implement `get_document_by_file_path` and lifecycle update helpers in `crates/storage/src/documents.rs` using the same canonical path format that ingest stores.
- [ ] Compute source hash and file modified time in `crates/engine/src/ingest.rs` before expensive extraction/embedding.
- [ ] Recheck existing document hash under the existing ingest pipeline lock and return the existing document result without re-chunking/re-embedding when unchanged.
- [ ] Store lifecycle fields on initial ingest and successful changed ingest.
- [ ] Add engine-owned auto-tag assignment in `crates/engine/src/ingest.rs` for `source_kind:document`, `workspace:<name>`, and OpenSpec path mappings to `type:prd|spec|architecture|guide|rfc` on documents and chunks; do not propagate `status`.
- [ ] Run focused ingest/storage tests, then `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.
```

### Workload / review notes

- Delivery path remains stacked-to-main.
- Current PR boundary should be reviewed as retrieval tag filters only.
- PR 4 should start only after parent review/commit and explicit approval.

## PR 4 update — Ingest lifecycle skip + auto-tags

- Status consumed: change `v0.4.0-tags-lifecycle-ollama`, apply `ready`, repo-local allowed root, branch `feat/v0.4-retrieval-tag-filters`; PR 4 only, PR 5+ untouched.
- Completed/persisted: all PR 4 RED/GREEN/VERIFY checkboxes marked `- [x]` in `tasks.md`.
- Files changed: `Cargo.toml`, `crates/engine/Cargo.toml`, `crates/engine/src/ingest.rs`, `crates/storage/src/documents.rs`, `tasks.md`, `apply-progress.md`.
- Behavior: SHA-256 source lifecycle metadata, canonical source-path lookup, unchanged re-ingest skip under lock, engine-owned `source_kind`, `workspace`, and OpenSpec `type` auto-tags on documents/chunks without `status` propagation.
- Commands passed: focused storage lifecycle test; focused engine lifecycle, unchanged-skip, and auto-tag tests; `cargo fmt --check`; `cargo clippy -- -D warnings`; `cargo test`.
- TDD evidence: strict TDD inactive; focused PR4 tests were added and passed after implementation.
- Deviations/remaining: changed-source replacement and `status:changed` recalculation remain PR 5; final cross-slice verification/smokes remain unchecked. Diff is at/above the review budget once OpenSpec evidence is included, so parent should split or explicitly accept size before commit/PR.
