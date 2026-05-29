# Specification — Installation & Setup UX

## Requirement: TOML config file loading

The config crate MUST load configuration from a TOML file at the XDG-compliant path.

- Linux/macOS: `~/.config/cite/config.toml`
- Windows: `%APPDATA%\cite\config.toml`
- Override: `CITE_CONFIG` env var or `--config` CLI flag

### Config file format

```toml
[provider]
type = "gemini"                    # "gemini" | "openai" | "custom"
api_key = "AIza..."                # or use CITE_EMBEDDING_API_KEY env var
model = "text-embedding-004"       # optional, uses provider default

[retrieval]
top_k = 5
evidence_floor = 0.5
confidence_threshold = 0.7

[data]
dir = "~/.local/share/cite"        # or CITE_DATA_DIR env var
```

### Precedence order

1. CLI flags (`--provider`, `--api-key`, `--data-dir`)
2. Environment variables (`CITE_EMBEDDING_API_KEY`, `CITE_EMBEDDING_PROVIDER`, `CITE_DATA_DIR`)
3. Config file (`~/.config/cite/config.toml`)
4. Built-in defaults

#### Scenario: Config file loaded when present

- GIVEN a valid `config.toml` with `provider.type = "openai"` and `api_key = "sk-test"`
- WHEN `Config::load()` is called
- THEN `config.embedding.provider` is `"openai"`
- AND `config.embedding.api_key` is `Some("sk-test")`

#### Scenario: Env var overrides config file

- GIVEN config.toml has `provider.type = "gemini"`
- AND env var `CITE_EMBEDDING_PROVIDER=openai`
- WHEN `Config::load()` is called
- THEN `config.embedding.provider` is `"openai"`

#### Scenario: CLI flag overrides env var

- GIVEN env var `CITE_EMBEDDING_PROVIDER=gemini`
- AND CLI flag `--provider openai`
- WHEN config is loaded with CLI overrides
- THEN `config.embedding.provider` is `"openai"`

#### Scenario: Graceful degradation when config file missing

- GIVEN no config file exists at the default path
- AND no `CITE_CONFIG` env var set
- WHEN `Config::load()` is called
- THEN config loads from env vars and defaults without error

---

## Requirement: API key in config

`EmbeddingConfig` MUST include an optional `api_key` field.

- When present in config file, it is used as fallback if no env var is set
- The `setup` wizard writes it to the config file
- The file should be created with restrictive permissions (0600 on Unix)

#### Scenario: API key from config file used when no env var

- GIVEN config.toml has `api_key = "AIza..."`
- AND env var `CITE_EMBEDDING_API_KEY` is not set
- WHEN a provider is created
- THEN the API key from config is used

#### Scenario: Env var takes precedence over config file key

- GIVEN config.toml has `api_key = "from-file"`
- AND env var `CITE_EMBEDDING_API_KEY=from-env`
- WHEN a provider is created
- THEN the API key `"from-env"` is used

---

## Requirement: Enhanced health diagnostics

`cite health --json` MUST report:

- CLI version
- Runtime mode
- Config file path (if loaded)
- API key status: `set` (with masked value) or `missing`
- Provider ID and model
- Provider reachability: `reachable` (with latency) or `unreachable` (with error)
- Data directory: path, writable status
- Database: exists, document count, chunk count

`cite setup --check` MUST be an alias for `cite health`.

#### Scenario: Health check reports full diagnostics

- GIVEN a configured cite installation with API key and populated database
- WHEN `cite health --json` is executed
- THEN output contains `version`, `config_path`, `api_key_status`, `provider`, `data_dir`, `database` fields
- AND `api_key_status` is `"set"` with masked key

#### Scenario: Health check with missing API key

- GIVEN no API key configured
- WHEN `cite health --json` is executed
- THEN `api_key_status` is `"missing"`
- AND `provider_status` is `"skipped"` (no test attempted)

#### Scenario: Health check with unreachable provider

- GIVEN an invalid API key
- WHEN `cite health --json` is executed
- THEN `provider_status` is `"unreachable"` with error message
- AND exit code is still 0 (informational, not fatal)

---

## Requirement: Setup wizard (`cite setup`)

