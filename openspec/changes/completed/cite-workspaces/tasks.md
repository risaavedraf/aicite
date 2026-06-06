# Tasks: Cite Workspaces + Check-docs

**Change:** cite-workspaces
**Status:** Draft
**Created:** 2026-06-06
**Design:** design.md

---

## Phase 1: Workspace Infrastructure

### 1.1 Create workspace crate

- [ ] Create `crates/workspace/` with Cargo.toml
- [ ] Implement `WorkspaceConfig` struct
- [ ] Implement `WorkspaceType` and `DetectionMethod` enums
- [ ] Implement `resolve_workspace()` function
- [ ] Unit tests for workspace detection

**Files:** `crates/workspace/src/lib.rs`, `crates/workspace/src/resolver.rs`

---

### 1.2 Add workspace storage layer

- [ ] Create `crates/storage/src/workspace.rs`
- [ ] Implement `WorkspaceStorage` struct with dual-DB support
- [ ] Implement `merge_results()` for search deduplication
- [ ] Implement project-first retrieval logic
- [ ] Add `IngestTarget` enum (Project/Global)
- [ ] Unit tests for merge logic

**Files:** `crates/storage/src/workspace.rs`

---

### 1.3 Implement workspace CLI commands

- [ ] Create `crates/cli/src/commands/workspace.rs`
- [ ] Implement `workspace init` command
- [ ] Implement `workspace status` command (human + JSON)
- [ ] Register commands in CLI router
- [ ] Integration tests

**Files:** `crates/cli/src/commands/workspace.rs`, `crates/cli/src/main.rs`

---

### 1.4 Modify existing commands for workspace awareness

- [ ] Update `search` command to use WorkspaceStorage
- [ ] Update `retrieve` command for project-first logic
- [ ] Update `ingest` command with target selection
- [ ] Update `context` command for dual-DB
- [ ] Update `health` command to show workspace status
- [ ] Update `list` command for workspace filtering
- [ ] Update `get` command for project-first lookup
- [ ] Add `--global` flag to all relevant commands
- [ ] Update all existing tests (should pass without workspace)

**Files:** `crates/cli/src/commands/*.rs`

---

### 1.5 Workspace integration tests

- [ ] Test: auto-detect workspace in cwd
- [ ] Test: auto-detect with .cite.db in root
- [ ] Test: no workspace = global-only
- [ ] Test: --global flag forces global-only
- [ ] Test: search merges results correctly
- [ ] Test: project version wins on conflict
- [ ] Test: ingest targets correct DB
- [ ] Test: workspace init creates correct structure
- [ ] Test: workspace status shows correct info

**Files:** `crates/workspace/tests/`, `crates/cli/tests/`

---

## Phase 2: Check-docs Engine

### 2.1 Create check-docs crate

- [ ] Create `crates/check_docs/` with Cargo.toml
- [ ] Implement `CheckDocsEngine` struct
- [ ] Implement `VerificationReport` and related types
- [ ] Implement `CommandResult` with status enum

**Files:** `crates/check_docs/src/lib.rs`

---

### 2.2 Implement markdown parser

- [ ] Create `crates/check_docs/src/parser.rs`
- [ ] Implement `parse_markdown_sections()` — extract code blocks with context
- [ ] Implement `extract_cite_commands()` — filter for cite commands only
- [ ] Implement `parse_yaml_frontmatter()` — read metadata headers
- [ ] Handle edge cases: nested blocks, indented blocks, no language tag
- [ ] Unit tests for parser

**Files:** `crates/check_docs/src/parser.rs`

---

### 2.3 Implement command executor

- [ ] Create `crates/check_docs/src/executor.rs`
- [ ] Implement command execution against cite binary
- [ ] Capture stdout, stderr, exit code
- [ ] Add timeout (30s per command)
- [ ] Handle binary path resolution
- [ ] Unit tests with mock commands

**Files:** `crates/check_docs/src/executor.rs`

---

### 2.4 Implement output comparator

