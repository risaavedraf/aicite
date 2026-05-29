# Proposal — Installation & Setup UX (RFC Implementation + v0.2.0 Release)

## Why now

Phase 9 resolved installation pathways and runtime migration, but the user onboarding experience remains manual. The RFC (`docs/rfc-install-setup-ux.md`) defines a guided flow that was never implemented. With Phase 11 (hierarchical retrieval) complete, the CLI is feature-rich enough to justify a polished install/setup experience before broadening adoption.

Additionally, the workspace version is still `0.1.0` despite Phases 10-12 shipping significant features. A v0.2.0 release with proper install UX is the natural milestone.

## In scope

### RFC Implementation
1. **TOML config file support** — `~/.config/cite/config.toml` (XDG), precedence: CLI flags > env vars > file > defaults
2. **Enhanced health diagnostics** — `cite health --json` with API key status, provider reachability, DB stats
3. **Setup wizard** — `cite setup` interactive flow: provider selection, API key input, connection test, config save
4. **Non-interactive setup** — `cite setup --provider gemini --api-key $KEY --non-interactive`
5. **Install script** — `install.sh` at repo root, detects OS/arch, downloads binary, offers setup

### Prerequisite Refactor
6. **Extract shared helpers** — `resolve_data_dir()` (12 copies) and `create_provider()` (5 copies) to shared CLI utility

### v0.2.0 Release
7. **Version bump** — workspace `0.1.0` → `0.2.0`
8. **CHANGELOG.md** — v0.2.0 entry summarizing all features since v0.1.0
9. **Git tag** — `v0.2.0` on GitHub after verification

## Out of scope

- Package manager manifests (Scoop, Homebrew, apt) — separate effort
- GUI installer
- Full TUI with panels/mouse
- OS keychain integration for API keys
- Hierarchical graph changes (Phases 10-11 already done)
- Agent UX changes (Phase 12 already done)

## Affected areas

| Area | Files |
|------|-------|
| Config crate | `crates/config/src/lib.rs` — implement `FileConfig::load()`, add `api_key` to `EmbeddingConfig` |
| CLI commands | `crates/cli/src/commands/setup.rs` (new), `crates/cli/src/commands/health.rs` (expand) |
| CLI shared | `crates/cli/src/commands/mod.rs` — extract `resolve_data_dir()`, `create_provider()` |
| CLI main | `crates/cli/src/main.rs` — register `Setup` command, wire `--config` flag |
| CLI deps | `crates/cli/Cargo.toml` — add `dialoguer` |
| Workspace | `Cargo.toml` — add `dialoguer` to workspace deps, version bump |
| Install script | `install.sh` (new, repo root) |
| Docs | `docs/installation.md` (update), `CHANGELOG.md` (new) |

## Open decisions

| Decision | Recommendation | Rationale |
|----------|---------------|-----------|
| `cite health` vs `cite setup --check` | Keep `cite health` as primary, `setup --check` as alias | Backward compatible, avoids breaking existing workflows |
| API key in config file | Yes, with `chmod 600` warning | Simpler than OS keychain, acceptable for CLI tool |
| `CITE_API_KEY` as alias | Yes, with deprecation notice | Better DX, low cost |
| `install.sh` auto-run setup | Ask "Run setup now? [Y/n]" | Balances convenience vs `curl \| sh` trust concerns |

## Risks and mitigations

- **Risk:** Config file format becomes a contract once users adopt it.
  **Mitigation:** Document format stability commitment, version the schema.

- **Risk:** `dialoguer` may not work in non-TTY environments.
  **Mitigation:** `--non-interactive` flag for CI/scripts.

- **Risk:** Provider reachability test in `--check` requires real API call.
  **Mitigation:** Graceful failure with clear message, no panic.

- **Risk:** Scope creep (~530 LOC estimated).
  **Mitigation:** Strict slice boundaries, ask-always PR strategy, 300-line budget.

## Rollback plan

- Each slice is independent and can be reverted without breaking others.
- Version bump is the last slice; reverting it doesn't affect functionality.
- Config file support degrades gracefully to env vars if TOML parsing fails.

## Acceptance criteria

1. `curl -sSf .../install.sh | sh` installs cite binary to PATH on Linux/macOS.
2. `cite setup` guides new user through provider config with connection test.
3. `cite setup --non-interactive --provider gemini --api-key $KEY` works in CI.
4. `cite health --json` reports API key status, provider reachability, DB stats.
5. Config file at `~/.config/cite/config.toml` is loaded with correct precedence.
6. `resolve_data_dir()` and `create_provider()` exist once, not 12/5 times.
7. Workspace version is `0.2.0` with git tag `v0.2.0`.

## Proposed slices

| Slice | Content | Est. Lines | PR |
|-------|---------|------------|-----|
| 0 | Extract shared helpers (refactor, zero behavior change) | ~50 | PR-0 |
| 1 | TOML config file support | ~120 | PR-1 |
| 2 | Enhanced health diagnostics | ~80 | PR-2 |
| 3 | Setup wizard (`cite setup`) | ~150 | PR-3 |
| 4 | install.sh at repo root | ~80 | PR-4 |
| 5 | v0.2.0 release (version bump + changelog + tag) | ~30 | PR-5 |
