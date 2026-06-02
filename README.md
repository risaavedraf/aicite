[![CI](https://github.com/risaavedraf/aicite/actions/workflows/ci.yml/badge.svg)](https://github.com/risaavedraf/aicite/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/risaavedraf/aicite)](https://github.com/risaavedraf/aicite/releases)

# AI Cite CLI

CLI-first semantic document engine for AI agents. Ingest private documents, retrieve cited context through stable CLI commands.

## Quick start

### Prerequisites

- Rust 1.75+
- An embedding provider API key (Gemini or OpenAI-compatible)

### Canonical run/install matrix

| Pathway | Use when | Command style |
|---|---|---|
| Dev run | Iterating without building release binaries | `cargo run --bin cite -- <command> ...` |
| Local built binary | Testing the exact local release artifact | `cargo build --release` then `./target/release/cite <command> ...` |
| Installed release binary | Using a downloaded/installed release in PATH | `cite <command> ...` (or `cite.exe` on Windows) |

### Path A — Dev run (no release build required)

```bash
# Clone the repository
git clone https://github.com/risaavedraf/aicite.git
cd aicite

# Run directly from source
cargo run --bin cite -- health --json
cargo run --bin cite -- list
```

### Path B — Local built binary

```bash
# Build release binary
cargo build --release

# Run local release artifact
./target/release/cite health --json
./target/release/cite ingest ./demo/security-policy.txt
./target/release/cite context "how does authentication work"
```

### Path C — Installed release binary (one-command install)

```bash
# Install with one command (Linux/macOS)
curl -sSf https://raw.githubusercontent.com/risaavedraf/aicite/main/install.sh | sh

# Then configure
cite setup

# Or after manual install/download and adding to PATH
cite health --json
cite search "what is the security policy"
cite context "how does authentication work"
```

All commands accept `--json` for structured output suitable for agent pipelines.

## All commands

| Command | Description | Example |
|---|---|---|
| `health` | Check CLI runtime and local state health | `cite health --json` |
| `setup` | Interactive setup wizard for first-time configuration | `cite setup` |
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

### Retrieval flags (v0.2.0+)

Commands `search`, `retrieve`, and `context` support hierarchical retrieval:

| Flag | Purpose | Example |
|---|---|---|
| `--flat` | Use flat retrieval (v0.1.0 behavior, no hierarchy) | `cite context "query" --flat` |
| `--topic <name>` | Filter results to a specific topic | `cite search "query" --topic "Authentication"` |
| `--concept <name>` | Filter results to a specific concept | `cite context "query" --concept "JWT Tokens"` |
| `--full` | Return full JSON response (default: compact when `--json`) | `cite context "query" --json --full` |
| `--k <n>` | Number of results (1-10, default 5) | `cite search "query" --k 8` |

Global flags available on every command:

| Flag | Purpose |
|---|---|
| `--json` | Output structured JSON (for agent pipelines) |
| `--config <path>` | Override config file path |
| `--data-dir <path>` | Override data directory |
| `--runtime-mode <mode>` | Override runtime mode |
| `--no-banner` | Suppress provider disclosure banner |

## Hierarchical retrieval (v0.2.0+)

CITE organizes documents into a hierarchy: **Document → Topic → Concept → Chunk**. This enables more precise retrieval by filtering at different levels.

### How it works

1. **Ingestion**: Documents are parsed into topics (sections) and concepts (knowledge units). Chunks are small (30-200 chars) and atomic.
2. **Retrieval**: By default, CITE searches chunks but enriches results with topic/concept context (breadcrumb).
3. **Filtering**: Use `--topic` or `--concept` to narrow results to specific sections.

### Example output (compact mode)

```json
{
  "result_kind": "context",
  "citations": [
    {
      "citation_id": "c1",
      "text": "JWT tokens with 15-minute expiry",
      "score": 0.95,
      "breadcrumb": "architecture.txt > Authentication > JWT Tokens"
    }
  ]
}
```

### Breadcrumb navigation

Every citation includes a `breadcrumb` showing the path from document to chunk:

```
Document > Topic > Concept > Chunk
```

This helps agents understand the context and source of each piece of information.

### Flat mode (legacy)

If you need the v0.1.0 behavior (no hierarchy, larger chunks), use the `--flat` flag:

```bash
cite context "query" --flat
```

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
git clone https://github.com/risaavedraf/aicite.git
cd aicite
cargo build --release

# 2. Configure your embedding provider
cp .env.example .env
# Edit .env and set CITE_EMBEDDING_API_KEY

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
| `CITE_CONFIG` | Config file path | OS-appropriate |
| `CITE_DATA_DIR` | Data directory (SQLite, indexes) | OS-appropriate |
| `CITE_CACHE_DIR` | Cache directory | OS-appropriate |
| `CITE_RUNTIME_MODE` | Runtime mode | `local_private_demo` |
| `CITE_EMBEDDING_PROVIDER` | Embedding provider ID | `openai-compatible` |
| `CITE_EMBEDDING_MODEL` | Embedding model ID | `text-embedding-3-small` |
| `CITE_EMBEDDING_API_KEY` | Embedding provider API key | _(none)_ |
| `CITE_EMBEDDING_ENDPOINT` | Custom embedding endpoint (openai-compatible only) | Provider default |
| `CITE_EMBEDDING_TIMEOUT` | Embedding request timeout in seconds | `30` |
| `CITE_MAX_FILE_SIZE` | Maximum file size in bytes | `52428800` (50 MB) |
| `CITE_CHUNK_SIZE` | Chunk size in characters | `1000` |
| `CITE_CHUNK_OVERLAP` | Chunk overlap in characters | `200` |
| `CITE_TOP_K` | Default retrieval top-k | `5` |

### Runtime naming policy (Phase 9)

- Canonical runtime namespace: `CITE_*`.
- Canonical local paths: config under `.../cite/config.toml`, data dir `.../cite/`, database `cite.db`.
- Compatibility policy: legacy `HARNESS_*` runtime variables and legacy `harness` data/db naming are **not auto-aliased** by the runtime; migrate them manually in your local environment.
- Exception: provider key fallbacks `GEMINI_API_KEY` / `OPENAI_API_KEY` are still accepted for embedding commands, but `CITE_EMBEDDING_API_KEY` remains the documented default.

### Config file

TOML format at `$XDG_CONFIG_HOME/cite/config.toml` (Linux), `%APPDATA%\cite\config.toml` (Windows), or `~/Library/Application Support/cite/config.toml` (macOS).

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
| `production` | Blocked | Requires compliance checklist completion before real uploads are enabled. See [Legal and Privacy Compliance](openspec/prd/12-legal-privacy-compliance.md). |

## Storage paths

All CLI-managed data is stored in the configured data directory (`CITE_DATA_DIR` or OS default):

| Path | Content | Manual reset |
|---|---|---|
| `cite.db` | SQLite database: documents, chunks, embeddings, traces, metadata | `rm cite.db` |
| `cite.db-wal` | SQLite write-ahead log | Removed with `cite.db` |
| `cite.db-shm` | SQLite shared memory file | Removed with `cite.db` |

### Reset all local data

```bash
# Remove everything in the data directory
rm -rf $CITE_DATA_DIR/*
```

### Reset only the database

```bash
rm $CITE_DATA_DIR/cite.db*
```

After a database reset, re-ingest documents to rebuild the corpus:

```bash
cite refresh
```

## Privacy and compliance

- **Chilean privacy law**: This product accounts for Ley 19.628 (current baseline for personal data processing) and Ley 21.719 (modernized regime with deferred entry into force). The system treats ingestion, embeddings, graph metadata, retrieval context, context packs, traces, logs, and citations as personal-data processing surfaces.
- **Designed with Chilean privacy requirements in mind.** This is not a legal-compliance certification. Before handling real personal data in production, review the [production compliance checklist](openspec/prd/12-legal-privacy-compliance.md#production-compliance-checklist) with a qualified legal professional.
- **Provider disclosure**: Document snippets, query text, or embeddings may be sent to your configured AI provider. The CLI displays a disclosure banner on retrieval commands when a real external provider is active. Use `--no-banner` to suppress.
- **Data minimization**: Only chunks needed for the requested context are sent to providers, not full documents.
- **Local storage**: MVP local storage relies on operator-controlled device/OS/filesystem protections. Does not promise encryption at rest.
- **Logs**: The CLI avoids storing full document text, full prompts, secrets, raw filenames, provider error payloads, query text, citation text, chunk text, or raw personal data in logs.

For the full privacy and compliance documentation, see [openspec/prd/12-legal-privacy-compliance.md](openspec/prd/12-legal-privacy-compliance.md).

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

## Version history

| Version | Date | Highlights |
|---|---|---|
| v0.2.3 | 2026-06-02 | Trace provenance, offline trace retrieval, CLI overrides (`--data-dir`, `--runtime-mode`), docs→openspec rename |
| v0.2.2 | 2026-05-29 | Code quality: removed unwrap() in production, refactored build_context, newtype wrappers, 40+ doc comments |
| v0.2.1 | 2026-05-28 | rustfmt CI fix |
| v0.2.0 | 2026-05-29 | Hierarchical graph, `cite setup` wizard, TOML config, `install.sh`, topic/concept filters |
| v0.1.0 | 2026-05-28 | Initial release — ingest, search, context, evaluate, 12 CLI commands |

For the full changelog, see [CHANGELOG.md](CHANGELOG.md).

## License

MIT
