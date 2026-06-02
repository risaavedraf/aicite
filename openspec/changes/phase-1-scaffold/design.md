# SDD Design — mvp-scaffold

## 1. Architecture overview

```
┌─────────────────────────────────────────────────────────────┐
│                        CLI (clap)                           │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐     │
│  │  health  │  │  ingest  │  │  search  │  │  context │     │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘     │
│       │              │              │              │        │
│       └──────────────┴──────────────┴──────────────┘        │
│                          │                                  │
│                    ┌─────▼─────┐                            │
│                    │  Engine   │ (orchestration)            │
│                    └─────┬─────┘                            │
│          ┌───────────────┼───────────────┐                  │
│    ┌─────▼─────┐   ┌─────▼─────┐   ┌─────▼─────┐            │
│    │  Storage  │   │ Retrieval │   │  Ingest   │            │
│    │  (SQLite) │   │  (vector) │   │ (extract) │            │
│    └───────────┘   └───────────┘   └───────────┘            │
│                          │                                  │
│                    ┌─────▼─────┐                            │
│                    │ Providers │ (embedding)                │
│                    └───────────┘                            │
└─────────────────────────────────────────────────────────────┘
         │
    ┌────▼────┐
    │ Config  │ (env + file + flags)
    └─────────┘
         │
    ┌────▼────┐
    │ Common  │ (types, errors, exit codes)
    └─────────┘
```

## 2. Design decisions

### 2.1 Crate responsibility model

**Decision**: Each crate owns its domain and exposes a clean public API. Crates depend on `common` for shared types but not on each other (except through `engine` for orchestration).

**Rationale**: Loose coupling allows independent testing and future refactoring. The `engine` crate is the only one that knows about all other crates.

**Dependency graph**:
```
cli → engine, config, common
engine → storage, retrieval, ingest, providers, graph, common
storage → common
config → common
retrieval → common, storage
ingest → common, storage, providers
providers → common
graph → common
common → (none)
```

### 2.2 Config precedence implementation

**Decision**: Layered config with explicit merge strategy.

```rust
pub struct Config {
    pub runtime: RuntimeConfig,
    pub paths: PathsConfig,
    pub embedding: EmbeddingConfig,
    pub retrieval: RetrievalConfig,
    pub rate_limit: RateLimitConfig,
}

impl Config {
    pub fn load() -> Result<Self, CiteError> {
        let defaults = Defaults::load();
        let file = FileConfig::load(config_path_from_env())?;
        let env = EnvConfig::load();
        let flags = FlagConfig::parse();
        
        Ok(Self::merge(defaults, file, env, flags))
    }
}
```

**Key rules**:
- CLI flags override everything
- Env vars override config file
- Config file overrides defaults
- Secrets (API keys) never in config file, always in env vars
- Config file path: `CITE_CONFIG` env var or OS-appropriate default

### 2.3 SQLite connection management

**Decision**: Single connection per command invocation with WAL mode and busy timeout.

```rust
pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(data_dir: &Path) -> Result<Self, CiteError> {
        let db_path = data_dir.join("cite.db");
        let conn = Connection::open(&db_path)?;
        
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "busy_timeout", 5000)?;
        
        Self::run_migrations(&conn)?;
        
        Ok(Self { conn })
    }
}
```

**Rationale**: Single-shot process model means one connection per command. WAL mode enables concurrent reads during writes. Busy timeout prevents immediate lock failures.

### 2.4 Migration system

**Decision**: Numbered SQL files with a version tracking table.

```
crates/storage/src/migrations/
├── mod.rs           # Migration runner
├── 001_initial.sql  # Schema creation
└── 002_*.sql        # Future migrations
```

**Implementation**:
```rust
pub fn run_migrations(conn: &Connection) -> Result<(), CiteError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );"
    )?;
    
    let current = get_current_version(conn)?;
    let migrations = get_pending_migrations(current);
    
    for migration in migrations {
        conn.execute_batch(&migration.sql)?;
        conn.execute(
            "INSERT INTO _migrations (version) VALUES (?1)",
            [migration.version],
        )?;
    }
    
    Ok(())
}
```

### 2.5 Error handling strategy

**Decision**: Domain error enum with structured fields, convertible to JSON output.

