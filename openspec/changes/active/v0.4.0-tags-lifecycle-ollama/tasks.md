# Implementation Tasks: v0.4.0-tags-lifecycle-ollama

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 1,600-2,400 total across 8 slices |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 schema/core tags → PR 2 tag CLI/list → PR 3 retrieval filters → PR 4 lifecycle skip/auto-tags → PR 5 changed chunk status → PR 6 check-docs tags → PR 7 provider trait/factory → PR 8 Ollama provider |
| Delivery strategy | ask-on-risk |
| Chain strategy | stacked-to-main |

Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: stacked-to-main
400-line budget risk: High
GitHub issue: https://github.com/risaavedraf/aicite/issues/30

## Delivery gates

- GitHub issue for this change: https://github.com/risaavedraf/aicite/issues/30
- Chain strategy: `stacked-to-main`.
- Ask for explicit approval before applying each slice because project config uses `ask_always` and the total change is clearly over the 400-line review budget.
- Each slice should be one review-safe work unit with tests committed with the behavior they verify.
- If any slice approaches 400 changed lines, stop and split it before continuing.
- Verification baseline per slice: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` plus any narrower test commands listed below.

## PR 1 — Schema + core tag APIs

**Start:** Existing migration version 8; no tag table or document lifecycle columns.  
**Finish:** Additive schema and storage-level tag APIs exist with validation tests; no CLI/retrieval behavior yet.  
**Rollback:** Remove migration registration and `tags.rs` export before any later slices depend on them.

### RED

- [x] Add/extend storage tests in `crates/storage/src/lib.rs`, `crates/storage/src/documents.rs`, and new `crates/storage/src/tags.rs` for migration version 9, lifecycle columns on old/new rows, duplicate tag idempotency, entity-type scoping, reserved-key user rejection, engine reserved-key acceptance, malformed tag rejection, and document `status:changed` rejection for both user and engine paths.

### GREEN

- [x] Add `crates/storage/src/migrations/009_tags_lifecycle.sql` with `tags` table, indexes, nullable `documents.source_hash`, `documents.ingested_at`, `documents.file_modified_at`, and document path/hash indexes.
- [x] Register migration 009 in `crates/storage/src/migrations/mod.rs`.
- [x] Extend `common::types::Document` in `crates/common/src/types.rs` with nullable lifecycle fields and update row mapping/inserts in `crates/storage/src/documents.rs`.
- [x] Add tag domain/storage helpers in `crates/storage/src/tags.rs`: entity type, tag record/filter parsing helpers, `set_tag_user`, `set_tag_engine`, `remove_tag_user`, `remove_tag_engine`, `list_tags`, and `clear_chunk_status_changed_for_document`.
- [x] Export `pub mod tags;` from `crates/storage/src/lib.rs`.

### VERIFY / REFACTOR

- [x] Run focused storage tests, then `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.
- [x] Refactor only if tag parsing/validation duplication appears within this slice; do not add CLI behavior yet.

## PR 2 — Tag CLI + document-local list filters

**Depends on:** PR 1.  
**Finish:** Users can set/get/remove non-reserved local tags and filter `cite list` by document-local tags.  
**Rollback:** Remove `Tag` command and `ListArgs` changes without touching schema.

### RED

- [ ] Add CLI/parser tests in `crates/cli/src/commands/mod.rs`, new `crates/cli/src/commands/tag.rs`, and `crates/cli/src/commands/list.rs` for `key:value` mutation parsing, key-only mutation rejection, entity inference from `doc_*`/`chunk_*`, `--entity-type document|chunk`, reserved-key rejection, exact remove semantics, AND list filters, and `list --tag status:changed` not matching documents with only changed chunks.

### GREEN

- [ ] Add `Commands::Tag(commands::tag::TagArgs)` in `crates/cli/src/main.rs` and `pub mod tag;` in `crates/cli/src/commands/mod.rs`.
- [ ] Implement `crates/cli/src/commands/tag.rs` with `set/get/rm`, entity inference, optional `--entity-type`, and calls to user tag storage APIs.
- [ ] Convert `List` to `List(commands::list::ListArgs)` in `crates/cli/src/main.rs` and implement `--tag` parsing in `crates/cli/src/commands/list.rs`.
- [ ] Add document-local tag-filtered list storage helper in `crates/storage/src/documents.rs` using one bound `EXISTS` clause per filter.

### VERIFY / REFACTOR