Interactive wizard that guides users through initial configuration.

### Interactive flow

1. Provider selection: gemini / openai / custom
2. API key input (masked with `dialoguer::Password`)
3. Connection test (embed a test string via selected provider)
4. Config save location: `~/.config/cite/config.toml` (default) or custom path
5. Save config file
6. Show next steps

### Non-interactive mode

```bash
cite setup --provider gemini --api-key $KEY --non-interactive
```

- Skips all prompts
- Validates provider and key
- Saves config to default location
- Exits with success/failure

#### Scenario: Interactive setup completes successfully

- GIVEN a user with a valid Gemini API key
- WHEN `cite setup` is run and they select "gemini", enter key, accept defaults
- THEN config file is created at `~/.config/cite/config.toml`
- AND connection test passes
- AND next steps are displayed

#### Scenario: Non-interactive setup with valid args

- GIVEN `cite setup --provider gemini --api-key AIza... --non-interactive`
- WHEN executed
- THEN config file is created without prompts
- AND exit code is 0

#### Scenario: Non-interactive setup with invalid key

- GIVEN `cite setup --provider gemini --api-key invalid --non-interactive`
- WHEN executed
- THEN connection test fails
- AND error message is displayed
- AND exit code is non-zero

#### Scenario: Setup detects existing config

- GIVEN a config file already exists at `~/.config/cite/config.toml`
- WHEN `cite setup` is run
- THEN user is asked whether to overwrite or keep existing config
- AND choosing "keep" exits without changes

---

## Requirement: Install script (`install.sh`)

Bash script at repo root that:

1. Detects OS (linux/macos) and architecture (amd64/arm64)
2. Downloads correct binary from GitHub releases
3. Installs to `/usr/local/bin` (or `$INSTALL_DIR`)
4. Runs `cite health --json` to verify
5. Asks "Run cite setup now? [Y/n]"

#### Scenario: Install on Linux x86_64

- GIVEN a Linux x86_64 machine with curl
- WHEN `curl -sSf .../install.sh | sh` is executed
- THEN `cite` binary is installed to `/usr/local/bin/cite`
- AND `cite health --json` runs successfully

#### Scenario: Install with custom directory

- GIVEN `INSTALL_DIR=~/.local/bin`
- WHEN install script runs
- THEN binary is installed to `~/.local/bin/cite`

#### Scenario: Install offers setup

- GIVEN install completes successfully
- WHEN user answers "Y" to "Run cite setup now?"
- THEN `cite setup` is launched

---

## Requirement: `CITE_API_KEY` alias

`CITE_EMBEDDING_API_KEY` is the canonical env var. `CITE_API_KEY` MUST be accepted as a shorter alias.

Precedence: `CITE_EMBEDDING_API_KEY` > `CITE_API_KEY` > `GEMINI_API_KEY` > `OPENAI_API_KEY`

If both `CITE_EMBEDDING_API_KEY` and `CITE_API_KEY` are set, a deprecation notice is printed to stderr.

#### Scenario: CITE_API_KEY works as alias

- GIVEN env var `CITE_API_KEY=AIza...` (and no `CITE_EMBEDDING_API_KEY`)
- WHEN a provider is created
- THEN the key `AIza...` is used

#### Scenario: Deprecation notice when both set

- GIVEN `CITE_EMBEDDING_API_KEY=key1` and `CITE_API_KEY=key2`
- WHEN a provider is created
- THEN `key1` is used (canonical takes precedence)
- AND stderr shows deprecation notice for `CITE_API_KEY`

---

## Requirement: Shared helper extraction

`resolve_data_dir()` MUST exist in exactly one place (shared CLI utility or Config method).

`create_provider()` MUST exist in exactly one place (shared CLI utility).

All command modules MUST call the shared version.

#### Scenario: No duplicate resolve_data_dir

- GIVEN the CLI crate source
- WHEN `grep -r "fn resolve_data_dir" crates/cli/src/` is run
- THEN exactly 1 match is found

#### Scenario: No duplicate create_provider

- GIVEN the CLI crate source
- WHEN `grep -r "fn create_provider" crates/cli/src/` is run
- THEN exactly 1 match is found
