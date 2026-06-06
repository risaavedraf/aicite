# Design: Tags, Lifecycle Re-Ingest, and Ollama Embeddings

v0.4.0 adds a durable metadata foundation without replacing the existing retrieval stack in one large change. The design is additive: tags and lifecycle metadata are stored in SQLite, tag filters narrow candidates before vector ranking, re-ingest reuses source identity instead of duplicating documents, and Ollama becomes a local provider through the existing provider abstraction.

## Executive summary

| Area | Decision | Reviewer focus |
|---|---|---|
| Tag persistence | Add a `tags` table scoped by `(entity_type, entity_id)` for `document` and `chunk` entities. | Migration constraints and indexes. |
| Reserved keys | `workspace`, `type`, `session`, and `source_kind` are engine-managed. User tag commands cannot set/remove them. | Validation boundary between CLI/user paths and engine paths. |
| `status` semantics | `status` is a tag, but local-only and non-inheritable. | No document/chunk propagation, especially for `status:changed`. |
| `status:changed` | Forbidden on documents in v0.4.0; recomputed on changed chunks during successful re-ingest. | Stale changed tags are cleared/replaced, not accumulated. |
| Descriptive tags | Store locally on both documents and chunks when they should filter at both scopes. | Auto-tags appear at both scopes except local-only keys. |
| Retrieval filtering | Apply tag filters before final vector ranking; keep legacy `topic`/`concept` filters unchanged. | SQL candidate filtering and AND semantics. |
| List filtering | `cite list --tag` filters document-local tags only. | No inference from chunk tags. |
| Re-ingest | Look up existing physical documents by canonical source path; skip unchanged hash; process changed hash. | No duplicate active docs for same path. |
| Provider batch | Add `embed_batch` default sequential fallback plus `BatchStrategy`. | Backward compatibility for Gemini/OpenAI-compatible. |
| Ollama | Add no-key local HTTP provider using `/api/embed`, default endpoint `http://localhost:11434`. | Factory no longer requires API key before provider selection. |
| Delivery | Ship in review-safe slices under the 400-line budget and ask before each slice. | Avoid bundled tag+lifecycle+provider mega-PR. |

## Current implementation shape

Relevant existing code paths:

- Storage migrations are registered in `crates/storage/src/migrations/mod.rs` through version 8.
- Core document/chunk types live in `crates/common/src/types.rs`.
- Document storage maps `SELECT * FROM documents` in `crates/storage/src/documents.rs`.
- Ingest currently creates a new `doc_*` ID every time in `crates/engine/src/ingest.rs`.
- Retrieval candidates come from `crates/engine/src/retrieve.rs` and `crates/storage/src/embeddings.rs`.
- CLI retrieval scope validation is shared in `crates/cli/src/commands/mod.rs`.
- `cite list` currently has no args and lists all documents.
- Provider factory is in `crates/cli/src/commands/mod.rs::create_provider` and currently resolves an API key before provider matching.
- Provider trait is `crates/providers/src/lib.rs::EmbeddingProvider` with single-text `embed` only.
- `check-docs` parses code blocks but not markdown tag comments.

## Proposed architecture

```text
CLI
  ├─ tag command + --tag parsers
  ├─ list/search/retrieve/context pass TagFilter[]
  └─ health reports provider batch strategy / Ollama details

Engine
  ├─ tag service semantics: user vs engine-owned writes
  ├─ ingest lifecycle: source identity, hash skip, changed processing
  └─ retrieval: tag-aware candidate fetch before ranking

Storage
  ├─ migration 009_tags_lifecycle.sql
  ├─ tags.rs: set/get/remove/filter helpers
  ├─ documents.rs: lifecycle columns + source path lookup/update
  └─ embeddings.rs: tag-filtered candidate SQL

Providers
  ├─ EmbeddingProvider::embed_batch default fallback
  ├─ BatchStrategy enum
  ├─ ollama.rs HTTP provider
  └─ factory supports cloud key validation and local no-key providers
```

This keeps `packages/coding-agent` out of scope; all implementation work is in the Rust Cite crates under `crates/*`.

## Data model

### Migration 009

