# Tasks — Installation & Setup UX

## Review Workload Forecast

| Field | Value |
|---|---|
| Estimated changed lines | ~530 total |
| 300-line budget risk | Medium (each slice within budget) |
| Chained PRs recommended | Yes |
| Suggested split | PR-0 through PR-5 (one per slice) |
| Delivery strategy | ask-always |
| Chain strategy | by-slice |

## Task 0 — Slice 0: Extract shared helpers (refactor)

**Goal**: Eliminate `resolve_data_dir()` (12 copies) and `create_provider()` (5 copies) duplication.

**Files (allowlist)**
- `crates/cli/src/commands/mod.rs`
- `crates/cli/src/commands/context.rs`
- `crates/cli/src/commands/search.rs`
- `crates/cli/src/commands/retrieve.rs`
- `crates/cli/src/commands/ingest.rs`
- `crates/cli/src/commands/trace.rs`
- `crates/cli/src/main.rs`

**Steps**
1. Move `resolve_data_dir()` from `main.rs` to `commands/mod.rs` as `pub fn`
2. Move `create_provider()` from `context.rs` to `commands/mod.rs` as `pub fn`
3. Update all 6 command modules to use `super::resolve_data_dir()` and `super::create_provider()`
4. Remove duplicate definitions
5. Update `main.rs` to use `commands::resolve_data_dir()`

**Verify**
- `cargo build --workspace`
- `cargo test --workspace`
- `grep -rn "fn resolve_data_dir" crates/cli/src/` → 1 result
- `grep -rn "fn create_provider" crates/cli/src/` → 1 result

---

## Task 1 — Slice 1: TOML config file support

**Goal**: Implement `FileConfig::load()` to read TOML config with XDG paths.

**Files (allowlist)**
- `crates/config/src/lib.rs`
- `crates/cli/src/main.rs`

**Steps**
1. Add `api_key: Option<String>` to `EmbeddingConfig` struct
2. Define `FileConfig` struct matching TOML schema (provider, retrieval, data sections)
3. Implement `FileConfig::load()`:
   - Resolve path from `CITE_CONFIG` env var or XDG default
   - Read file, parse TOML into `FileConfig`
   - Return `None` if file not found (graceful degradation)
4. Update `merge()` to apply file config between defaults and env overrides
5. Add `CITE_API_KEY` alias in `EnvOverrides::load()` with deprecation notice
6. Wire `--config` CLI flag to `Config::load(path: Option<&str>)`
7. Add tests for TOML parsing, precedence order, missing file

**Verify**
- `cargo test -p config`
- `cargo clippy --workspace -- -D warnings`
- Manual: create `~/.config/cite/config.toml`, run `cite health --json`

---

## Task 2 — Slice 2: Enhanced health diagnostics

**Goal**: Expand `cite health` with API key, provider reachability, DB stats.

**Files (allowlist)**
- `crates/cli/src/commands/health.rs`
- `crates/cli/src/main.rs` (alias routing only)

**Steps**
1. Expand `HealthOutput` struct with: `config_path`, `api_key_status`, `provider_status`, `data_dir`, `database`
2. Add API key check: read from config, show masked or "missing"
3. Add provider reachability test: `provider.embed("test")`, measure latency
4. Add DB stats: open DB, count documents and chunks
5. Add data dir writable check
6. Make `cite setup --check` alias route to same function
7. Handle graceful failures (no API key → skip provider test, DB not found → report)

**Verify**
- `cargo test -p cli`
- `cargo run --bin cite -- health --json`
- `cargo run --bin cite -- setup --check --json`

---

## Task 3 — Slice 3: Setup wizard

**Goal**: Interactive `cite setup` command for first-time configuration.

**Files (allowlist)**
- `crates/cli/src/commands/setup.rs` (new)
- `crates/cli/src/commands/mod.rs`
- `crates/cli/src/main.rs`
- `crates/cli/Cargo.toml`
- `Cargo.toml` (workspace deps)

**Steps**
1. Add `dialoguer = "0.11"` to workspace deps and cli/Cargo.toml
2. Create `setup.rs` with `SetupArgs` struct (provider, api-key, non-interactive flags)
3. Implement interactive flow:
   - `dialoguer::Select` for provider (gemini/openai/custom)
   - `dialoguer::Password` for API key
   - `create_provider()` + `embed("test connection")` for validation
   - `dialoguer::Confirm` for config location
   - Write TOML config file with 0600 permissions
4. Implement non-interactive flow: validate args, test connection, save
5. Handle existing config: ask overwrite or skip
6. Register `Setup` in `Commands` enum
7. Add tests for non-interactive mode

**Verify**
- `cargo test -p cli`
- Manual: `cargo run --bin cite -- setup` (interactive)
- Manual: `cargo run --bin cite -- setup --provider gemini --api-key test --non-interactive`

---

## Task 4 — Slice 4: install.sh

**Goal**: Standalone install script at repo root.

**Files (allowlist)**
- `install.sh` (new)

**Steps**
1. Extract script from `docs/installation.md` template
2. Update default version to `0.2.0`
3. Add "Run cite setup now? [Y/n]" prompt after install
4. Add checksum verification (optional, if release assets provide SHA256)
5. Make executable

**Verify**
- `bash install.sh` (dry-run or test on Linux/macOS)
- Script detects platform correctly
- `--help` flag shows usage

---

## Task 5 — Slice 5: v0.2.0 release

**Goal**: Version bump, changelog, git tag.

**Files (allowlist)**
- `Cargo.toml` (workspace version)
- `CHANGELOG.md` (new)
- `install.sh` (default version)

**Steps**
1. Bump `workspace.package.version` from `"0.1.0"` to `"0.2.0"` in root `Cargo.toml`
2. Create `CHANGELOG.md` with v0.2.0 entry:
   - New: TOML config file support
   - New: `cite setup` interactive wizard
   - New: Enhanced `cite health` diagnostics
   - New: `install.sh` for one-command install
   - New: `CITE_API_KEY` alias
   - Improved: Extracted shared CLI helpers (DRY)
3. Update `install.sh` default version to `0.2.0`
4. Commit all changes
5. Tag: `git tag v0.2.0`

**Verify**
- `cargo build --workspace` (version shows 0.2.0)
- `git tag -l v0.2.0` shows tag
- CHANGELOG.md exists with v0.2.0 entry
