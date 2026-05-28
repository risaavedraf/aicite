# Rename: Harness → CITE

## Decision

The CLI tool will be renamed from `harness` to `cite`.

**New name**: CITE
**Full form**: **C**ITE **I**s **T**he **E**vidence
**Tagline**: "CITE Is The Evidence — grounded retrieval with verifiable citations"

## Rationale

- **Short**: 4 letters, easy to type as CLI command
- **Descriptive**: The core feature is citing sources with evidence
- **Recursive acronym**: Like WINE (Wine Is Not an Emulator)
- **Natural usage**: `cite context "query"`, `cite ingest doc.txt`, `cite evaluate`

## Impact

The rename affects:

### Code
- `Cargo.toml` — binary name
- `crates/cli/src/main.rs` — CLI name in clap
- All imports referencing `harness` binary

### Documentation
- `README.md` — all command examples
- `docs/demo.md` — demo script
- `docs/installation.md` — installation commands
- `docs/agent-usage-guide.md` — agent usage examples
- `docs/v0.2.0-hierarchical-graph.md` — examples
- `docs/prd/` — all PRD documents (optional, could keep as-is for historical reference)

### Infrastructure
- `.github/workflows/release.yml` — binary artifact names
- `.github/workflows/ci.yml` — any references
- `install.sh` — installation script

### Environment variables
- `HARNESS_*` → `CITE_*` (breaking change)
- Or keep `HARNESS_*` for backward compatibility

### Data
- `harness.db` → `cite.db` (or keep for backward compatibility)
- Data directory: `harness/` → `cite/`

## Proposed CLI usage

```bash
# Health check
cite health --json

# Ingest documents
cite ingest ./docs/readme.md
cite ingest ./docs/

# List documents
cite list

# Search
cite search "how does authentication work"

# Retrieve with full text
cite retrieve "database setup"

# Context with citations
cite context "what are the acceptance criteria?"

# Read a citation
cite read --citation-id c1 --trace-id trace_xxx

# Trace
cite trace trace_xxx

# Evaluate
cite evaluate

# Refresh
cite refresh

# Retry
cite retry doc_abc123
```

## Environment variables

```bash
# New naming
CITE_CONFIG=/path/to/config
CITE_DATA_DIR=/path/to/data
CITE_CACHE_DIR=/path/to/cache
CITE_RUNTIME_MODE=local_private_demo
CITE_EMBEDDING_PROVIDER=gemini
CITE_EMBEDDING_MODEL=gemini-embedding-001
CITE_EMBEDDING_API_KEY=AIza...
CITE_TOP_K=5
```

## Implementation plan

### Phase 1: Documentation
- [x] Document rename decision (this file)
- [ ] Update README with new name
- [ ] Update all docs with new CLI examples

### Phase 2: Code rename
- [ ] Update Cargo.toml binary name
- [ ] Update CLI clap name
- [ ] Update all env var references (HARNESS_* → CITE_*)
- [ ] Update data directory names

### Phase 3: Infrastructure
- [ ] Update CI/CD workflows
- [ ] Update install script
- [ ] Update release workflow artifact names

### Phase 4: Migration
- [ ] Add backward compatibility for HARNESS_* env vars
- [ ] Add migration for harness.db → cite.db
- [ ] Deprecation warnings for old names

## Backward compatibility

To avoid breaking existing installations:

1. **Env vars**: Support both `HARNESS_*` and `CITE_*`, prefer `CITE_*`
2. **Data dir**: Support both `harness/` and `cite/`, prefer `cite/`
3. **Database**: Support both `harness.db` and `cite.db`, prefer `cite.db`
4. **Binary**: Ship as `cite`, but could add `harness` as alias/symlink

## Status

- [x] Decision made: Rename to CITE
- [ ] Documentation updated
- [ ] Code renamed
- [ ] Infrastructure updated
- [ ] Migration implemented