- [ ] Create `crates/check_docs/src/comparator.rs`
- [ ] Implement exact match comparison
- [ ] Implement regex patterns for dynamic values (latency, UUIDs, timestamps)
- [ ] Implement `ComparisonResult` with status and detail
- [ ] Unit tests for comparison logic

**Files:** `crates/check_docs/src/comparator.rs`

---

### 2.5 Implement CLI command

- [ ] Create `crates/cli/src/commands/check_docs.rs`
- [ ] Register `check-docs` subcommand
- [ ] Implement `--recursive` flag for directory scanning
- [ ] Implement `--json` flag for machine-readable output
- [ ] Implement `--update-metadata` flag
- [ ] Implement `--skip-dynamic` flag
- [ ] Set exit codes (0=pass, 1=outdated, 2=error)

**Files:** `crates/cli/src/commands/check_docs.rs`, `crates/cli/src/main.rs`

---

### 2.6 Implement report generator

- [ ] Human-readable report format (default)
- [ ] JSON report format (--json)
- [ ] Aggregate stats for directory scans
- [ ] Color-coded output (green OK, red OUTDATED, yellow WARNING)

**Files:** `crates/check_docs/src/report.rs`

---

### 2.7 Check-docs integration tests

- [ ] Test: parse markdown with cite commands
- [ ] Test: ignore non-cite commands
- [ ] Test: extract expected output from following code block
- [ ] Test: command success = OK
- [ ] Test: command failure = OUTDATED
- [ ] Test: output mismatch = OUTDATED
- [ ] Test: dynamic value change = WARNING
- [ ] Test: recursive directory scan
- [ ] Test: JSON output format
- [ ] Test: exit codes

**Files:** `crates/check_docs/tests/`

---

## Phase 3: Polish + Documentation

### 3.1 Add metadata headers to docs

- [ ] Add YAML frontmatter to `openspec/guides/agent-usage-guide.md`
- [ ] Add YAML frontmatter to other behavioral docs
- [ ] Document metadata header format

**Files:** `openspec/guides/*.md`

---

### 3.2 Update existing documentation

- [ ] Update README.md with workspace feature
- [ ] Update agent-usage-guide.md with check-docs
- [ ] Create workspace usage guide
- [ ] Fix known desyncs found in EVALUACION_CITE.md

**Files:** `openspec/guides/`, `README.md`

---

### 3.3 Performance verification

- [ ] Benchmark: search with workspace vs global-only
- [ ] Benchmark: workspace detection overhead
- [ ] Verify < 20% regression requirement
- [ ] Document performance characteristics

**Files:** `crates/workspace/benches/`

---

### 3.4 Final integration

- [ ] Run full test suite
- [ ] Run clippy with -D warnings
- [ ] Run cargo fmt --check
- [ ] Manual smoke test: workspace init → ingest → search → check-docs
- [ ] Update CHANGELOG.md

**Files:** (project-wide)

---

## Dependency Graph

```
Phase 1 (Workspace):
  1.1 → 1.2 → 1.3 → 1.4 → 1.5

Phase 2 (Check-docs):
  2.1 → 2.2 → 2.3 → 2.4 → 2.5 → 2.6 → 2.7

Phase 3 (Polish):
  1.5 + 2.7 → 3.1 → 3.2 → 3.3 → 3.4
```

---

## Estimated Effort

| Phase | Tasks | Estimated Lines | Complexity |
|-------|-------|-----------------|------------|
| Phase 1 | 1.1-1.5 | ~800 | Medium |
| Phase 2 | 2.1-2.7 | ~600 | Medium |
| Phase 3 | 3.1-3.4 | ~200 | Low |
| **Total** | | ~1600 | Medium |

---

## Definition of Done

- [ ] All tasks checked off
- [ ] All tests passing (`cargo test`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Code formatted (`cargo fmt --check`)
- [ ] Manual smoke test passed
- [ ] Documentation updated
- [ ] CHANGELOG.md updated
