# Design: Cite Workspaces + Check-docs

**Status:** Draft
**Created:** 2026-06-06
**Specs:** specs/workspace/spec.md, specs/check-docs/spec.md

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                         CLI Layer                            │
│  workspace init | workspace status | check-docs | search    │
└────────────────────────────┬────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────┐
│                    Workspace Resolver                        │
│  detect_workspace() → WorkspaceConfig                       │
│  resolve_db_path() → (global_path, project_path|None)       │
└────────────────────────────┬────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────┐
│                    Storage Layer                             │
│  WorkspaceStorage { global: Storage, project: Option<Storage>│
│  search() → merge(global.search, project.search)            │
│  retrieve() → project.retrieve || global.retrieve           │
└─────────────────────────────────────────────────────────────┘
```

---

## Component Design

### 1. Workspace Resolver

**Responsibility:** Detect and configure workspace context

```rust
// crates/workspace/src/resolver.rs

pub struct WorkspaceConfig {
    pub global_db: PathBuf,
    pub project_db: Option<PathBuf>,
    pub active_workspace: WorkspaceType,
    pub detection_method: DetectionMethod,
}

pub enum WorkspaceType {
    GlobalOnly,
    Project { path: PathBuf },
}

pub enum DetectionMethod {
    AutoDetected,
    ExplicitFlag,
    NoProjectFound,
}

pub fn resolve_workspace(cwd: &Path, force_global: bool) -> WorkspaceConfig {
    // 1. If force_global, return global-only
    // 2. Check for .cite/cite.db in cwd (walk up to git root)
    // 3. Check for .cite.db in cwd (fallback)
    // 4. Return config
}
```

**Key decisions:**
- Walk up directory tree to find workspace (stops at git root or filesystem root)
- `.cite/cite.db` preferred over `.cite.db` in root
- `--global` flag forces global-only mode

---

### 2. Workspace Storage

**Responsibility:** Unified interface for dual-DB operations

```rust
// crates/storage/src/workspace.rs

pub struct WorkspaceStorage {
    global: Storage,
    project: Option<Storage>,
    config: WorkspaceConfig,
}

impl WorkspaceStorage {
    pub fn open(config: WorkspaceConfig) -> Result<Self> {
        let global = Storage::open(&config.global_db)?;
        let project = match &config.project_db {
            Some(path) => Some(Storage::open(path)?),
            None => None,
        };
        Ok(Self { global, project, config })
    }

    pub async fn search(&self, query: &str, options: SearchOptions) -> Result<Vec<SearchResult>> {
        let global_results = self.global.search(query, options.clone()).await?;
        
        let mut results = match &self.project {
            Some(project) => {
                let project_results = project.search(query, options).await?;
                merge_results(project_results, global_results)
            }
            None => global_results,
        };
        
        // Sort by score, deduplicate by document_id
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.dedup_by(|a, b| a.document_id == b.document_id);
        
        Ok(results)
    }

    pub async fn retrieve(&self, query: &str, options: RetrieveOptions) -> Result<Vec<RetrieveResult>> {
        // Project-first: try project, fall back to global
        if let Some(project) = &self.project {
            let results = project.retrieve(query, options.clone()).await?;
            if !results.is_empty() {
                return Ok(results);
            }
        }
        self.global.retrieve(query, options).await
    }

    pub fn ingest(&self, target: IngestTarget) -> &Storage {
        match target {
            IngestTarget::Project => self.project.as_ref().unwrap_or(&self.global),
            IngestTarget::Global => &self.global,
        }
    }
}

fn merge_results(project: Vec<SearchResult>, global: Vec<SearchResult>) -> Vec<SearchResult> {
    // Project results take priority for same document_id
    let mut seen = HashSet::new();
    let mut merged = Vec::new();
    
    for result in project {
        seen.insert(result.document_id.clone());
        merged.push(result);
    }
    
    for result in global {
        if !seen.contains(&result.document_id) {
            merged.push(result);
        }
    }
    
    merged
}
```

---

### 3. Workspace CLI Commands

```rust
// crates/cli/src/commands/workspace.rs

pub fn workspace_init(cwd: &Path) -> Result<()> {
    let cite_dir = cwd.join(".cite");
    let db_path = cite_dir.join("cite.db");
    
    if db_path.exists() {
        println!("Workspace already initialized at {}", db_path.display());
        // Show stats
        return Ok(());
    }
    
    fs::create_dir_all(&cite_dir)?;
    Storage::create_empty(&db_path)?;
    
    println!("Workspace initialized at {}", db_path.display());
    // Show global and project stats
    Ok(())
}

pub fn workspace_status(config: &WorkspaceConfig, json: bool) -> Result<()> {
    if json {
        // Print JSON
    } else {
        // Print human-readable
    }
    Ok(())
}
```

---

### 4. Check-docs Engine

```rust
// crates/check_docs/src/lib.rs

pub struct CheckDocsEngine {
    binary_path: PathBuf,
    workspace: WorkspaceStorage,
}

impl CheckDocsEngine {
    pub fn verify_file(&self, path: &Path) -> Result<VerificationReport> {
        let content = fs::read_to_string(path)?;
        let sections = parse_markdown_sections(&content)?;
        let metadata = parse_yaml_frontmatter(&content)?;
        
        let mut results = Vec::new();
        
        for section in sections {
            let commands = extract_cite_commands(&section)?;
            for cmd in commands {
                let result = self.verify_command(&cmd)?;
                results.push(result);
            }
        }
        
        Ok(VerificationReport {
            file: path.to_path_buf(),
            binary_version: self.binary_version()?,
            metadata,
            results,
        })
    }
    
    fn verify_command(&self, cmd: &CiteCommand) -> Result<CommandResult> {
        let output = self.execute_command(&cmd.command)?;
        
        match &cmd.expected_output {
            Some(expected) => {
                let comparison = compare_outputs(&output, expected);
                Ok(CommandResult {
                    section: cmd.section.clone(),
                    line: cmd.line,
                    status: comparison.status,
                    detail: comparison.detail,
                })
            }
            None => {
                // No expected output, just check if command succeeds
                if output.exit_code == 0 {
                    Ok(CommandResult::ok(cmd))
                } else {
                    Ok(CommandResult::outdated(cmd, &output.stderr))
                }
            }
        }
    }
}

pub fn parse_markdown_sections(content: &str) -> Result<Vec<Section>> {
    // Parse markdown, extract code blocks with context
}

pub fn extract_cite_commands(section: &Section) -> Result<Vec<CiteCommand>> {
    // Extract commands starting with "cite "
}

pub fn compare_outputs(actual: &Output, expected: &str) -> Comparison {
    // Exact match, regex match for dynamic values, semantic match (Phase 2)
}
```

---

## Data Flow

### Workspace Detection Flow

```
User runs: cite search "query"
         │
         ▼
┌─────────────────────────────┐
│ Workspace Resolver          │
│ - Check cwd for .cite/      │
│ - Check parent dirs (git root)│
│ - Return WorkspaceConfig    │
└─────────────┬───────────────┘
              │
              ▼
┌─────────────────────────────┐
│ WorkspaceStorage::open()    │
│ - Open global DB            │
│ - Open project DB (if found)│
└─────────────┬───────────────┘
              │
              ▼
┌─────────────────────────────┐
│ search() on both DBs        │
│ - Merge results             │
│ - Deduplicate by doc_id     │
│ - Sort by score             │
└─────────────────────────────┘
```

### Check-docs Flow

```
User runs: cite check-docs docs/guide.md
         │
         ▼
┌─────────────────────────────┐
│ Read file                   │
│ Parse markdown sections     │
│ Extract cite commands       │
└─────────────┬───────────────┘
              │
              ▼
┌─────────────────────────────┐
│ For each command:           │
│ - Execute against binary    │
│ - Capture output            │
│ - Compare with expected     │
│ - Assign status (OK/OUTDATED/WARNING) │
└─────────────┬───────────────┘
              │
              ▼
┌─────────────────────────────┐
│ Generate report             │
│ - Human-readable (default)  │
│ - JSON (--json)             │
│ - Update metadata (--update)│
└─────────────────────────────┘
```

---

## Module Structure

```
crates/
├── workspace/          (NEW)
│   ├── src/
│   │   ├── lib.rs
│   │   ├── resolver.rs
│   │   └── storage.rs
│   └── Cargo.toml
├── check_docs/         (NEW)
│   ├── src/
│   │   ├── lib.rs
│   │   ├── parser.rs
│   │   ├── executor.rs
│   │   └── comparator.rs
│   └── Cargo.toml
├── storage/            (MODIFIED)
│   └── src/
│       └── workspace.rs  (NEW)
├── cli/                (MODIFIED)
│   └── src/
│       └── commands/
│           ├── workspace.rs  (NEW)
│           └── check_docs.rs (NEW)
└── ...
```

---

## API Changes

### Existing commands (add workspace awareness)

| Command | Change |
|---------|--------|
| `cite search` | Query both DBs when workspace active |
| `cite retrieve` | Project-first with global fallback |
| `cite ingest` | Default to project DB when workspace active |
| `cite context` | Same as search/retrieve |
| `cite health` | Show workspace status in output |
| `cite list` | Show documents from active workspace |
| `cite get` | Check project DB first, then global |

### New commands

| Command | Description |
|---------|-------------|
| `cite workspace init` | Initialize project workspace |
| `cite workspace status` | Show workspace configuration |
| `cite check-docs <path>` | Verify documentation |

---

## Performance Considerations

### Dual-DB query overhead

- **Concern:** Two DB queries per search instead of one
- **Mitigation:** SQLite is fast for local queries. Measure and optimize if needed.
- **Expected:** < 50ms additional latency for typical queries

### Workspace detection overhead

- **Concern:** Walking directory tree on every command
- **Mitigation:** Cache workspace config for session. Only re-detect if cwd changes.
- **Expected:** < 5ms for detection

---

## Migration Path

1. **v0.3.0 → v0.4.0:**
   - Global DB unchanged (no migration needed)
   - Project DB is opt-in (user runs `cite workspace init`)
   - All existing commands work without workspace

2. **Backward compatibility:**
   - No workspace = same behavior as v0.3.0
   - `--global` flag available if workspace causes issues

---

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Storage layer complexity | Keep WorkspaceStorage thin, delegate to existing Storage |
| Breaking existing tests | All existing tests run against global-only mode |
| Check-docs false positives | Start with exact match, add regex for dynamic values |
| Directory traversal edge cases | Test with nested dirs, symlinks, git submodules |