- [ ] Run focused CLI/storage tests, then `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.

## PR 3 — Retrieval tag filters

**Depends on:** PR 1; preferably PR 2 shared parser.  
**Finish:** `search`, `retrieve`, and `context` accept chunk-local `--tag` filters before vector ranking while preserving topic/concept behavior.  
**Rollback:** Remove `tag_filters` request fields and SQL fragments; unfiltered retrieval remains intact.

### RED

- [ ] Add tests in `crates/storage/src/embeddings.rs`, `crates/engine/src/retrieve.rs`, `crates/engine/src/context.rs`, and CLI command modules `search.rs`, `retrieve.rs`, `context.rs` for exact tag filtering, multiple-filter AND semantics, `status` chunk-local behavior, sibling non-inheritance, pre-ranking exclusion, and legacy `--topic`/`--concept` regression behavior.

### GREEN

- [ ] Add shared retrieval tag filter parsing/validation in `crates/cli/src/commands/mod.rs` or a concrete helper module under `crates/cli/src/commands/`.
- [ ] Add `--tag` args to `crates/cli/src/commands/search.rs`, `retrieve.rs`, and `context.rs` and pass parsed filters into engine requests.
- [ ] Extend retrieval request/data flow in `crates/engine/src/retrieve.rs` and `crates/engine/src/context.rs` with `tag_filters`.
- [ ] Update candidate SQL in `crates/storage/src/embeddings.rs` to append bound chunk-local tag `EXISTS` clauses without string-interpolating user values.

### VERIFY / REFACTOR

- [ ] Run focused retrieval tests, then `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.

## PR 4 — Ingest lifecycle skip + auto-tags

**Depends on:** PR 1.  
**Finish:** First ingest stores lifecycle metadata and auto-tags; unchanged re-ingest skips chunk/embed work and avoids duplicate active documents.  
**Rollback:** Disable lifecycle lookup/skip and auto-tag writes; schema can remain unused.

### RED

- [ ] Add tests in `crates/engine/src/ingest.rs`, `crates/storage/src/documents.rs`, and `crates/storage/src/tags.rs` for `source_hash`, `ingested_at`, `file_modified_at`, source-path lookup, unchanged hash skip, no duplicate active source path, and OpenSpec path auto-tags on both documents and chunks.

### GREEN

- [ ] Implement `get_document_by_file_path` and lifecycle update helpers in `crates/storage/src/documents.rs` using the same canonical path format that ingest stores.
- [ ] Compute source hash and file modified time in `crates/engine/src/ingest.rs` before expensive extraction/embedding.
- [ ] Recheck existing document hash under the existing ingest pipeline lock and return the existing document result without re-chunking/re-embedding when unchanged.
- [ ] Store lifecycle fields on initial ingest and successful changed ingest.
- [ ] Add engine-owned auto-tag assignment in `crates/engine/src/ingest.rs` for `source_kind:document`, `workspace:<name>`, and OpenSpec path mappings to `type:prd|spec|architecture|guide|rfc` on documents and chunks; do not propagate `status`.

### VERIFY / REFACTOR

- [ ] Run focused ingest/storage tests, then `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.

## PR 5 — Changed re-ingest replacement + chunk-local `status:changed`

**Depends on:** PR 4.  
**Finish:** Changed re-ingest reuses the document identity, replaces chunks/embeddings safely, recalculates chunk-local `status:changed`, and clears stale changed tags.  
**Rollback:** Revert changed-source replacement path to full-process behavior while retaining lifecycle storage.

### RED

- [x] Add tests in `crates/engine/src/ingest.rs`, `crates/storage/src/chunks.rs`, `crates/storage/src/embeddings.rs`, and `crates/storage/src/tags.rs` for changed source processing, document ID reuse, no duplicate active documents, content-hash changed chunk detection including duplicate text counts, stale `status:changed` cleanup, document `status:changed` ban, and failure preserving last ready representation where practical.

### GREEN

- [x] Add storage helpers for transactional replacement of a document's chunks, embeddings, related chunk tags, and hierarchy rows in `crates/storage/src/chunks.rs`, `crates/storage/src/embeddings.rs`, and `crates/storage/src/tags.rs`.
- [x] Implement changed-source branch in `crates/engine/src/ingest.rs` that compares previous/new chunk text hashes, marks only known changed/new chunks with engine-owned `status:changed`, and never writes document-local `status:changed`.
- [x] Clear stale chunk-local `status:changed` for the document during successful replacement before recreating current changed tags.
- [x] Update lifecycle metadata, chunk count, and pipeline status only after successful extract/chunk/embed/replace.

### VERIFY / REFACTOR

- [x] Run focused ingest replacement tests, then `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.

## PR 6 — `check-docs` markdown status tags

**Depends on:** PR 1 semantic parser types only if reused; otherwise independent.  
**Finish:** Planned command examples annotated with markdown tag comments are not reported as outdated; implemented/unknown tags preserve verification behavior.  
**Rollback:** Remove markdown tag association parser; existing code-block verification remains.