Add `crates/storage/src/migrations/009_tags_lifecycle.sql` and register it in `mod.rs`.

```sql
CREATE TABLE IF NOT EXISTS tags (
    tag_id TEXT PRIMARY KEY,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL CHECK (entity_type IN ('document', 'chunk')),
    key TEXT NOT NULL CHECK (length(trim(key)) > 0),
    value TEXT NOT NULL CHECK (length(trim(value)) > 0),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(entity_type, entity_id, key, value)
);

CREATE INDEX IF NOT EXISTS idx_tags_entity
    ON tags(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_tags_key
    ON tags(key);
CREATE INDEX IF NOT EXISTS idx_tags_key_value
    ON tags(key, value);
CREATE INDEX IF NOT EXISTS idx_tags_filter
    ON tags(entity_type, key, value, entity_id);

ALTER TABLE documents ADD COLUMN source_hash TEXT;
ALTER TABLE documents ADD COLUMN ingested_at TEXT;
ALTER TABLE documents ADD COLUMN file_modified_at TEXT;
CREATE INDEX IF NOT EXISTS idx_documents_file_path ON documents(file_path);
CREATE INDEX IF NOT EXISTS idx_documents_source_hash ON documents(source_hash);
```

Notes:

- `UNIQUE(entity_type, entity_id, key, value)` is intentional. It is safer than assuming IDs remain globally unique across entity kinds forever.
- Keep `documents.file_path TEXT NOT NULL` for v0.4.0 physical ingest. Virtual documents/notes are future scope.
- Lifecycle columns are nullable so the additive migration works against existing rows.

### Common types

Extend `Document` with nullable lifecycle fields:

- `source_hash: Option<String>`
- `ingested_at: Option<DateTime<Utc>>`
- `file_modified_at: Option<DateTime<Utc>>`

Add small tag domain types where they reduce parser bugs:

- `EntityType { Document, Chunk }`
- `Tag { key: String, value: String }`
- `TagFilter { key: String, value: Option<String> }`

Keep tag parsing flat. `key:value` is the CLI filter/mutation display form; markdown uses `tag:key=value` comments.

## Tag service and storage APIs

Create `crates/storage/src/tags.rs` and export `pub mod tags;` from storage.

Recommended storage helpers:

```rust
pub enum TagEntityType { Document, Chunk }
pub struct TagRecord { pub key: String, pub value: String }
pub struct TagFilter { pub key: String, pub value: Option<String> }

impl Database {
    pub fn set_tag_engine(&self, entity_type: TagEntityType, entity_id: &str, tag: &TagRecord) -> Result<(), CiteError>;
    pub fn set_tag_user(&self, entity_type: TagEntityType, entity_id: &str, tag: &TagRecord) -> Result<(), CiteError>;
    pub fn remove_tag_user(&self, entity_type: TagEntityType, entity_id: &str, tag: &TagRecord) -> Result<(), CiteError>;
    pub fn remove_tag_engine(&self, entity_type: TagEntityType, entity_id: &str, tag: &TagRecord) -> Result<(), CiteError>;
    pub fn list_tags(&self, entity_type: TagEntityType, entity_id: &str) -> Result<Vec<TagRecord>, CiteError>;
    pub fn clear_chunk_status_changed_for_document(&self, document_id: &str) -> Result<u64, CiteError>;
}
```

API rules:

- User mutation APIs validate reserved keys.
- Engine mutation APIs may write reserved keys.
- Both APIs reject document-local `status:changed`.
- Both APIs reject empty key/value and malformed mutation inputs.
- Mutations use `INSERT OR IGNORE` to make duplicate setting idempotent.
- `rm` requires exact `key:value`; key-only removal is not supported in v0.4.0 user commands.

Reserved key enforcement:

```text
reserved engine-managed keys: workspace, type, session, source_kind
local-only key: status
forbidden document-local pair: status:changed
```

`status` is not reserved: users may set meaningful local statuses such as `status:implemented` or `status:planned`. Its special behavior is non-inheritance and local-only filtering.

## CLI contract

### `cite tag`

Add `Commands::Tag(commands::tag::TagArgs)` and `crates/cli/src/commands/tag.rs`.

