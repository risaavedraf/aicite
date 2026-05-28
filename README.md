# AI Harness CLI

CLI-first semantic document engine for AI agents. Ingest private documents, retrieve cited context through stable CLI commands.

## Quick start

### Prerequisites

- Rust 1.75+
- An embedding provider API key (Gemini or OpenAI-compatible)

### Build and run

```bash
# Clone the repository
git clone https://github.com/your-org/aiharness.git
cd aiharness

# Build the release binary
cargo build --release

# Verify the installation
./target/release/cite health

# Ingest a document
./target/release/cite ingest ./demo/security-policy.txt

# Search the corpus
./target/release/cite search "what is the security policy"

# Get agent-consumable context with citations
./target/release/cite context "how does authentication work"
```

All commands accept `--json` for structured output suitable for agent pipelines:

```bash
cite health --json
```

## All commands

| Command | Description | Example |
|---|---|---|
| `health` | Check CLI runtime and local state health | `cite health --json` |
| `ingest` | Ingest a document into the corpus | `cite ingest ./doc.txt` |
| `list` | List documents in the corpus | `cite list` |
| `get` | Get document metadata | `cite get <doc-id>` |
| `retry` | Retry a failed document | `cite retry <doc-id>` |
| `search` | Search the ready corpus using vector similarity | `cite search "what is the API gateway"` |
| `retrieve` | Retrieve top-ranked chunks with full text | `cite retrieve "database setup"` |
| `context` | Build an agent-consumable context pack with citations | `cite context "how does auth work"` |
| `read` | Read a citation or chunk by ID | `cite read <citation-id>` |
| `trace` | Look up trace metadata for a context/retrieval request | `cite trace <trace-id>` |
| `refresh` | Refresh corpus with atomic snapshot swap | `cite refresh` |
| `evaluate` | Run golden dataset evaluation to verify retrieval quality | `cite evaluate --json` |

Global flags available on every command:

| Flag | Purpose |
|---|---|
| `--json` | Output structured JSON (for agent pipelines) |
| `--config <path>` | Override config file path |
| `--data-dir <path>` | Override data directory |
| `--runtime-mode <mode>` | Override runtime mode |
| `--no-banner` | Suppress provider disclosure banner |

## Demo

### Packaged demo (no Rust needed)

Download the pre-built binary for your platform, then run:

```bash
# 1. Verify runtime health
cite health --json

# 2. List the bundled sample documents
cite list

# 3. Search the sample corpus
cite search "what is the security policy"

# 4. Get a full context pack with citations
cite context "how does the API reference work"

# 5. Inspect a specific citation by ID
cite read <citation-id>

# 6. View trace metadata for the context request
cite trace <trace-id>
```

The packaged demo runs in `public_packaged_demo` mode: uploads are disabled and only bundled sample documents are available. Do not enter personal or confidential information.

### Local/private demo (with Rust)

```bash
# 1. Clone and build
git clone https://github.com/your-org/aiharness.git
cd aiharness
cargo build --release

# 2. Configure your embedding provider
cp .env.example .env
# Edit .env and set HARNESS_EMBEDDING_API_KEY

# 3. Ingest the demo documents
./target/release/cite ingest ./demo/api-reference.md
./target/release/cite ingest ./demo/architecture.txt
./target/release/cite ingest ./demo/security-policy.txt

# 4. Search and retrieve
./target/release/cite search "what are the architecture boundaries"
./target/release/cite context "explain the API contract"

# 5. Evaluate retrieval quality against golden fixtures
./target/release/cite evaluate --json
```

The local/private demo runs in `local_private_demo` mode: uploads are enabled and data is stored locally. Do not upload personal, sensitive, or confidential information unless you control the environment and provider configuration.

## Configuration

### Precedence (highest to lowest)

1. CLI flags
2. Environment variables (`.env` file or shell)
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
| `HARNESS_EMBEDDING_API_KEY` | Embedding provider API key | _(none)_ |
| `HARNESS_EMBEDDING_ENDPOINT` | Custom embedding endpoint (openai-compatible only) | Provider default |
| `HARNESS_EMBEDDING_TIMEOUT` | Embedding request timeout in seconds | `30` |
| `HARNESS_MAX_FILE_SIZE` | Maximum file size in bytes | `52428800` (50 MB) |
| `HARNESS_CHUNK_SIZE` | Chunk size in characters | `1000` |
| `HARNESS_CHUNK_OVERLAP` | Chunk overlap in characters | `200` |
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
| `public_packaged_demo` | Disabled | Safe public demo with bundled sample documents. Shows a banner: _"Demo uses sample documents. Public uploads are disabled."_ |
| `local_private_demo` | Enabled | Developer evaluation with private documents. Shows a no-sensitive-data warning and provider disclosure. Original files, extracted text, chunks, embeddings, and metadata are stored in CLI-managed local storage. |
| `production` | Blocked | Requires compliance checklist completion before real uploads are enabled. See [Legal and Privacy Compliance](docs/prd/12-legal-privacy-compliance.md). |

## Storage paths

All CLI-managed data is stored in the configured data directory (`HARNESS_DATA_DIR` or OS default):

| Path | Content | Manual reset |
|---|---|---|
| `harness.db` | SQLite database: documents, chunks, embeddings, traces, metadata | `rm harness.db` |
| `harness.db-wal` | SQLite write-ahead log | Removed with `harness.db` |
| `harness.db-shm` | SQLite shared memory file | Removed with `harness.db` |

### Reset all local data

```bash
# Remove everything in the data directory
rm -rf $HARNESS_DATA_DIR/*
```

### Reset only the database

```bash
rm $HARNESS_DATA_DIR/harness.db*
```

After a database reset, re-ingest documents to rebuild the corpus:

```bash
cite refresh
```

## Privacy and compliance

- **Chilean privacy law**: This product accounts for Ley 19.628 (current baseline for personal data processing) and Ley 21.719 (modernized regime with deferred entry into force). The system treats ingestion, embeddings, graph metadata, retrieval context, context packs, traces, logs, and citations as personal-data processing surfaces.
- **Designed with Chilean privacy requirements in mind.** This is not a legal-compliance certification. Before handling real personal data in production, review the [production compliance checklist](docs/prd/12-legal-privacy-compliance.md#production-compliance-checklist) with a qualified legal professional.
- **Provider disclosure**: Document snippets, query text, or embeddings may be sent to your configured AI provider. The CLI displays a disclosure banner on retrieval commands when a real external provider is active. Use `--no-banner` to suppress.
- **Data minimization**: Only chunks needed for the requested context are sent to providers, not full documents.
- **Local storage**: MVP local storage relies on operator-controlled device/OS/filesystem protections. Does not promise encryption at rest.
- **Logs**: The CLI avoids storing full document text, full prompts, secrets, raw filenames, provider error payloads, query text, citation text, chunk text, or raw personal data in logs.

For the full privacy and compliance documentation, see [docs/prd/12-legal-privacy-compliance.md](docs/prd/12-legal-privacy-compliance.md).

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
