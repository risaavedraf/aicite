# Explore: v0.4.0-tags-lifecycle-ollama

## 1. Current SQLite schema and migration needs

Current migrations are registered in `crates/storage/src/migrations/mod.rs` through version 8:
- `001_initial.sql`: `documents`, `chunks`, `embeddings`, `traces`
- `006_hierarchy.sql`: `topics`, `concepts`, `semantic_links`, plus `chunks.topic_id` and `chunks.concept_id`

Current important tables:
- `documents`: `document_id`, `display_name`, `file_path TEXT NOT NULL`, `file_type`, `file_size_bytes`, pipeline status/retry/error fields, `created_at`, `updated_at`
- `chunks`: `chunk_id`, `document_id`, optional `section_id`, `chunk_index`, text/page/offsets, `created_at`, and later nullable `topic_id`/`concept_id`
- `embeddings`: `chunk_id`, vector blob, `model_id`, `provider_id`

Needed migration:
- Add migration `009_tags_lifecycle.sql` (name tentative).
- Create `tags` table:
  - `tag_id TEXT PRIMARY KEY`
  - `entity_id TEXT NOT NULL`
  - `entity_type TEXT NOT NULL` constrained logically to `chunk` / `document`
  - `key TEXT NOT NULL`
  - `value TEXT NOT NULL`
  - `created_at TEXT NOT NULL DEFAULT (datetime('now'))`
  - `UNIQUE(entity_id, entity_type, key, value)` is safer than RFC’s `UNIQUE(entity_id, key, value)` because IDs may not be globally typed forever.
- Indexes:
  - `idx_tags_entity ON tags(entity_type, entity_id)`
  - `idx_tags_key ON tags(key)`
  - `idx_tags_key_value ON tags(key, value)`
  - Consider `idx_tags_filter ON tags(entity_type, key, value, entity_id)` for retrieval filters.
- Add lifecycle columns to `documents`:
  - `source_hash TEXT`
  - `ingested_at TEXT`
  - `file_modified_at TEXT`
- Existing `created_at`/`updated_at` are not enough: they track DB row creation/status changes, not source content freshness.
- Current `documents.file_path TEXT NOT NULL` is okay for v0.4.0 physical ingest, but conflicts with future virtual docs/notes.

Storage extension points:
- Add `storage/src/tags.rs` module and export it from `storage/src/lib.rs`.
- Add DB helpers for set/get/remove tags and list/filter by tag.
- Retrieval filtering should likely happen in SQL before ranking, by restricting candidate chunks/documents.

## 2. Provider trait and `embed_batch` / `BatchStrategy`

Current provider trait in `crates/providers/src/lib.rs`:

```rust
pub trait EmbeddingProvider {
    fn embed(&self, text: &str) -> Result<Embedding, CiteError>;
    fn model_id(&self) -> &str;
    fn provider_id(&self) -> &str;
}
```

Existing providers:
- `GeminiProvider` in `crates/providers/src/gemini.rs`
- `OpenAICompatibleProvider` in `crates/providers/src/openai.rs`

Needed provider changes:
- Add `BatchStrategy` enum in `providers/src/lib.rs`:
  - `Native`
  - `RateLimited { max_concurrent: usize, delay_ms: u64 }`
  - `Sequential`
- Extend trait with default methods:
  - `embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, CiteError>` defaulting to sequential `embed`.
  - `batch_strategy(&self) -> BatchStrategy` defaulting to `Sequential`.
  - Optional future metadata: dimensions/is_local, but keep v0.4.0 minimal unless required by health.
- Set Gemini strategy to `RateLimited` or default sequential plus strategy override.
- OpenAI-compatible may support native batch, but current request type is single string. Native batch requires changing request/response parsing.
- Ollama should implement native batch against `/api/embed`.

Important factory issue:
- `crates/cli/src/commands/mod.rs::create_provider` resolves/requires API key before matching provider type.
- Ollama must bypass API key requirement.
- Config currently stores provider/model/api_key under `EmbeddingConfig`, while endpoint lives under `IngestConfig.embedding_endpoint`; RFC wants provider endpoint/dimensions/device/batch_size/workspace. This likely needs config expansion.

## 3. Ingest pipeline hooks for auto-tags and lifecycle metadata

Current ingest flow is in `crates/engine/src/ingest.rs`:
1. Validate file.
2. Acquire `ingest_pipeline` lock.
3. Derive display name.
4. Generate new document ID every time.
5. Insert `Document`.
6. Mark processing.
7. Extract text.
8. Chunk.
9. Insert chunks.
10. Embed chunks one by one with `provider.embed`.
11. Insert embeddings.
12. Optionally build hierarchy.
13. Mark ready and update chunk count.

Hook points:
- Lifecycle metadata:
  - Compute hash and file mtime after validation/before document insert.
  - Store `source_hash`, `ingested_at`, `file_modified_at` on `documents`.
  - Current `Document` struct in `common/src/types.rs` must gain these fields or storage functions need separate update helpers.
