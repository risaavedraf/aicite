# Changelog

## v0.3.0 (2026-06-05)

### Breaking changes

- **DateTime fields in graph models** — `Topic.created_at` and `Concept.created_at` changed from `String` to `DateTime<Utc>` for type safety and consistency. External serialization format (`%Y-%m-%d %H:%M:%S`) is preserved via custom `sqlite_datetime_serde` module. See `crates/graph/src/types.rs` for migration details.
- **Removed duplicated ID fields** — `ScoredChunk` and `ChunkEmbeddingRecord` no longer carry string ID fields alongside typed IDs. Use `.id` (typed) with `.as_ref()` for `&str` when needed.

### New features

- **Typed string identifiers** — Introduced `DocumentId`, `ChunkId`, `CitationId`, `TraceId`, `EmbeddingBlobId`, and `SnapshotPointerId` newtypes in `common/src/types.rs`. All derives: `Display`, `From<String>`, `AsRef<str>`, `Serialize`, `Deserialize`, `PartialEq`, `Eq`, `Hash`, `Clone`, `Debug`. Ready for incremental adoption across all crates.
- **Snapshot pointer timestamp** — New migration `008_snapshot_pointer_updated_at.sql` adds `updated_at` column to `snapshot_pointer` table for tracking update time. Snapshot activation now sets/refreshes this timestamp.
- **Graph typed IDs** — Graph hierarchy types (`TopicId`, `ConceptId`) now use typed identifiers for compile-time ID safety.

### Improvements

- **DRY fixes (CodeRabbit)** — Consolidated duplicated code paths in CLI commands and engine modules.
- **Unwrap safety** — Replaced `.unwrap()` calls with proper error handling in `evaluate.rs` and other critical paths.
- **Integer cast safety** — Replaced `as u32` casts with `u32::try_from()` to prevent silent truncation.
- **CI stacked PR checks** — Added workflow support for validating stacked pull requests.

### Quality

- All tests pass, 0 clippy warnings, clean formatting.
- Full SDD artifacts in `openspec/changes/active/error-remediation-v3/`.
- Judgment Day dual adversarial review completed and approved.

## v0.2.4 (2026-06-02)

### Critical fixes

- **UTF-8 bytes-vs-chars confusion** — Fixed `str::len()` (byte count) being used instead of `chars().count()` (character count) in 5 files across `common`, `graph`, and `ingest` crates. This caused runtime panics on non-ASCII filenames, corrupted chunk offsets for multi-byte text (Japanese, emoji, accented), and inflated `total_chars` metadata. Added `char_len()` and `char_truncate()` helpers in `common` crate.
- **FK enforcement disabled** — Added `PRAGMA foreign_keys = ON` in `Database::open()` and `Database::open_memory()`. Foreign key constraints in schema were previously decorative, allowing orphan rows.
- **heading_parser double-increment bug** — Fixed `char_offset` being incremented twice for lines inside code blocks, causing incorrect offsets for all documents with fenced code blocks.

### Security

- **Empty API key rejection** — Providers (Gemini, OpenAI-compatible) now reject empty API keys at construction with a clear `ConfigError` message. CLI `create_provider` replaced `.unwrap_or_default()` with actionable error mentioning `CITE_API_KEY`.
- **Production mode guard wired** — `check_ingest_allowed()` is now called in the CLI ingest command, blocking ingest in `Production` and `PublicPackagedDemo` modes. Previously defined but never called (dead code).
- **Composite rate limit key** — Rate limiting now uses `provider_id:model_id` instead of just `provider_id`, giving each model its own rate limit bucket per FR-109.

### Improvements

- **Config field consolidation** — Removed confusing duplicate `min_chunk_size_chars` field, consolidated into `min_chunk_chars`. Timeout config (`embedding_timeout_secs`) now wired to provider constructors.
- **Silenced error elimination** — Replaced `.ok()` with `.optional()` in `snapshots.rs` for proper DB error handling. Cleanup failures in engine now logged to stderr.
- **Integer cast safety** — Replaced `as u32` casts with `u32::try_from()` in `storage/src/util.rs` and `storage/src/embeddings.rs` to prevent silent truncation.
- **Provider unwrap consistency** — Added `CommandContext::provider()` helper returning `Result`, replacing `.unwrap()` in 3 CLI commands.
- **Graph robustness** — Fixed duplicate heading boundary matching in `hierarchy.rs` using sequential consumption instead of `find()`.
- **CiteError PartialEq** — Added `PartialEq` derive to `CiteError` enum for cleaner test assertions.
- **Unused deps removed** — Removed `tokio` and `tracing` from `providers/Cargo.toml` (never used).

