# Phase 7: Packaging + Docs — Spec

## 1. Demo corpus

### 1.1 File structure

```
demo/
├── architecture.txt     # System architecture overview
├── api-reference.md     # API endpoint reference
└── security-policy.txt  # Security and compliance policy
```

### 1.2 Content requirements

Each document must be a real, readable document (not test fixture snippets). Content is based on the golden evaluation corpus topics but expanded for human readability:

- **architecture.txt** (~400 words): API gateway, microservices routing, PostgreSQL with read replicas, JWT authentication, ELK stack logging
- **api-reference.md** (~350 words): GET /users, POST /users, error codes (429), rate limiting (100 req/min)
- **security-policy.txt** (~450 words): Password requirements (12+ chars), AES-256 encryption, TLS 1.3, access logging (90 days), prompt injection note

Total: ~1200 words, 12+ retrievable facts, 3 documents.

### 1.3 Integration

- `cite ingest demo/` ingests all 3 files in `local_private_demo` mode
- `public_packaged_demo` mode auto-loads demo corpus on first run if no corpus exists
- Demo files are committed to repo (not binary artifacts)

## 2. Provider disclosure

### 2.1 Behavior

| Provider type | Disclosure shown |
|---|---|
| `eval`, `golden`, `mock` | No disclosure (deterministic test provider) |
| Any real provider (`openai-compatible`, `gemini`, etc.) | Disclosure banner on first retrieval/context call per session |

### 2.2 Banner text

```
⚠ Provider disclosure: Document snippets, query text, or embeddings may be sent
  to your configured AI provider (openai-compatible / text-embedding-3-small).
  See README for privacy details.
```

### 2.3 Suppression

- `--no-banner` global CLI flag suppresses the disclosure for the current invocation
- Disclosure is shown once per CLI invocation (not per command call)

### 2.4 Implementation location

- Engine layer: `engine::runtime_guard::check_provider_disclosure()`
- CLI layer: print banner before first retrieval/context command output
- Track `disclosure_shown` bool in CLI session state

## 3. README overhaul

### 3.1 Sections to add/expand

#### All Commands reference

Table with columns: Command | Description | Example

All 12 commands: health, ingest, list, get, retry, search, retrieve, context, read, trace, refresh, evaluate

#### Demo section

Two subsections:
1. **Packaged demo** (no Rust): Download binary → run → see results
2. **Local/private demo** (with Rust): Clone → build → ingest demo docs → query

#### Storage Paths table

| Path | Content | Manual reset |
|---|---|---|
| `$CITE_DATA_DIR/cite.db` | SQLite database | Delete file |
| `$CITE_DATA_DIR/cite.db-wal` | WAL file | Delete with .db |
| ... | ... | ... |

#### Compliance summary

Brief section referencing:
- Chile Ley 19.628 / Ley 21.719
- Link to `docs/prd/12-legal-privacy-compliance.md`
- "Designed with Chilean privacy requirements in mind" (not "compliant")

### 3.2 Sections to update

- Quick start: add full build + demo commands
- Environment variables: verify all current vars documented
- Runtime modes: add demo flow description per mode

## 4. .env.example cleanup

### 4.1 Changes

- Remove "Phase 3 — not yet implemented" from Retrieval section
- Add comment that all phases are now complete
- Verify all documented vars match actual config crate

## 5. Demo script

### 5.1 File: `docs/demo.md`

### 5.2 Structure

```markdown
# AI Cite CLI — Demo Guide

## Track A: Packaged Demo (5 minutes)
1. Download binary
2. Run health check
3. See sample documents
4. Query: "What does the API gateway do?"
5. Inspect citations
6. Query: "What is quantum computing?"
7. See no_results behavior
8. See provider disclosure + verification disclaimer

## Track B: Local/Private Demo (5 minutes)
1. Clone + build
2. Configure .env
3. Ingest demo documents
4. Query + inspect citations
5. No-results query
6. Run evaluation
7. Check compliance notes
```

### 5.3 Acceptance criteria mapping

Each step maps to a PRD acceptance criterion (§Demo acceptance).

## 6. Release CI

### 6.1 File: `.github/workflows/release.yml`

### 6.2 Trigger

```yaml
on:
  push:
    tags: ['v*']
```

### 6.3 Build matrix

| Target | Runner | Binary |
|---|---|---|
| x86_64-unknown-linux-gnu | ubuntu-latest | cite-linux-amd64 |
| x86_64-pc-windows-msvc | windows-latest | cite-windows-amd64.exe |
| aarch64-apple-darwin | macos-latest | cite-macos-arm64 |

### 6.4 Steps per target

1. Checkout
2. Install Rust stable
3. `cargo build --release`
4. Rename binary to target-specific name
5. Smoke test: `./cite health --json`
6. Upload as release artifact

### 6.5 Release creation

- Create GitHub release from tag
- Auto-generate release notes from commits since last tag
- Attach all 3 binaries

## 7. Non-functional requirements

- All existing tests continue to pass
- `cargo clippy -- -D warnings` clean
- `cargo fmt --check` clean
- No new dependencies added (docs/CI only, except possibly `atty` or similar for banner detection)
- Demo corpus total size < 50KB (text files)