- Change detection:
  - Current ingest creates a brand-new document ID and does not look up existing documents by file path.
  - Need a DB helper like `get_document_by_file_path`.
  - On re-ingest:
    - If same source hash: skip or return existing status.
    - If different: re-chunk/re-embed and set `status:changed` tag.
  - “Status” here is a tag, not `DocumentStatus`; avoid confusing with pipeline status.
- Auto-tags:
  - After document insert and chunk insert, assign engine-managed tags:
    - document: `workspace:<name>`, `source_kind:document`, maybe `type:<path-category>`
    - chunks: same inherited/leaf tags as needed for retrieval semantics.
  - Path mappings from RFC:
    - `openspec/prd/*` → `type:prd`
    - `openspec/specs/*` → `type:spec`
    - `openspec/architecture/*` → `type:architecture`
    - `openspec/guides/*` → `type:guide`
    - `openspec/rfc/*` → `type:rfc`
- Reserved key enforcement:
  - Enforce in storage or engine tag service for user-driven tag commands.
  - Engine itself must still be able to write reserved keys.

## 4. CLI structure for tags and tag filters

CLI root is `crates/cli/src/main.rs`, with subcommands in `crates/cli/src/commands`.

Current command enum includes:
- `Health`
- `Setup`
- `Ingest`
- `List`
- `Get`
- `Retry`
- `Search`
- `Retrieve`
- `Context`
- `Read`
- `Trace`
- `Refresh`
- `Evaluate`
- `Workspace`
- `CheckDocs`

Needed additions:
- Add `Tag(commands::tag::TagArgs)` to `Commands`.
- Add `pub mod tag;` to `crates/cli/src/commands/mod.rs`.
- Implement nested Clap subcommands:
  - `cite tag set <entity_id> <key:value>...`
  - `cite tag get <entity_id>`
  - `cite tag rm <entity_id> <key:value>`
- Entity type ambiguity:
  - RFC command omits entity type. Current IDs are prefixed (`doc_`, `chunk_`) but not all future IDs may be.
  - Either infer from ID prefix or add optional `--entity-type document|chunk`.
- Add `--tag` filters:
  - `search`, `retrieve`, `context`: currently share `validate_retrieval_scope` for `--flat`, `--topic`, `--concept`.
  - `list`: currently has no args and calls `ingest::list_documents`.
  - Need to convert `List` to `List(commands::list::ListArgs)` if adding `--tag`.
  - Prefer shared parser type for tag filters, e.g. `Vec<String>` parsed into `{ key, value: Option<String> }`.
- Retrieval engine signatures currently accept topic/concept filters only. Need to add tag filters to:
  - CLI args
  - engine request struct (`RetrievalRequest`)
  - `fetch_candidates`
  - storage embedding query methods

`check-docs`:
- Parser currently extracts fenced code blocks and cite commands only.
- It does not inspect markdown comments.
- Add parsing for nearest preceding `<!-- tag:status=planned -->` or block-associated tags.
- `verify_command` can return planned/skipped/warning instead of outdated for planned commands, but `CheckStatus` may need extension or mapping.

## 5. Suggested scope split under 400 changed lines

The original plan says:
- PR 1: tags + lifecycle (~400 LOC)
- PR 2: Ollama provider (~540 LOC)

Exploration suggests PR 1 is likely tight because it touches migrations, storage helpers, CLI, retrieval filters, ingest, lifecycle, and tests.

Recommended split if enforcing 400-line review budget strictly:

### PR 1 — Minimal tag foundation + lifecycle schema
Include:
- Migration for `tags` and lifecycle document columns.
- Storage tag helpers: set/get/rm, basic tests.
- Reserved key validation helper.
- `cite tag set/get/rm` CLI.
- No retrieval filtering yet, or only storage-level tests for filtering.

Defer:
- `--tag` filters on all retrieval commands.
- path auto-tags.
- change detection behavior.
- check-docs planned parsing.

### PR 2 — Tag usage in ingest/retrieval + lifecycle behavior
Include:
- `--tag` on `search`, `retrieve`, `context`, `list`.
- Candidate filtering via tags before vector ranking.
- Path auto-tags and workspace detection.
- `source_hash`, `ingested_at`, `file_modified_at` population.
- Re-ingest hash comparison and `status:changed` tag.
- `check-docs` markdown tag parsing.

### Separate provider PR(s)
Ollama is logically independent and should not be bundled with tags/lifecycle if the 400-line budget is real:
- Provider PR A: trait `embed_batch` + `BatchStrategy`, factory refactor to allow no-key local providers.
- Provider PR B: `OllamaProvider`, config fields, health output. If small enough, combine; otherwise split.

## 6. Key unknowns to resolve in proposal/design

- Should tag inheritance be physically stored as document tags, or computed from chunk tags? RFC says documents inherit automatically; physical storage simplifies list/filter but needs sync rules.
- How exactly should re-ingest behave: skip unchanged, replace old chunks, or create a new document version?
- Should `status:changed` be added to document, chunks, or both?
- What is the authoritative workspace name source in config? Current config has no `workspace` field.
- Should `source_kind:document` be stored on document only, chunks only, or both for retrieval filtering?
- Should topic/concept CLI filters remain during transition, or be deprecated after tag filters land?