```rust
#[derive(Debug, thiserror::Error)]
pub enum CiteError {
    #[error("Unsupported file type: {file_type}")]
    UnsupportedFileType { file_type: String },
    
    #[error("File too large: {size_bytes} bytes (max: {max_bytes})")]
    FileTooLarge { size_bytes: u64, max_bytes: u64 },
    
    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },
    
    #[error("Document not found: {document_id}")]
    DocumentNotFound { document_id: String },
    
    #[error("Document not ready: {document_id}")]
    DocumentNotReady { document_id: String },
    
    #[error("Configuration error: {message}")]
    ConfigError { message: String },
    
    #[error("Storage error: {message}")]
    StorageError { message: String },
    
    #[error("Internal error: {message}")]
    InternalError { message: String },
}

impl CiteError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnsupportedFileType { .. } => "unsupported_file_type",
            Self::FileTooLarge { .. } => "file_too_large",
            Self::FileNotFound { .. } => "file_not_found",
            Self::DocumentNotFound { .. } => "document_not_found",
            Self::DocumentNotReady { .. } => "document_not_ready",
            Self::ConfigError { .. } => "config_error",
            Self::StorageError { .. } => "storage_error",
            Self::InternalError { .. } => "internal_error",
        }
    }
    
    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::UnsupportedFileType { .. } => ExitCode::Validation,
            Self::FileTooLarge { .. } => ExitCode::Validation,
            Self::FileNotFound { .. } => ExitCode::NotFound,
            Self::DocumentNotFound { .. } => ExitCode::NotFound,
            Self::DocumentNotReady { .. } => ExitCode::NotFound,
            Self::ConfigError { .. } => ExitCode::Validation,
            Self::StorageError { .. } => ExitCode::Internal,
            Self::InternalError { .. } => ExitCode::Internal,
        }
    }
}
```

### 2.6 CLI output formatting

**Decision**: Separate human-readable and JSON output paths.

```rust
pub enum OutputFormat {
    Human,
    Json,
}

pub trait Output {
    fn write_human(&self, writer: &mut impl Write) -> io::Result<()>;
    fn write_json(&self, writer: &mut impl Write) -> io::Result<()>;
}

pub fn print_output<T: Output>(output: &T, format: OutputFormat) {
    let mut writer = io::stdout();
    match format {
        OutputFormat::Human => output.write_human(&mut writer).unwrap(),
        OutputFormat::Json => output.write_json(&mut writer).unwrap(),
    }
}
```

**Rules**:
- stdout: primary result output
- stderr: diagnostics, errors, warnings
- JSON mode: machine-readable, stable schema
- Human mode: colored, formatted, user-friendly

### 2.7 Health command implementation

```rust
pub fn execute(config: &Config, db: &Database, format: OutputFormat) -> i32 {
    let output = HealthOutput {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        schema_version: "context-v1".to_string(),
        runtime_mode: config.runtime.mode.to_string(),
        data_dir_configured: config.paths.data_dir.is_some(),
        cache_dir_configured: config.paths.cache_dir.is_some(),
        database_ok: db.check_health().is_ok(),
    };
    
    print_output(&output, format);
    ExitCode::Success as i32
}
```

## 3. Testing strategy

### 3.1 Unit tests

- **Config**: Test env var parsing, file loading, precedence merge
- **Error**: Test code/exit_code mapping, JSON serialization
- **Types**: Test Document/Chunk/Citation serialization roundtrips

### 3.2 Integration tests

- **Health command**: Run CLI, verify JSON output, check exit code
- **Migrations**: Open fresh DB, run migrations, verify schema
- **Config loading**: Set env vars, run CLI, verify config values

### 3.3 Test fixtures

```rust
// Test config file
const TEST_CONFIG: &str = r#"
[runtime]
mode = "local_private_demo"

[paths]
data_dir = "/tmp/cite-test"

[embedding]
provider = "mock"
model = "test-model"
"#;
```

## 4. Future considerations

- **Provider abstraction**: The `providers` crate will define a trait `EmbeddingProvider` with `embed(&self, text: &str) -> Result<Vec<f32>>`. Implementations for OpenAI, local models, and mock providers.
- **Vector storage**: Will need to decide between SQLite-vss, raw cosine in-memory, or a separate vector DB. Deferred to retrieval phase.
- **Durable locks**: Will use SQLite advisory locks or a separate locks table. Deferred to durability phase.
