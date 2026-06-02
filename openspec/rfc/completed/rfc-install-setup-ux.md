# RFC: Installation & Setup UX

## Status: Implemented in v0.2.0

> All features described in this RFC were implemented and released in v0.2.0:
> - `install.sh` one-command install script
> - `cite setup` interactive wizard
> - TOML config file support with XDG paths
> - Enhanced `cite health` diagnostics
>
> See [CHANGELOG.md](../../CHANGELOG.md) for release details.

## Problem

Currently, installing CITE CLI requires:
1. Manually downloading a binary
2. Moving it to a directory in PATH
3. Setting environment variables (`CITE_EMBEDDING_API_KEY`, `CITE_EMBEDDING_PROVIDER`)
4. Knowing which commands to run to verify

This is friction for new users. There's no guided flow from "I want to try this" to "I'm querying my documents".

## Goals

1. One-command install (binary in PATH)
2. Guided setup wizard for first-time configuration
3. Validation that everything works before leaving the user on their own
4. Support for non-interactive setup (CI/scripts)

## Non-goals

- Full TUI with panels/mouse (overkill for a CLI tool)
- Package manager manifests (Scoop, Homebrew — separate effort)
- GUI installer

## Proposed approach

### Install script (`install.sh`)

Minimal script that:
1. Detects OS + architecture
2. Downloads the correct binary from GitHub releases
3. Moves it to `/usr/local/bin` (or `$INSTALL_DIR`)
4. Runs `cite --version` to verify
5. Optionally runs `cite setup` if the user says yes

```bash
curl -sSf https://raw.githubusercontent.com/risaavedraf/aicite/main/install.sh | sh
```

The script does NOT configure anything — it only installs the binary.

### Setup command (`cite setup`)

Interactive wizard built into the CLI:

```
$ cite setup

  CITE CLI Setup
  ══════════════

  ? Embedding provider: (Use arrow keys)
    ❯ gemini
      openai

  ? API key: ********************************

  ✓ Testing connection...
  ✓ Embedding test successful (768 dimensions)

  ? Where to save config?
    ❯ ~/.config/cite/config.toml  (recommended)
      .env in current directory

  ✓ Config saved to ~/.config/cite/config.toml

  Ready! Try:
    cite ingest your-doc.md
    cite context "what is this about?"
```

#### What it does:
1. Asks for provider (gemini/openai/custom)
2. Asks for API key (masked input)
3. Tests the key by embedding a test string
4. Asks where to save config (XDG config or local .env)
5. Saves config
6. Shows next steps

#### Non-interactive mode:

```bash
# For CI/scripts
cite setup --provider gemini --api-key $KEY --non-interactive

# Or via environment variables (skip setup entirely)
CITE_EMBEDDING_API_KEY=$KEY CITE_EMBEDDING_PROVIDER=gemini cite context "query"
```

### Health check command (`cite setup --check`)

Diagnostic mode that checks:
- Is the binary in PATH?
- Is the config file present?
- Is the API key set?
- Can it reach the provider?
- Is the data directory writable?

```
$ cite setup --check

  CITE CLI Health Check
  ═════════════════════

  ✓ Binary: cite v0.2.0
  ✓ Config: ~/.config/cite/config.toml
  ✓ API key: set (gemini, ****...abc123)
  ✓ Provider: reachable (latency: 230ms)
  ✓ Data dir: ~/.local/share/cite/ (writable)
  ✓ Database: cite.db (3 documents, 15 chunks)

  All checks passed.
```

### Config file location

Follow XDG Base Directory spec:
- Linux/macOS: `~/.config/cite/config.toml`
- Windows: `%APPDATA%\cite\config.toml`

Override with `CITE_CONFIG` env var.

### Config file format

```toml
[provider]
type = "gemini"                    # "gemini" | "openai" | "custom"
api_key = "AIza..."                # or use CITE_EMBEDDING_API_KEY env var
model = "text-embedding-004"       # optional, uses provider default
base_url = "https://..."           # for custom providers

[retrieval]
top_k = 5
evidence_floor = 0.5
confidence_threshold = 0.7
use_hierarchy = true

[ingest]
sentence_chunking = false
min_chunk_chars = 30
max_chunk_chars = 200
build_hierarchy = false

[data]
dir = "~/.local/share/cite"        # or CITE_DATA_DIR env var
```

### Priority order for config

1. CLI flags (`--provider`, `--api-key`)
2. Environment variables (`CITE_EMBEDDING_API_KEY`, `CITE_EMBEDDING_PROVIDER`)
3. Config file (`~/.config/cite/config.toml`)
4. Defaults

## Implementation sketch

### New files/changes:

| Component | Location | Description |
|-----------|----------|-------------|
| `install.sh` | repo root | Download + PATH setup script |
| `cite setup` | `crates/cli/src/commands/setup.rs` | Interactive wizard |
| Config file support | `crates/config/src/lib.rs` | TOML parsing, XDG paths |
| `cite setup --check` | `crates/cli/src/commands/setup.rs` | Health diagnostics |

### Dependencies (new crates):

| Crate | Purpose |
|-------|---------|
| `dialoguer` | Interactive prompts (select, input, password) |
| `toml` | Config file parsing |
| `directories` | XDG/platform config paths |

### Estimated scope:

| Area | Est. Lines |
|------|------------|
| install.sh | ~80 |
| setup command (interactive) | ~150 |
| setup --check (diagnostics) | ~80 |
| Config file support (TOML) | ~120 |
| Tests | ~100 |
| **Total** | **~530** |

## Open questions

1. **Should `install.sh` auto-run `cite setup`?**
   - Pro: seamless first experience
   - Con: `curl | sh` with interactive prompts feels sketchy to some users
   - Recommendation: ask "Run setup now? [Y/n]" after install

2. **Config file vs env vars priority?**
   - Recommendation: env vars override config file (12-factor friendly)

3. **Should API key be stored in config file?**
   - Security concern: plaintext key on disk
   - Alternative: OS keychain (complex, platform-specific)
   - Recommendation: config file with `chmod 600`, note in docs about security

4. **Support `CITE_API_KEY` as alias for `CITE_EMBEDDING_API_KEY`?**
   - Shorter, easier to remember
   - Recommendation: yes, with deprecation warning if old name used

5. **Should `cite setup` also create the data directory?**
   - Recommendation: yes, create `~/.local/share/cite/` if it doesn't exist

## Related

- Phase 9 installation experience (already done)
- [Installation Guide](../../guides/installation.md) — current install methods
- [Agent Usage Guide](../../guides/agent-usage-guide.md) — usage patterns
