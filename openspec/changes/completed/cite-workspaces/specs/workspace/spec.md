# Spec: Workspace Feature

**Feature:** Two-tier storage model (global + project DB)
**Status:** Draft
**Created:** 2026-06-06

---

## Overview

Cite supports a two-tier storage model:
- **Global DB:** `~/.local/share/cite/cite.db` — general knowledge, shared across all projects
- **Project DB:** `.cite/cite.db` in project root — project-specific, transportable

The engine reads both DBs for search/retrieve operations. Project results take priority when the same document exists in both.

---

## Workspace Detection

### Scenario: Auto-detect project workspace

```
GIVEN a project directory with .cite/cite.db present
WHEN any cite command is run from that directory (or subdirectory)
THEN cite detects the project workspace automatically
AND search/retrieve queries both global and project DBs
```

### Scenario: Auto-detect with .cite.db in root

```
GIVEN a project directory with .cite.db in root (no .cite/ directory)
WHEN any cite command is run from that directory
THEN cite uses .cite.db as the project workspace
AND logs a suggestion to migrate to .cite/ directory
```

### Scenario: No project workspace

```
GIVEN a directory without .cite/cite.db or .cite.db
WHEN any cite command is run
THEN cite uses only the global DB
AND behaves exactly as v0.3.0 (no regression)
```

### Scenario: Force global-only mode

```
GIVEN any directory (with or without project workspace)
WHEN cite is run with --global flag
THEN cite uses only the global DB
AND ignores any project workspace present
```

---

## Workspace Init

### Scenario: Initialize project workspace

```
GIVEN a project directory without .cite/ directory
WHEN user runs `cite workspace init`
THEN cite creates .cite/ directory
AND creates .cite/cite.db with empty schema
AND prints confirmation with path and stats
```

### Scenario: Initialize with existing .cite/ directory

```
GIVEN a project directory with .cite/ directory but no cite.db
WHEN user runs `cite workspace init`
THEN cite creates .cite/cite.db inside existing directory
AND does not overwrite other files in .cite/
```

### Scenario: Initialize when workspace already exists

```
GIVEN a project directory with .cite/cite.db already present
WHEN user runs `cite workspace init`
THEN cite prints "Workspace already initialized"
AND shows current workspace stats
AND exits with code 0 (not an error)
```

---

## Workspace Status

### Scenario: Show workspace status

```
GIVEN a project with active workspace (global + project DBs)
WHEN user runs `cite workspace status`
THEN cite shows:
  - Global DB: path, document count, chunk count
  - Project DB: path, document count, chunk count
  - Active workspace: project (auto-detected)
  - Resolution strategy: project-first
```

### Scenario: Status with no project workspace

```
GIVEN a directory without project workspace
WHEN user runs `cite workspace status`
THEN cite shows:
  - Global DB: path, document count, chunk count
  - Project DB: (none)
  - Active workspace: global-only
```

---

## Dual-DB Query Resolution

### Scenario: Search across both DBs

```
GIVEN global DB has documents A, B, C
AND project DB has documents B, D, E (B is different version)
WHEN user runs `cite search "query"`
THEN cite queries both DBs
AND returns results from both, deduplicated
AND for document B, project version takes priority
AND results are ranked by relevance across both DBs
```

### Scenario: Retrieve with priority

```
GIVEN document X exists in both global and project DBs
WHEN user runs `cite retrieve "query" --full`
THEN cite returns the project version of document X
AND includes source indicator: "project" or "global"
```

### Scenario: Search in global-only mode

```
GIVEN a project with workspace active
WHEN user runs `cite search "query" --global`
THEN cite queries only the global DB
AND behaves exactly as v0.3.0
```

---

## Ingest Behavior

### Scenario: Ingest with workspace active

```
GIVEN a project with workspace active
WHEN user runs `cite ingest document.md`
THEN cite ingests into the project DB (.cite/cite.db)
AND prints confirmation with target DB indicator
```

### Scenario: Ingest to global

```
GIVEN a project with workspace active
WHEN user runs `cite ingest document.md --global`
THEN cite ingests into the global DB
AND prints confirmation with target DB indicator
```

---

## CLI Commands

### `cite workspace init`

**Purpose:** Initialize a project workspace in the current directory

**Flags:**
- (none for MVP)

**Output:**
```
Workspace initialized at .cite/cite.db
Global DB: ~/.local/share/cite/cite.db (40 documents, 351 chunks)
Project DB: .cite/cite.db (0 documents, 0 chunks)
```

---

### `cite workspace status`

**Purpose:** Show current workspace configuration and stats

**Flags:**
- `--json` — machine-readable output

**Output (human):**
```
Workspace Status
────────────────
Active: project (auto-detected)
Resolution: project-first

Global DB:
  Path: ~/.local/share/cite/cite.db
  Documents: 40
  Chunks: 351

Project DB:
  Path: .cite/cite.db
  Documents: 0
  Chunks: 0
```

**Output (JSON):**
```json
{
  "active_workspace": "project",
  "detection_method": "auto",
  "resolution_strategy": "project_first",
  "global_db": {
    "path": "~/.local/share/cite/cite.db",
    "document_count": 40,
    "chunk_count": 351
  },
  "project_db": {
    "path": ".cite/cite.db",
    "document_count": 0,
    "chunk_count": 0
  }
}
```

---

## File Structure

```
project/
├── .cite/
│   └── cite.db          ← Project workspace (transportable)
├── docs/
│   └── architecture.md
└── src/
    └── main.rs

~/.local/share/cite/
└── cite.db              ← Global workspace (shared)
```

---

## Migration Notes

- Existing v0.3.0 installations continue working (global DB unchanged)
- No migration required for global DB
- Project DB is opt-in via `cite workspace init`
- `.cite.db` in root works but `.cite/cite.db` is preferred
