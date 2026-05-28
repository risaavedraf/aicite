# Phase 7: Packaging + Docs — Design

## 1. Demo corpus

### 1.1 Directory layout

```
demo/
├── architecture.txt
├── api-reference.md
└── security-policy.txt
```

### 1.2 Content design

Each file is a standalone, human-readable document. Content is derived from the golden evaluation corpus but expanded into proper prose.

**architecture.txt** sections:
- Title: "System Architecture Overview"
- API Gateway section (routes, external requests, microservices)
- Database section (PostgreSQL, read replicas, failover)
- Authentication section (JWT, 15-min expiry, refresh tokens)
- Logging section (structured JSON, ELK stack)

**api-reference.md** sections:
- Title: "API Reference"
- GET /users (pagination, default 20, max 100)
- POST /users (email + role fields, 201 response)
- Error codes (429 rate limit, Retry-After header)
- Rate limiting config (100 req/min per key, burst 10)

**security-policy.txt** sections:
- Title: "Security Policy"
- Password requirements (12+ chars, complexity rules)
- Encryption (AES-256 at rest, TLS 1.3 in transit)
- Access logging (90-day retention)
- Prompt injection note (documented as attack vector, not executable)

### 1.3 Auto-ingest for packaged demo

Add to engine startup (not CLI command):

```rust
// In engine::lib or a new demo module
pub fn ensure_demo_corpus(db: &Database, mode: RuntimeMode) -> Result<(), HarnessError> {
    if mode != RuntimeMode::PublicPackagedDemo {
        return Ok(());
    }
    if db.document_count()? > 0 {
        return Ok(()); // Already has docs
    }
    // Ingest demo/ directory files
    // ...
}
```

This runs on first `health`, `list`, or any retrieval command when in `public_packaged_demo` mode with an empty corpus.

## 2. Provider disclosure

### 2.1 Detection logic

```rust
// crates/engine/src/runtime_guard.rs (new function)
pub fn is_real_provider(provider_id: &str) -> bool {
    !matches!(provider_id, "eval" | "golden" | "mock" | "test")
}
```

### 2.2 CLI integration

```rust
// crates/cli/src/main.rs or commands/mod.rs
struct CliSession {
    disclosure_shown: bool,
}

impl CliSession {
    fn maybe_show_disclosure(&mut self, provider_id: &str) {
        if !self.disclosure_shown && runtime_guard::is_real_provider(provider_id) {
            eprintln!("⚠ Provider disclosure: ...");
            self.disclosure_shown = true;
        }
    }
}
```

### 2.3 Flag handling

- `--no-banner` added to global CLI args (alongside `--json`, `--config`, etc.)
- Stored in args struct, passed to command execution
- When set, `maybe_show_disclosure` is a no-op

### 2.4 Provider ID source

The provider ID comes from `config.embedding.provider` (already available in Config struct). No need to make a provider call — just check the configured value.

## 3. README structure

### 3.1 New section order

1. Title + one-liner
2. Quick start (build + run)
3. **All Commands** ← NEW
4. **Demo** ← NEW
5. Configuration (env vars, config file, precedence)
6. Runtime modes
7. **Storage Paths** ← NEW
8. Privacy and Compliance (expanded)
9. **Compliance** ← NEW (links to PRD)
10. Development (tests, lint, format)
11. License

### 3.2 Commands table format

```markdown
| Command | Description | Example |
|---|---|---|
| `health` | Check CLI runtime and local state health | `harness health --json` |
| `ingest` | Ingest a document into the corpus | `harness ingest ./doc.txt` |
| `list` | List documents in the corpus | `harness list` |
| `get` | Get document metadata | `harness get <doc-id>` |
| `retry` | Retry a failed document | `harness retry <doc-id>` |
| `search` | Search the ready corpus using vector similarity | `harness search "query"` |
| `retrieve` | Retrieve top-ranked chunks with full text | `harness retrieve "query"` |
| `context` | Build an agent-consumable context pack | `harness context "query"` |
| `read` | Read a citation or chunk by ID | `harness read <citation-id>` |
| `trace` | Look up trace metadata | `harness trace <trace-id>` |
| `refresh` | Refresh corpus with atomic snapshot swap | `harness refresh` |
| `evaluate` | Run golden dataset evaluation | `harness evaluate --json` |
```

## 4. Demo script design

### 4.1 Track A: Packaged demo

```
Step 1: Download → shows binary name, no Rust needed
Step 2: harness health --json → shows healthy status
Step 3: harness list → shows 3 preloaded sample docs
Step 4: harness context "What does the API gateway do?"
        → shows context pack with citations
Step 5: harness read <citation-id>
        → shows source snippet
Step 6: harness context "What is quantum computing?"
        → shows no_results
Step 7: Verify provider disclosure + disclaimer visible
```

### 4.2 Track B: Local/private demo

```
Step 1: git clone + cargo build --release
Step 2: cp .env.example .env → set API key
Step 3: harness ingest demo/
        → shows no-sensitive-data warning
        → shows 3 docs ingested
Step 4: harness context "How are passwords validated?"
        → shows context with citations
Step 5: harness read <citation-id>
        → shows source
Step 6: harness context "What is quantum computing?"
        → shows no_results
Step 7: harness evaluate
        → shows 8/8 pass
Step 8: Check README compliance section
```

## 5. Release CI design

### 5.1 Workflow structure

```yaml
name: Release

on:
  push:
    tags: ['v*']

permissions:
  contents: write

jobs:
  build:
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            runner: ubuntu-latest
            artifact: harness-linux-amd64
          - target: x86_64-pc-windows-msvc
            runner: windows-latest
            artifact: harness-windows-amd64.exe
          - target: aarch64-apple-darwin
            runner: macos-latest
            artifact: harness-macos-arm64

    runs-on: ${{ matrix.runner }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo build --release
      - name: Rename binary
        run: ...  # platform-specific rename
      - name: Smoke test
        run: ./harness health --json
      - uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact }}
          path: target/release/harness*

  release:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
      - uses: softprops/action-gh-release@v2
        with:
          files: harness-*
          generate_release_notes: true
```

### 5.2 Binary naming

- Linux: `harness-linux-amd64`
- Windows: `harness-windows-amd64.exe`
- macOS: `harness-macos-arm64`

## 6. File change summary

| File | Action | Lines (est.) |
|---|---|---|
| `demo/architecture.txt` | New | ~80 |
| `demo/api-reference.md` | New | ~70 |
| `demo/security-policy.txt` | New | ~90 |
| `crates/cli/src/main.rs` | Edit | ~20 |
| `crates/cli/src/commands/mod.rs` | Edit | ~15 |
| `crates/engine/src/runtime_guard.rs` | Edit | ~15 |
| `README.md` | Rewrite | ~200 |
| `.env.example` | Edit | ~5 |
| `docs/demo.md` | New | ~150 |
| `.github/workflows/release.yml` | New | ~80 |
| **Total** | | **~725** |

Note: ~425 lines are docs/demo (no code risk), ~300 lines are code changes (minimal, well-bounded).