### RED

- [ ] Add tests in `crates/cli/src/commands/check_docs.rs` and `crates/check-docs/src/parser.rs` if that parser crate owns markdown parsing for adjacent `<!-- tag:status=planned -->`, `<!-- tag:status=implemented -->`, unknown tags, blank-line adjacency, and untagged default verification.

### GREEN

- [ ] Extend markdown parsing to capture `<!-- tag:key=value -->` comments with line numbers and associate recognized status tags to the next adjacent Cite command block.
- [ ] Map `status=planned` to planned/skipped/warning output such as `Planned command; verification skipped` instead of outdated failure.
- [ ] Ensure `status=implemented`, unknown tags, and untagged commands use existing verification paths.

### VERIFY / REFACTOR

- [ ] Run focused check-docs tests, then `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.

## PR 7 — Provider trait, config, factory, and health batch strategy

**Depends on:** None from tag/lifecycle slices.  
**Finish:** Provider abstraction supports batch fallback/strategy; factory validates API keys after provider selection; health can report batch strategy.  
**Rollback:** Revert trait defaults/factory refactor before adding Ollama.

### RED

- [ ] Add tests in `crates/providers/src/lib.rs`, `crates/providers/src/gemini.rs`, `crates/providers/src/openai.rs`, `crates/cli/src/commands/mod.rs`, `crates/cli/src/commands/health.rs`, and `crates/config/src/lib.rs` for sequential `embed_batch` order, `BatchStrategy` defaults/overrides, Gemini/OpenAI-compatible key enforcement, placeholder Ollama no-key branch behavior if introduced here, endpoint fallback compatibility, and health strategy output.

### GREEN

- [ ] Add `BatchStrategy` and default `EmbeddingProvider::embed_batch` / `batch_strategy` methods in `crates/providers/src/lib.rs`.
- [ ] Update `crates/providers/src/gemini.rs` and `crates/providers/src/openai.rs` to compile with the extended trait and report the selected strategy.
- [ ] Refactor `create_provider` in `crates/cli/src/commands/mod.rs` so provider selection happens before API-key validation.
- [ ] Extend `crates/config/src/lib.rs` with optional provider fields: `endpoint`, `dimensions`, `device`, `batch_size`, and `workspace`, preserving `ingest.embedding_endpoint` as compatibility fallback where needed.
- [ ] Update `crates/cli/src/commands/health.rs` to include provider id/model and batch strategy without requiring live Ollama support yet.

### VERIFY / REFACTOR

- [ ] Run focused provider/config/health tests, then `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.

## PR 8 — Ollama provider + health details

**Depends on:** PR 7.  
**Finish:** `provider=ollama` creates a no-key local HTTP provider using `/api/embed`, supports native batch, and reports actionable health.  
**Rollback:** Remove `ollama` factory branch and module; Gemini/OpenAI-compatible remain unchanged.

### RED

- [ ] Add unit tests in new `crates/providers/src/ollama.rs`, `crates/providers/src/lib.rs`, `crates/cli/src/commands/mod.rs`, and `crates/cli/src/commands/health.rs` for no-key creation, default endpoint `http://localhost:11434`, request body to `/api/embed`, response parsing order/count, `embed` delegating to batch, native `BatchStrategy`, unreachable endpoint health message, and existing provider regression behavior.

### GREEN

- [ ] Add `crates/providers/src/ollama.rs` with HTTP client, endpoint/model validation, `embed_batch` native request, `embed` single-text wrapper, response parsing, and actionable errors.
- [ ] Export `pub mod ollama;` from `crates/providers/src/lib.rs` and add the `ollama` factory branch in `crates/cli/src/commands/mod.rs` with no API-key requirement.
- [ ] Resolve Ollama endpoint from `embedding.endpoint` first, compatibility fallback if appropriate, then default local endpoint.
- [ ] Extend `crates/cli/src/commands/health.rs` to report Ollama provider id, model, endpoint, connectivity status, latency when measurable, error text, and native batch strategy.
- [ ] Add an ignored live-test target or documented manual check only if it stays outside normal `cargo test` requirements.

### VERIFY / REFACTOR

- [ ] Run focused Ollama/provider/health tests, then `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.

## Final cross-slice verification

- [ ] Run `cargo fmt --check`.
- [ ] Run `cargo clippy -- -D warnings`.
- [ ] Run `cargo test`.
- [ ] Manually smoke-test representative commands after implementation: `cite tag set/get/rm`, `cite list --tag type:rfc`, `cite retrieve "query" --tag status:changed`, unchanged re-ingest, changed re-ingest, `cite check-docs`, and `cite health` for existing and Ollama provider configs.
