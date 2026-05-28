# SDD Spec — mvp-scaffold

## 1. Workspace structure

```
aicite/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── cli/                # CLI binary, clap commands
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── commands/
│   │       │   ├── mod.rs
│   │       │   └── health.rs
│   │       └── output.rs   # JSON/human output formatting
│   ├── engine/             # Retrieval orchestration (stub)
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── storage/            # SQLite persistence
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── db.rs       # Connection, WAL, busy timeout
│   │       └── migrations/
│   │           ├── mod.rs
│   │           └── 001_initial.sql
│   ├── config/             # Config loading + precedence
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── env.rs      # Environment variable parsing
│   │       ├── file.rs     # TOML config file loading
│   │       └── defaults.rs # Default values
│   ├── graph/              # Document/section/chunk relationships (stub)
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── retrieval/          # Vector search (stub)
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── ingest/             # File ingestion (stub)
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── providers/          # Embedding provider abstraction (stub)
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   └── common/             # Shared types, errors, exit codes
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── types.rs    # Document, Chunk, Citation, etc.
│           ├── error.rs    # Error enum + format
│           └── exit.rs     # Exit code enum
├── docs/
│   ├── prd/                # Product requirements (existing)
│   └── sdd/                # SDD artifacts
├── .github/
│   └── workflows/
│       └── ci.yml
├── .env.example
├── README.md
└── .gitignore
```

## 2. Crate dependencies

| Crate | Depends on | Purpose |
|---|---|---|
| cli | config, engine, common | Binary entry point |
| engine | storage, retrieval, ingest, providers, graph, common | Orchestration |
| storage | common | SQLite persistence |
| config | common | Config loading |
| graph | common | Document/chunk relationships |
| retrieval | common, storage | Vector search |
| ingest | common, storage, providers | File processing |
| providers | common | Embedding abstraction |
| common | (none) | Shared types |

## 3. Config contract

### Precedence (highest to lowest)

1. CLI flags (`--config`, `--data-dir`, `--cache-dir`, `--runtime-mode`, `--top-k`, `--json`)
2. Environment variables (`CITE_CONFIG`, `CITE_DATA_DIR`, `CITE_CACHE_DIR`, `CITE_RUNTIME_MODE`, `CITE_EMBEDDING_PROVIDER`, `CITE_EMBEDDING_MODEL`)
3. Config file (TOML at `$XDG_CONFIG_HOME/cite/config.toml` or `CITE_CONFIG`)
4. Runtime defaults

### Config file format (TOML)

```toml
[runtime]
mode = "local_private_demo"  # public_packaged_demo | local_private_demo | production

[paths]
data_dir = "/path/to/data"
cache_dir = "/path/to/cache"

[embedding]
provider = "openai-compatible"
model = "text-embedding-3-small"
api_key_env = "OPENAI_API_KEY"  # env var name, not the key itself

[retrieval]
top_k = 5
evidence_floor = 0.50
confidence_threshold = 0.70

[rate_limit]
max_requests = 20
window_seconds = 60
```

### Environment variables

| Variable | Purpose | Default |
|---|---|---|
| `CITE_CONFIG` | Config file path | OS-appropriate |
| `CITE_DATA_DIR` | Data directory (SQLite, indexes, locks) | OS-appropriate |
| `CITE_CACHE_DIR` | Cache directory (temp extraction) | OS-appropriate |
| `CITE_RUNTIME_MODE` | Runtime mode | `local_private_demo` |
| `CITE_EMBEDDING_PROVIDER` | Embedding provider ID | `openai-compatible` |
| `CITE_EMBEDDING_MODEL` | Embedding model ID | `text-embedding-3-small` |
| `CITE_TOP_K` | Default retrieval top-k | `5` |

## 4. Storage schema

### Migration system

- Version table: `_migrations` with columns `version INTEGER PRIMARY KEY`, `applied_at TEXT`
- Migrations in `crates/storage/src/migrations/`
- Run on startup before any command

### Initial schema (migration 001)

