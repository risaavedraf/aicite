# Design — Installation & Setup UX

## Architecture decisions

### 1. Config file layer: extend existing `Config::load()` pipeline

The config crate already has a `defaults() → FileConfig::load() → EnvOverrides::load() → merge()` pipeline. `FileConfig::load()` is a stub returning `None`. The design fills in this stub rather than adding a new layer.

```
defaults()          → Config with all defaults
    ↓
FileConfig::load()  → Parse TOML, return partial Config (new)
    ↓
EnvOverrides::load() → Override from env vars (existing)
    ↓
CLI flag overrides   → Override from --config, --provider, etc. (new)
    ↓
merge()             → Final Config (existing)
```

### 2. API key storage: config file with restrictive permissions

- `api_key` field added to `EmbeddingConfig` (Optional<String>)
- On write (via `setup`), file is created with 0600 permissions on Unix
- On read, key is loaded into memory, never logged or printed in full
- `cite health` shows masked key (last 4 chars only)

### 3. Health diagnostics: expand existing command

Rather than creating a new `setup --check` command, expand `cite health`:
- Add `config_path`, `api_key_status`, `provider_status`, `data_dir`, `database` fields
- `cite setup --check` becomes an alias that delegates to the same code
- Keeps backward compatibility

### 4. Setup wizard: new command with `dialoguer`

New `crates/cli/src/commands/setup.rs`:
- Uses `dialoguer::Select` for provider choice
- Uses `dialoguer::Password` for masked API key input
- Uses `dialoguer::Confirm` for overwrite confirmation
- Non-interactive mode via `--provider`, `--api-key`, `--non-interactive` flags
- Connection test calls `provider.embed("test connection")` 

### 5. Shared helpers: extract to `commands/mod.rs`

Move to `crates/cli/src/commands/mod.rs`:
- `resolve_data_dir(config: &Config) -> PathBuf`
- `create_provider(config: &Config) -> Result<Box<dyn EmbeddingProvider>, CiteError>`

All command modules import from `super::` instead of defining locally.

### 6. Install script: standalone bash at repo root

- Extracted from `docs/installation.md` template
- Version reads from `CITE_VERSION` env var or defaults to `0.2.0`
- Post-install offers `cite setup`

## File changes by slice

### Slice 0 — Shared helpers (refactor)

| File | Change |
|------|--------|
| `crates/cli/src/commands/mod.rs` | Add `resolve_data_dir()`, `create_provider()` |
| `crates/cli/src/commands/context.rs` | Remove local copies, use `super::` |
| `crates/cli/src/commands/search.rs` | Same |
| `crates/cli/src/commands/retrieve.rs` | Same |
| `crates/cli/src/commands/ingest.rs` | Same |
| `crates/cli/src/commands/trace.rs` | Same |
| `crates/cli/src/main.rs` | Use `commands::resolve_data_dir()` |

### Slice 1 — TOML config file

| File | Change |
|------|--------|
| `crates/config/src/lib.rs` | Implement `FileConfig::load()`, add `api_key` to `EmbeddingConfig`, add `CITE_API_KEY` alias logic |
| `crates/cli/src/main.rs` | Wire `--config` flag to `Config::load()` |

### Slice 2 — Enhanced health

| File | Change |
|------|--------|
| `crates/cli/src/commands/health.rs` | Expand output with API key, provider, DB stats |
| `crates/cli/src/main.rs` | Add `setup --check` alias routing |

### Slice 3 — Setup wizard

| File | Change |
|------|--------|
| `crates/cli/src/commands/setup.rs` | New file: interactive + non-interactive setup |
| `crates/cli/src/commands/mod.rs` | Add `pub mod setup;` |
| `crates/cli/src/main.rs` | Register `Setup` command |
| `crates/cli/Cargo.toml` | Add `dialoguer` dependency |
| `Cargo.toml` (workspace) | Add `dialoguer` to workspace deps |

### Slice 4 — install.sh

| File | Change |
|------|--------|
| `install.sh` | New file at repo root |

### Slice 5 — v0.2.0 release

| File | Change |
|------|--------|
| `Cargo.toml` | Bump `version = "0.2.0"` |
| `CHANGELOG.md` | New file with v0.2.0 entry |
| `install.sh` | Update default version to 0.2.0 |
