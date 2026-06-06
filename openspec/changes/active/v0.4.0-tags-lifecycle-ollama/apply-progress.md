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