Supported shape:

```bash
cite tag set <entity_id> <key:value>...
cite tag get <entity_id>
cite tag rm <entity_id> <key:value>...
```

Entity type resolution:

1. Infer `doc_*` as document and `chunk_*` as chunk for current IDs.
2. Add optional `--entity-type document|chunk` to avoid future ambiguity and support tests.
3. If inference fails and no explicit type is supplied, return validation error.

### `--tag` filters

Add a shared parser in `commands/mod.rs` or a small `commands/tags.rs` helper:

- Mutation parser: exact `key:value` only.
- Filter parser: exact `key:value`; key-only `key` may be accepted for filters if implemented deliberately.
- Multiple `--tag` flags use AND semantics.

Add `#[arg(long = "tag")] pub tags: Vec<String>` to `search`, `retrieve`, `context`, and new `list::ListArgs`.

Convert main command from `List` to `List(commands::list::ListArgs)`.

## Retrieval and list filtering flow

### Retrieval commands

Flow for `search`, `retrieve`, and `context`:

1. CLI parses query, `--topic`, `--concept`, `--flat`, and `--tag`.
2. Shared validation preserves existing topic/concept rules.
3. Engine request gains `tag_filters: &'a [TagFilter]`.
4. Candidate fetch applies topic/concept filters as today and tag filters in SQL.
5. Vector ranking runs only over filtered candidates.
6. Result shape remains backward-compatible; tags do not need to be emitted in v0.4.0 unless tests require it.

Status behavior:

- Retrieval evaluates tag filters against chunk-local tags only.
- `--tag status:changed` matches a chunk only if that exact chunk has local `status:changed`.
- Parent document status never makes a chunk match.
- Sibling chunk status never makes a chunk match.

SQL pattern for AND filters:

```sql
WHERE d.status = 'ready'
  AND EXISTS (SELECT 1 FROM tags t WHERE t.entity_type='chunk' AND t.entity_id=c.chunk_id AND t.key=? AND (? IS NULL OR t.value=?))
  AND EXISTS (... one EXISTS per filter ...)
```

This avoids row multiplication and keeps AND semantics clear. Implement by constructing a small dynamic SQL fragment with bound parameters; do not interpolate user strings.

When hierarchy is enabled, keep the existing hierarchy query and add the same chunk-local tag `EXISTS` clauses. When hierarchy is disabled and topic/concept filters are present, preserve current empty-result behavior.

### `cite list --tag`

Flow:

1. CLI parses document tag filters.
2. Storage lists documents with document-local `EXISTS` clauses.
3. `status` filters are evaluated locally on documents only.
4. A document with a changed chunk is not listed for `--tag status:changed` unless a future feature explicitly adds status aggregation.

Because document-local `status:changed` is forbidden in v0.4.0, `cite list --tag status:changed` should normally return no results. That is correct and protects the local-only semantic rule.

## Ingest and re-ingest lifecycle

### Source identity

Use canonicalized source path as the v0.4.0 source identity.

Add storage helpers:

```rust
pub fn get_document_by_file_path(&self, path: &Path) -> Result<Option<Document>, CiteError>;
pub fn update_document_lifecycle(&self, document_id: &str, source_hash: &str, ingested_at: DateTime<Utc>, file_modified_at: Option<DateTime<Utc>>) -> Result<(), CiteError>;
pub fn replace_document_chunks_and_embeddings(...);
```

Path normalization should match whatever `insert_document` stores. Prefer canonical path after validation; if canonicalization fails, use the validated path consistently.

### Hash skip

Before acquiring expensive pipeline work:

1. Validate file.
2. Compute source hash from file bytes. Use SHA-256 or an existing project hashing crate if already present; store as stable lowercase hex.
3. Read file mtime when available.
4. Look up existing document by source path.
5. If existing document has identical `source_hash`, return an `IngestResult` for the existing doc without re-chunking/re-embedding.

Whether the lock is acquired before or after the lookup is an implementation detail, but the check/update must be race-safe enough for the current single-shot durable process model. The existing `ingest_pipeline` lock can guard the recheck before writing.

### Changed processing

If the source hash differs:

