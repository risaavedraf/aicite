# Spec: Check-docs Feature

**Feature:** Documentation verification against current binary
**Status:** Draft
**Created:** 2026-06-06

---

## Overview

`cite check-docs` verifies that documentation accurately reflects the current binary's behavior. It parses markdown for cite command examples, executes them, and compares actual output vs documented expectations.

---

## Core Behavior

### Scenario: Verify single file

```
GIVEN a markdown file with cite command examples
WHEN user runs `cite check-docs path/to/file.md`
THEN cite parses the file for fenced code blocks containing cite commands
AND executes each command against the current binary
AND compares actual output vs expected output (if documented)
AND prints a report with status per section
```

### Scenario: Verify directory recursively

```
GIVEN a directory with multiple markdown files
WHEN user runs `cite check-docs path/to/dir/ --recursive`
THEN cite scans all .md files in the directory tree
AND verifies each file independently
AND prints aggregate report with totals
```

### Scenario: No cite commands found

```
GIVEN a markdown file without cite command examples
WHEN user runs `cite check-docs path/to/file.md`
THEN cite prints "No cite commands found in file"
AND exits with code 0
```

---

## Command Parsing

### Scenario: Extract bash code blocks

```
GIVEN a markdown file with this content:
  ```bash
  cite search "retrieval" --topic "Authentication" --json
  ```
WHEN cite parses the file
THEN it extracts the command: `cite search "retrieval" --topic "Authentication" --json`
AND identifies the line number (for reporting)
```

### Scenario: Extract with expected output

```
GIVEN a markdown file with:
  ```bash
  cite health --json
  ```
  ```json
  {"status": "ok", "version": "0.3.0"}
  ```
WHEN cite parses the file
THEN it extracts the command AND the expected output
AND will compare actual vs expected during verification
```

### Scenario: Ignore non-cite commands

```
GIVEN a markdown file with:
  ```bash
  cargo build --release
  ```
  ```bash
  cite search "test"
  ```
WHEN cite parses the file
THEN it ignores the `cargo build` command
AND only verifies the `cite search` command
```

---

## Verification Logic

### Scenario: Command succeeds, output matches

```
GIVEN a documented command: `cite health --json`
AND expected output: `{"status": "ok"}`
WHEN cite executes the command
AND actual output contains `"status": "ok"`
THEN status is ✅ OK
```

### Scenario: Command succeeds, output differs

```
GIVEN a documented command: `cite search "test" --json`
AND expected output shows `--topic` flag working
WHEN cite executes the command
AND `--topic` flag is silently ignored (no filtering)
THEN status is ❌ OUTDATED
AND detail explains the discrepancy
```

### Scenario: Command fails

```
GIVEN a documented command: `cite old-command --flag`
WHEN cite executes the command
AND command returns non-zero exit code
THEN status is ❌ OUTDATED
AND detail includes the error message
```

### Scenario: Dynamic output

```
GIVEN a documented command: `cite health --json`
AND expected output includes `"latency_ms": 1218`
WHEN cite executes the command
AND actual output has `"latency_ms": 850`
THEN status is ⚠️ WARNING (dynamic value changed)
AND note: "latency_ms is dynamic, consider removing from expected output"
```

---

## Report Format

### Scenario: Human-readable report (default)

```
GIVEN verification results for a file
WHEN cite prints the report
THEN output format is:

Checking agent-usage-guide.md against cite v0.3.0...

❌ OUTDATED: Section "Compact/snippet mode"
   Line 45-52
   Claim: "compact mode is a proposal"
   Reality: compact is default since v0.2.0
   Suggestion: Update to reflect current behavior

✅ OK: Section "Real-world invocation"
   Line 12-18
   All examples pass

⚠️  WARNING: Section "Performance characteristics"
   Line 120
   Claim: "Query latency: 800-1500ms"
   Last verified: 2026-05-28 (9 days ago)
   Suggestion: Re-verify with current binary

Results: 1 outdated, 1 warning, 5 OK
```

### Scenario: JSON report

```
GIVEN verification results
WHEN user runs `cite check-docs file.md --json`
THEN output is machine-readable JSON:

{
  "file": "agent-usage-guide.md",
  "binary_version": "0.3.0",
  "total_sections": 7,
  "results": [
    {
      "section": "Compact/snippet mode",
      "status": "outdated",
      "line": 45,
      "detail": "compact is default, not proposal"
    }
  ],
  "summary": {
    "ok": 5,
    "outdated": 1,
    "warning": 1
  }
}
```

---

## Metadata Headers

### Scenario: Document with metadata header

```
GIVEN a markdown file with YAML frontmatter:
  ---
  type: behavioral
  verified_with: v0.3.0
  last_verified: 2026-06-05
  verification_status: pass
  ---
WHEN cite check-docs runs
THEN it reads the metadata for context
AND updates last_verified and verification_status after verification
```

### Scenario: Document without metadata

```
GIVEN a markdown file without metadata header
WHEN cite check-docs runs
THEN cite verifies normally
AND suggests adding metadata header in the report
```

---

## CLI Interface

### `cite check-docs <path>`

**Purpose:** Verify documentation against current binary

**Arguments:**
- `<path>` — File or directory to verify

**Flags:**
- `--recursive` — Scan directory recursively
- `--json` — Machine-readable output
- `--update-metadata` — Update YAML frontmatter after verification
- `--skip-dynamic` — Skip verification of known dynamic values (latency, UUIDs, timestamps)

**Exit codes:**
- `0` — All checks passed (or only warnings)
- `1` — At least one outdated section found
- `2` — Error (file not found, parse error, etc.)

---

## Dynamic Value Handling

### Known dynamic values (skip or regex-match):

| Value | Pattern | Action |
|-------|---------|--------|
| `latency_ms` | `\d+` | Skip comparison |
| UUIDs | `[a-f0-9-]{36}` | Skip comparison |
| Timestamps | ISO 8601 patterns | Skip comparison |
| `chunk_count` | `\d+` | Warn if changed significantly (>50%) |
| `document_count` | `\d+` | Warn if changed significantly (>50%) |

---

## Workspace Integration

### Scenario: Check-docs with workspace active

```
GIVEN a project with workspace active
WHEN user runs `cite check-docs docs/guide.md`
THEN cite uses the active workspace context for executing commands
AND report indicates which workspace was used
```

### Scenario: Check-docs against global only

```
GIVEN a project with workspace active
WHEN user runs `cite check-docs docs/guide.md --global`
THEN cite executes commands against global DB only
AND report indicates "verified against global workspace"
```

---

## Limitations (MVP)

1. **Only cite commands** — does not verify non-cite bash blocks
2. **No auto-fix** — suggests but does not modify files
3. **No CI integration** — manual execution only (future: GitHub Action)
4. **Exact match only** — semantic comparison in Phase 2