### Quality

- 308 tests pass, 0 clippy warnings, clean formatting.
- Full SDD artifacts in `openspec/changes/error-remediation/`.
- Error tracking in `openspec/reports/error-tracking.md`.

## v0.2.3 (2026-06-02)

### New features

- **Trace provenance** — `cite trace` now stores and displays the embedding model and provider used at retrieval time. New migration `007_trace_provenance.sql` adds `embedding_model_registry_id` and `provider` columns to the `traces` table.
- **Offline trace retrieval** — `cite trace` no longer requires an active embedding provider. Provenance data is read from the database, enabling trace inspection in offline or degraded-provider scenarios.
- **CLI overrides** — New global flags `--data-dir <path>` and `--runtime-mode <mode>` for runtime configuration without config files or env vars.
- **`RuntimeMode` parsing** — `RuntimeMode` now implements `FromStr` for reusable validation across CLI and env overrides.

### Maintenance

- Renamed `docs/` to `openspec/` to reflect SDD artifact store convention.

## v0.2.2 (2026-05-29)

### Critical fixes

- **unwrap() in production code** — Replaced `.unwrap()` calls on `Option<Provider>` in `trace.rs` and `search.rs` with `match` that returns `ExitCode::Validation` and a descriptive error. Prevents CLI panics when provider is unavailable.

### Improvements

- **Refactored `build_context`** — Extracted `validate_corpus_ready()`, `build_citations_from_ranked()`, and `persist_trace()` helpers from the 212-line monolith. Public API unchanged, all 17 context tests pass.
- **DRY fix: API key resolution** — Extracted `resolve_api_key()` shared helper in `commands/mod.rs`, eliminating duplicated env var precedence chain in `health.rs`.
- **Doc comments on public APIs** — Added `///` documentation with examples to all public types in `common`, `retrieval`, `graph`, and `storage` crates (40+ APIs documented).
- **Newtype wrappers** — Added `DocumentId`, `ChunkId`, `TraceId` in `common/src/types.rs` with `Display`, `From<String>`, `AsRef<str>`, `Serialize`, `Deserialize` and standard derives. Ready for incremental adoption.
- **12 doc tests** — Added compilable doc examples across `common`, `retrieval`, `graph`, and `storage` crates.

### Code quality

- Ran comprehensive code quality review with Clean Code, Rust Idioms, and GitHub Structure references.
- All 260 tests pass, zero compiler warnings, clippy clean.

## v0.2.1 (2026-05-28)

### Fixes

- **rustfmt consistency** — Removed unnecessary braces in `Commands::Setup` match arm to fix CI formatting check.

## v0.2.0 (2026-05-29)

### New features

- **TOML config file support** — Configuration can now be loaded from `~/.config/cite/config.toml` (XDG) with precedence: CLI flags > env vars > config file > defaults. Override path with `CITE_CONFIG` env var or `--config` flag.
- **`cite setup` command** — Setup wizard for first-time configuration. Supports `--provider`, `--api-key`, and `--non-interactive` flags for CI/scripts.
- **Enhanced `cite health` diagnostics** — Now reports API key status (masked), provider reachability with latency, data directory writability, and database statistics (document/chunk counts).
- **`install.sh` one-command install** — `curl -sSf .../install.sh | sh` detects OS/arch, downloads the correct binary, and offers to run setup.
- **`CITE_API_KEY` alias** — Shorter alias for `CITE_EMBEDDING_API_KEY`. Deprecation notice shown when both are set.

### Improvements

- **DRY refactor** — Extracted `resolve_data_dir()` (12 copies → 1) and `create_provider()` (5 copies → 1) to shared CLI utilities.
- **API key in config file** — `api_key` field in `[provider]` section, used as fallback when env vars are not set.

### Dependencies

- Added `dialoguer` for interactive terminal prompts.

## v0.1.0 (2026-05-28)

Initial release with:
- Document ingest (markdown, PDF, text)
- Vector search with embedding providers (Gemini, OpenAI-compatible)
- Context pack generation with citations
- Hierarchical retrieval (topics/concepts)
- Golden dataset evaluation
- CLI with 12 commands