1. Reuse the existing `document_id` for that source path.
2. Mark pipeline status `processing`.
3. Extract, chunk, and embed the new content.
4. Compare new chunks to previous chunks to identify changed/new chunks.
5. Replace old chunks/embeddings in a transaction.
6. Store descriptive tags on the document and all new chunks.
7. Clear stale chunk-local `status:changed` for the document.
8. Add `status:changed` only to new chunks known to be changed.
9. Update `source_hash`, `ingested_at`, `file_modified_at`, `chunk_count`, and pipeline status `ready` after successful processing.

Rollback safety:

- Do not update lifecycle metadata or clear old chunks until the changed source has been successfully extracted/chunked/embedded.
- Prefer a transaction for delete-old/insert-new chunks, embeddings, tags, hierarchy rows, and lifecycle update.
- On failure, preserve the last ready representation when practical. If the current storage helpers make full atomic replacement too large for the first slice, keep current cleanup behavior but document the limitation in tests.

### Changed-chunk detection

Minimum v0.4.0 algorithm:

- Compare new chunk text to previous chunk text by stable content hash.
- A new chunk is unchanged if its text hash appears in the previous chunk set.
- A new chunk is changed if its text hash is absent from previous chunks.
- This handles unchanged chunks that shift index due to earlier edits better than index-only comparison.

Limitations:

- Duplicate identical chunks are ambiguous. Count occurrences by hash to avoid marking all duplicates unchanged when only one existed before.
- Chunk-boundary changes can mark more chunks as changed than a semantic diff would. This is acceptable if tags remain chunk-local and no document-level `status:changed` is synthesized.

### Auto-tags

Path-based tags are engine-owned and written locally to both document and chunks:

| Source path | Tag |
|---|---|
| `openspec/prd/*` | `type:prd` |
| `openspec/specs/*` | `type:spec` |
| `openspec/architecture/*` | `type:architecture` |
| `openspec/guides/*` | `type:guide` |
| `openspec/rfc/*` | `type:rfc` |

Also write:

- `source_kind:document` on document and chunks.
- `workspace:<name>` on document and chunks.

Workspace detection priority:

1. New config field `workspace`, if present.
2. Git repo root directory name, if cheap to detect.
3. Current working directory name.

Do not inherit or auto-propagate `status`.

## Check-docs markdown tag parsing

Extend `check-docs` parsing without changing default command verification.

Design:

1. Add a parser function that extracts markdown comments matching `<!-- tag:key=value -->` with line numbers.
2. Associate tags with the next Cite command code block when the tag comment is immediately adjacent or separated only by blank lines.
3. At minimum, recognize `status=planned` and `status=implemented`.
4. `status=planned` marks the command result as planned/warning rather than outdated when the command is unavailable.
5. `status=implemented` uses existing verification behavior.
6. Unknown tags are ignored for verification decisions.

Result model option:

- If adding `CheckStatus::Planned` is too broad for the first slice, map planned commands to `Warning` with detail `Planned command; verification skipped`. The spec only requires not reporting as outdated.

## Provider design

### Trait changes

In `crates/providers/src/lib.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchStrategy {
    Native,
    RateLimited { max_concurrent: usize, delay_ms: u64 },
    Sequential,
}

pub trait EmbeddingProvider {
    fn embed(&self, text: &str) -> Result<Embedding, CiteError>;
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, CiteError> {
        texts.iter().map(|text| self.embed(text)).collect()
    }
    fn batch_strategy(&self) -> BatchStrategy { BatchStrategy::Sequential }
    fn model_id(&self) -> &str;
    fn provider_id(&self) -> &str;
}
```

Keep dimensions/max-tokens out of the trait for this slice unless health needs them. This minimizes changes and preserves existing implementations.

Provider strategies:

| Provider | Strategy |
|---|---|
| Gemini | `RateLimited { max_concurrent: 1, delay_ms: 0 }` or default sequential if kept minimal |
| OpenAI-compatible | `Sequential` initially unless native batch response parsing is implemented |
| Ollama | `Native` |

Ingest can switch from per-chunk `embed` to `embed_batch` in a later slice or same provider slice if the diff is small. The trait default makes this safe.

