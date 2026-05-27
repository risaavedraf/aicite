# SDD Tasks — mvp-scaffold

## Task overview

| # | Task | Depends on | Estimated lines | Commit message |
|---|---|---|---|---|
| 1 | Cargo workspace + crate stubs | — | ~150 | `feat: initialize cargo workspace with 9 crate stubs` |
| 2 | Common crate (types, errors, exit codes) | 1 | ~200 | `feat(common): add shared types, error enum, and exit codes` |
| 3 | Config crate (env, file, defaults, precedence) | 2 | ~250 | `feat(config): add config loading with env/file/flag precedence` |
| 4 | Storage crate (SQLite, migrations, health) | 2 | ~200 | `feat(storage): add SQLite connection, WAL mode, and migration system` |
| 5 | CLI crate (clap, health command, output) | 2, 3, 4 | ~200 | `feat(cli): add clap CLI with health command and JSON output` |
| 6 | CI pipeline + .gitignore | 1 | ~50 | `ci: add GitHub Actions for test, clippy, fmt` |
| 7 | README + .env.example | 1, 2, 3, 4, 5 | ~100 | `docs: add README with setup, config, and env var docs` |
| 8 | Integration tests | 5 | ~100 | `test: add integration tests for health command and config` |

## Task 1: Cargo workspace + crate stubs

**Goal**: Create the Cargo workspace with all 9 crates as stubs.

**Steps**:
1. Create `Cargo.toml` workspace root
2. Create `crates/{cli,engine,storage,config,graph,retrieval,ingest,providers,common}/Cargo.toml`
3. Create minimal `src/lib.rs` (or `src/main.rs` for cli) in each crate
4. Verify `cargo check` passes

**Acceptance**:
- `cargo check` compiles without errors
- All 9 crates exist with valid Cargo.toml

---

## Task 2: Common crate

**Goal**: Implement shared types, error enum, and exit codes.

**Files**:
- `crates/common/src/lib.rs` — re-exports
- `crates/common/src/types.rs` — Document, Chunk, Citation, DocumentStatus, FileType
- `crates/common/src/error.rs` — HarnessError enum with code/exit_code/message methods
- `crates/common/src/exit.rs` — ExitCode enum

**Dependencies**: `serde`, `serde_json`, `thiserror`, `chrono`

**Acceptance**:
- `cargo test -p common` passes
- Error code mapping matches PRD table
- Exit code mapping matches PRD table

---

## Task 3: Config crate

**Goal**: Implement config loading with precedence.

**Files**:
- `crates/config/src/lib.rs` — Config struct, load() method
- `crates/config/src/env.rs` — EnvConfig parsing
- `crates/config/src/file.rs` — TOML file loading
- `crates/config/src/defaults.rs` — Default values

**Dependencies**: `toml`, `dirs` (for OS paths)

**Acceptance**:
- Config loads from env vars
- Config loads from TOML file
- Precedence works: flags > env > file > defaults
- `cargo test -p config` passes

---

## Task 4: Storage crate

**Goal**: Implement SQLite connection, WAL mode, and migration system.

**Files**:
- `crates/storage/src/lib.rs` — Database struct, open(), health check
- `crates/storage/src/db.rs` — Connection management, pragmas
- `crates/storage/src/migrations/mod.rs` — Migration runner
- `crates/storage/src/migrations/001_initial.sql` — Schema creation

**Dependencies**: `rusqlite`

**Acceptance**:
- Database opens with WAL mode and busy timeout
- Migrations run on first startup
- `_migrations` table tracks versions
- `cargo test -p storage` passes

---

## Task 5: CLI crate

**Goal**: Implement clap CLI with health command and output formatting.

**Files**:
- `crates/cli/src/main.rs` — Entry point, arg parsing
- `crates/cli/src/commands/mod.rs` — Command dispatch
- `crates/cli/src/commands/health.rs` — Health command implementation
- `crates/cli/src/output.rs` — OutputFormat, JSON/human formatting

**Dependencies**: `clap`, `serde_json`

**Acceptance**:
- `harness health` shows human-readable output
- `harness health --json` returns valid JSON
- Exit codes match PRD table
- `cargo test -p cli` passes

---

## Task 6: CI pipeline

**Goal**: Set up GitHub Actions for automated checks.

**Files**:
- `.github/workflows/ci.yml`
- `.gitignore`

**Acceptance**:
- CI runs on push and PR
- CI runs `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`
- `.gitignore` covers Rust artifacts, SQLite files, .env

---

## Task 7: README + .env.example

**Goal**: Document setup, config, and environment variables.

**Files**:
- `README.md`
- `.env.example`

**Acceptance**:
- README includes project description, setup instructions, config paths, env vars
- `.env.example` documents all environment variables with comments
- README mentions Chile privacy law baseline

---

## Task 8: Integration tests

**Goal**: Verify end-to-end behavior of health command and config loading.

**Files**:
- `tests/health.rs`
- `tests/config.rs`

**Acceptance**:
- `harness health --json` returns valid JSON with expected fields
- Config loading from env vars works
- Config loading from file works
- `cargo test` passes all integration tests

---

## Execution order

```
1 → 2 → 3 → 4 → 5 → 6, 7 (parallel) → 8
```

Tasks 6 and 7 can be done in parallel after task 5 completes.

## Review budget

| Task | Estimated lines | Within 400-line budget? |
|---|---|---|
| 1 | ~150 | ✓ |
| 2 | ~200 | ✓ |
| 3 | ~250 | ✓ |
| 4 | ~200 | ✓ |
| 5 | ~200 | ✓ |
| 6 | ~50 | ✓ |
| 7 | ~100 | ✓ |
| 8 | ~100 | ✓ |

All tasks are within the 400-line review budget. No chained PRs needed.
