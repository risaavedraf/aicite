# Phase 7: Packaging + Docs — Explore Notes

## Current state inventory

### README.md
- Exists with good foundation: quick start, config, env vars, runtime modes, storage, privacy, dev commands
- **Gaps**: 
  - No per-command usage examples (only `health` shown)
  - Missing demo instructions (packaged and local/private flows)
  - No storage path table with all CLI-managed directories
  - Provider disclosure present but could be more prominent
  - No "all commands" reference section

### .env.example
- Well-structured with sections: Runtime, Embedding Provider, Ingest Pipeline, Retrieval
- **Gap**: Retrieval section still says "Phase 3 — not yet implemented" (outdated since Phase 3 is done)

### CI/CD (.github/workflows/ci.yml)
- Basic pipeline: check, test, clippy, fmt
- **Gap**: No release/build workflow for cross-platform binaries

### CLI surface (12 commands)
`health`, `ingest`, `list`, `get`, `retry`, `search`, `retrieve`, `context`, `read`, `trace`, `refresh`, `evaluate`

- All commands have `--help` via clap
- All support `--json`, `--config`, `--data-dir`, `--runtime-mode`
- Binary name: `harness` (user noted future rename, keep as-is for now)

### Sample/demo documents
- Only test fixtures exist: `crates/ingest/tests/fixtures/sample.md`, `sample.txt`
- Need proper demo corpus for packaged demo mode

### Legal/privacy docs
- Comprehensive: `docs/prd/12-legal-privacy-compliance.md`
- README has privacy section but could reference PRD for details
- Chile law (Ley 19.628 / Ley 21.719) documented

### Provider disclosure
- README mentions it in Privacy section
- Need to verify CLI output includes disclosure when provider calls happen
- PRD requires: "Provider disclosure in CLI output"

### Runtime modes
- `public_packaged_demo` (uploads disabled)
- `local_private_demo` (uploads enabled)  
- `production` (blocked until compliance)

## Phase 7 deliverables mapping

| Deliverable | Current state | Work needed |
|---|---|---|
| Reproducible CLI binary builds | Cargo build works locally | Release workflow + cross-platform targets |
| Packaged demo with sample docs | Test fixtures only | Create demo corpus (3+ docs, 10+ facts) |
| Complete README | Good foundation | Add command reference, demo guide, storage table |
| `.env.example` | Exists, mostly complete | Fix outdated comment, minor cleanup |
| Chile privacy compliance notes | In README + PRD | Add summary to README, link to PRD |
| Provider disclosure in CLI output | In README | Verify runtime output, add if missing |
| Demo acceptance flow | PRD has criteria | Create step-by-step demo script |
| CI/CD for releases | Basic CI only | Add release workflow |

## Key decisions needed

1. **Demo corpus**: Should we create real sample documents (markdown/text) or reuse golden dataset fixtures?
2. **Release targets**: Which platforms? (linux-x86_64, windows-x86_64, macos-arm64 at minimum)
3. **Demo script**: Standalone markdown doc or integrated into README?
4. **Provider disclosure**: Engine-level output or CLI-level banner?

## PRD acceptance criteria checklist (Phase 7 relevant)

### Engineering acceptance
- [x] Clear README in English
- [x] `.env.example` documents env vars
- [x] CLI can be built and launched locally
- [ ] Packaging support for CLI binary ← Phase 7
- [x] Tests run from single command (`cargo test`)
- [x] CI runs tests and linting
- [x] No secrets committed
- [x] Logs avoid sensitive data

### Demo acceptance
- [ ] Packaged sample build works in < 5 min ← Phase 7
- [ ] Local/private demo works in < 5 min ← Phase 7
- [ ] Provider disclosure visible ← Phase 7
- [ ] Verification disclaimer visible ← verify

### CLI/config acceptance
- [ ] Commands have stable help text ← verify all
- [ ] `--json` output is machine-readable ← verify
- [ ] README documents all config, paths, reset steps ← Phase 7