### Factory API-key behavior

Refactor factory selection so API-key validation happens after provider type is known:

```text
provider=gemini             -> require key, create Gemini
provider=openai-compatible  -> require key, create OpenAI-compatible
provider=ollama             -> no key required, create Ollama with endpoint/model
unknown                     -> config error with supported provider list
```

This fixes the current issue where `create_provider` resolves an API key before matching provider type.

### Config

Add optional fields to `EmbeddingConfig` and env/file merge support:

- `endpoint: Option<String>`
- `dimensions: Option<usize>`
- `device: Option<String>`
- `batch_size: Option<usize>`
- `workspace: Option<String>` (or top-level config field if preferred later)

Compatibility note: current config also has `ingest.embedding_endpoint`. For v0.4.0, prefer `embedding.endpoint` as the provider endpoint, but keep reading `ingest.embedding_endpoint` as a fallback for OpenAI-compatible compatibility.

### Ollama provider

Add `crates/providers/src/ollama.rs`.

Defaults:

- endpoint: `http://localhost:11434`
- model: required from config
- API key: not required
- batch strategy: native

Request:

```http
POST /api/embed
{
  "model": "nomic-embed-text",
  "input": ["text 1", "text 2"]
}
```

Response parsing should accept Ollama's `embeddings` array and return one vector per input in order. `embed(text)` delegates to `embed_batch(&[text])` and returns the first vector.

Health:

- Create provider without API key.
- Measure a small `embed("test")` latency if reachable.
- Report provider id, model, endpoint, status, latency, error, and `batch_strategy`.
- If unreachable, use actionable text such as `Cannot reach Ollama at <endpoint>; is ollama serve running and is the model pulled?`.

## File change map

| Slice | Files |
|---|---|
| Tag schema | `crates/storage/src/migrations/009_tags_lifecycle.sql`, `crates/storage/src/migrations/mod.rs`, `crates/common/src/types.rs`, `crates/storage/src/documents.rs` |
| Tag storage/CLI | `crates/storage/src/tags.rs`, `crates/storage/src/lib.rs`, `crates/cli/src/main.rs`, `crates/cli/src/commands/mod.rs`, `crates/cli/src/commands/tag.rs` |
| List/retrieval filters | `crates/cli/src/commands/list.rs`, `search.rs`, `retrieve.rs`, `context.rs`, `crates/engine/src/retrieve.rs`, `crates/engine/src/context.rs`, `crates/storage/src/embeddings.rs` |
| Ingest lifecycle | `crates/engine/src/ingest.rs`, `crates/storage/src/documents.rs`, `crates/storage/src/chunks.rs`, `crates/storage/src/embeddings.rs`, `crates/storage/src/tags.rs` |
| Check-docs | `crates/cli/src/commands/check_docs.rs`, `crates/check-docs/src/parser.rs` if parser crate exposes command/block parsing |
| Provider trait/factory | `crates/providers/src/lib.rs`, `gemini.rs`, `openai.rs`, `crates/cli/src/commands/mod.rs`, `health.rs`, `crates/config/src/lib.rs` |
| Ollama | `crates/providers/src/ollama.rs`, `crates/providers/src/lib.rs`, provider tests/fixtures |

## Testing strategy

| Slice | Tests |
|---|---|
| Schema + tag storage | Migration version 9 runs on memory DB; duplicate tags are ignored; indexes/query helpers work; lifecycle columns are readable on old rows. |
| Reserved/status rules | User set/remove rejects reserved keys; engine set accepts reserved keys; document `status:changed` rejected for user and engine paths; `status` does not propagate. |
| Tag CLI | `set/get/rm` parse valid tags; malformed/key-only mutation rejected; entity inference and `--entity-type` behavior. |
| List filtering | Document-local AND filters; `list --tag status:changed` does not match documents with only changed chunks. |
| Retrieval filtering | `search/retrieve/context --tag` excludes non-matching chunks before ranking; multiple filters AND; `status` chunk-local; topic/concept filters still pass existing tests. |
| Ingest lifecycle | First ingest stores hash/timestamps; unchanged re-ingest skips chunk/embed writes; changed re-ingest reuses document ID; no duplicate active source path. |
| Changed chunks | New/changed chunk receives `status:changed`; unchanged known chunk does not; stale `status:changed` cleared on later successful ingest. |
| Auto-tags | OpenSpec path patterns create local tags on document and chunks; reserved validation does not block engine writes. |
| Check-docs | `<!-- tag:status=planned -->` adjacent to Cite command prevents outdated failure; `implemented` verifies; unknown tag ignored. |
| Provider trait | Default `embed_batch` preserves order; `batch_strategy` default is sequential; existing test providers compile. |
| Factory/config | Gemini/OpenAI-compatible still require keys; Ollama does not require key; endpoint default resolves. |
| Ollama | Unit tests with mocked HTTP for request/response and health failure; ignored live test for local Ollama. |

