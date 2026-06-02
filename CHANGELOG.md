# Changelog

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
