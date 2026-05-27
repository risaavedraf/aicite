# AI Harness CLI

CLI-first semantic document engine for AI agents. Ingest private documents, retrieve cited context through stable CLI commands.

## Quick start

### Prerequisites

- Rust 1.75+
- (Optional) Embedding provider API key

### Build

```bash
cargo build --release
```

### Run

```bash
# Check health
cargo run -- health

# JSON output (for agents)
cargo run -- health --json
```

## Configuration

### Precedence (highest to lowest)

1. CLI flags
2. Environment variables
3. Config file
4. Runtime defaults

### Environment variables

| Variable | Purpose | Default |
|---|---|---|
| `HARNESS_CONFIG` | Config file path | OS-appropriate |
| `HARNESS_DATA_DIR` | Data directory (SQLite, indexes) | OS-appropriate |
| `HARNESS_CACHE_DIR` | Cache directory | OS-appropriate |
| `HARNESS_RUNTIME_MODE` | Runtime mode | `local_private_demo` |
| `HARNESS_EMBEDDING_PROVIDER` | Embedding provider ID | `openai-compatible` |
| `HARNESS_EMBEDDING_MODEL` | Embedding model ID | `text-embedding-3-small` |
| `HARNESS_TOP_K` | Default retrieval top-k | `5` |

### Config file

TOML format at `$XDG_CONFIG_HOME/harness/config.toml` (Linux), `%APPDATA%\harness\config.toml` (Windows), or `~/Library/Application Support/harness/config.toml` (macOS).

```toml
[runtime]
mode = "local_private_demo"

[paths]
data_dir = "/path/to/data"
cache_dir = "/path/to/cache"

[embedding]
provider = "openai-compatible"
model = "text-embedding-3-small"

[retrieval]
top_k = 5
evidence_floor = 0.50
confidence_threshold = 0.70
```

## Runtime modes

| Mode | Uploads | Purpose |
|---|---|---|
| `public_packaged_demo` | Disabled | Safe public demo with sample documents |
| `local_private_demo` | Enabled | Developer evaluation with private documents |
| `production` | Blocked | Requires compliance checklist completion |

## Local storage

Data is stored in the configured data directory:

- `harness.db` — SQLite database (documents, chunks, embeddings, traces)
- `harness.db-shm`, `harness.db-wal` — SQLite WAL files

### Manual reset

To reset all local data:

```bash
rm -rf $HARNESS_DATA_DIR/*
```

Or delete specific files:

```bash
rm $HARNESS_DATA_DIR/harness.db*
```

## Privacy and compliance

- **Chile privacy law**: This product accounts for Ley 19.628 and Ley 21.719 at a product/engineering level.
- **Provider disclosure**: Document snippets or embeddings may be sent to configured AI providers. See provider documentation.
- **Data minimization**: Only chunks needed for requested context are sent to providers, not full documents.
- **Local storage**: MVP local storage relies on operator-controlled device/OS/filesystem protections. Does not promise encryption at rest.

## Development

### Run tests

```bash
cargo test
```

### Lint

```bash
cargo clippy -- -D warnings
```

### Format

```bash
cargo fmt
```

## License

MIT