Run commands per project config:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

## Rollout and rollback

- Migration is additive. If later slices are rolled back, tags/lifecycle columns can remain unused.
- Tag filters are opt-in. Existing retrieval without `--tag` should behave as before.
- Existing `topic`/`concept` filters remain supported and are not reinterpreted as tags.
- If tag-filtered SQL has an issue, disable only the `--tag` option while preserving unfiltered vector retrieval.
- If lifecycle skip/update is risky, keep full-process ingest temporarily while still storing lifecycle metadata.
- If changed-chunk detection is uncertain, mark only chunks known to be changed/new and report/document limitations; never synthesize document `status:changed`.
- Ollama failure should not affect Gemini/OpenAI-compatible users. Factory branches isolate provider-specific validation.
- `embed_batch` default preserves sequential single-call behavior, so provider trait rollout can precede native batch usage.

## Review-safe slice plan

The SDD preflight requires `ask_always` and a 400 changed-line review budget. Do not implement all of v0.4.0 in one PR.

| Slice | Content | Gate |
|---|---|---|
| 1. Schema + core tag APIs | Migration 009, lifecycle fields in `Document`, tag storage helpers, validation helpers, unit tests. | Ask before apply. |
| 2. Tag CLI + list filters | `cite tag set/get/rm`, `cite list --tag`, document-local filter tests. | Ask before apply. |
| 3. Retrieval tag filters | `--tag` for search/retrieve/context, SQL candidate filtering, legacy topic/concept regression tests. | Ask before apply. |
| 4. Ingest lifecycle | source hash/timestamps, source path lookup, unchanged skip, auto-tags. | Ask before apply. |
| 5. Changed chunk status | changed-source replacement, chunk-local `status:changed`, stale cleanup tests. | Ask before apply. |
| 6. Check-docs tags | markdown tag parsing and planned-command behavior. | Ask before apply. |
| 7. Provider trait/factory | `embed_batch`, `BatchStrategy`, config/factory key behavior, health batch strategy. | Ask before apply. |
| 8. Ollama provider | `ollama.rs`, HTTP request/response, endpoint config/default, health details. | Ask before apply. |

If any slice forecasts over 400 changed lines, split it further before implementation.

## Open questions and risks

| Risk/question | Design stance |
|---|---|
| Exact transaction shape for changed re-ingest may be larger than one slice. | Prefer atomic replacement; if not feasible in first lifecycle slice, defer changed-marking to its own slice and keep old ready content on failure. |
| Workspace config location is not settled. | Add minimal `workspace: Option<String>` in config merge, or implement git/CWD fallback first. Do not block tag schema on this. |
| Key-only filter support is optional in spec. | Implement only if parser and SQL stay small; otherwise support exact `key:value` first and document key-only as deferred. |
| `list --tag status:changed` normally returns no rows due to document ban. | This is intentional. Future aggregation must be a new explicit command/API behavior. |
| Dynamic SQL for tag AND filters can become error-prone. | Build small helper that appends `EXISTS` clauses and bound parameters; unit test generated behavior via DB fixtures. |
| Ollama response formats can vary by version. | Keep parser focused on `/api/embed` documented response and make errors actionable. |
| Provider config currently stores endpoint under ingest. | Read new `embedding.endpoint` first, old `ingest.embedding_endpoint` as compatibility fallback. |