```sql
CREATE TABLE IF NOT EXISTS _migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS documents (
    document_id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_type TEXT NOT NULL,
    file_size_bytes INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',  -- pending | processing | ready | failed
    chunk_count INTEGER NOT NULL DEFAULT 0,
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retry_count INTEGER NOT NULL DEFAULT 3,
    next_retry_at TEXT,
    error_code TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS chunks (
    chunk_id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL REFERENCES documents(document_id),
    section_id TEXT,
    chunk_index INTEGER NOT NULL,
    text TEXT NOT NULL,
    page INTEGER,
    offset_start INTEGER,
    offset_end INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS embeddings (
    chunk_id TEXT PRIMARY KEY REFERENCES chunks(chunk_id),
    vector BLOB NOT NULL,
    model_id TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS traces (
    trace_id TEXT PRIMARY KEY,
    query_id TEXT,
    context_pack_id TEXT,
    request_type TEXT NOT NULL,  -- search | retrieve | context | ingest | refresh
    document_ids TEXT,  -- JSON array
    citation_ids TEXT,  -- JSON array
    top_k INTEGER,
    evidence_floor REAL,
    confidence_threshold REAL,
    ranking_method TEXT,
    latency_ms INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_documents_status ON documents(status);
CREATE INDEX IF NOT EXISTS idx_chunks_document ON chunks(document_id);
CREATE INDEX IF NOT EXISTS idx_traces_created ON traces(created_at);
```

## 5. CLI contract

### Command: `cite health`

**Purpose**: Verify CLI runtime and local state are usable.

**Flags**: `--json` for machine output

**Human output**:
```
✓ AI Cite CLI v0.1.0
  Runtime mode: local_private_demo
  Data dir: /home/user/.local/share/cite
  Cache dir: /home/user/.cache/cite
  Database: ok
```

**JSON output**:
```json
{
  "status": "ok",
  "version": "0.1.0",
  "schema_version": "context-v1",
  "runtime_mode": "local_private_demo",
  "data_dir_configured": true,
  "cache_dir_configured": true,
  "database_ok": true
}
```

**Exit codes**:
- `0`: Success
- `1`: Config error
- `5`: Storage error

### Error format

All errors follow this shape (from PRD):
```json
{
  "error": {
    "code": "config_error",
    "message": "Missing required configuration: CITE_DATA_DIR",
    "details": {}
  }
}
```

### Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | Validation, config, or contract error |
| 2 | Not found or not ready |
| 3 | Provider or external dependency failure |
| 4 | Runtime mode forbidden |
| 5 | Internal error |
| 6 | Operation in progress / lock conflict |
| 7 | Rate limit exceeded |

## 6. Common types

### Document

```rust
pub struct Document {
    pub document_id: String,
    pub display_name: String,
    pub file_path: PathBuf,
    pub file_type: FileType,
    pub file_size_bytes: u64,
    pub status: DocumentStatus,
    pub chunk_count: u32,
    pub retry_count: u32,
    pub max_retry_count: u32,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub error: Option<ErrorInfo>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub enum DocumentStatus {
    Pending,
    Processing,
    Ready,
    Failed,
}

pub enum FileType {
    Pdf,
    Txt,
    Md,
}
```

### Error

```rust
pub enum CiteError {
    UnsupportedFileType { file_type: String },
    FileTooLarge { size_bytes: u64, max_bytes: u64 },
    FileNotFound { path: PathBuf },
    DocumentNotFound { document_id: String },
    DocumentNotReady { document_id: String },
    ConfigError { message: String },
    StorageError { message: String },
    InternalError { message: String },
    // ... other error variants from PRD
}

impl CiteError {
    pub fn code(&self) -> &'static str;
    pub fn exit_code(&self) -> i32;
    pub fn message(&self) -> String;
}
```

### Exit code

```rust
pub enum ExitCode {
    Success = 0,
    Validation = 1,
    NotFound = 2,
    Provider = 3,
    RuntimeForbidden = 4,
    Internal = 5,
    OperationInProgress = 6,
    RateLimitExceeded = 7,
}
```

## 7. Testing requirements

### Unit tests

- Config loading from env vars, file, and defaults
- Config precedence (flags > env > file > defaults)
- Error code and exit code mapping
- Document/Chunk/Citation serialization

### Integration tests

- `cite health --json` returns valid JSON
- SQLite migration runs on first startup
- Config file loading from custom path

### CI checks

- `cargo test`
- `cargo clippy -- -D warnings`
- `cargo fmt --check`

## 8. Non-goals for this change

- Ingestion pipeline
- Retrieval pipeline
- Provider integrations
- Durable locks, rate limits
- Golden dataset, sample corpus
- Packaging/distribution
