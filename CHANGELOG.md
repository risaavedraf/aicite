# Changelog

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
