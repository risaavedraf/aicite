# Phase 7: Packaging + Docs — Proposal

## Problem statement

Phase 7 closes the MVP. The CLI works end-to-end (ingest → retrieve → context → evaluate, 160 tests green), but a reviewer cannot:
1. Download and run the CLI without a Rust toolchain
2. See a guided demo flow in under 5 minutes
3. Find all CLI commands documented in one place
4. See provider disclosure in CLI output when a real provider is configured

## Scope

### In scope

1. **Demo corpus**: 3 sample documents (architecture, API reference, security policy) bundled as static files for `public_packaged_demo` mode
2. **README overhaul**: complete command reference, demo guide, storage path table, provider disclosure prominence
3. **`.env.example` cleanup**: remove outdated "Phase 3 — not yet implemented" comment
4. **Provider disclosure in CLI output**: banner when real provider is configured (not eval/mock)
5. **Demo script**: standalone `docs/demo.md` with 5-minute acceptance flow for both modes
6. **Release CI workflow**: GitHub Actions workflow for cross-platform binary builds
7. **Compliance summary**: README section linking to PRD compliance doc

### Out of scope

- Actual binary distribution (Homebrew, apt, etc.) — post-MVP
- GUI/TUI — post-MVP
- Production compliance checklist completion — blocks production mode, not MVP
- CLI binary rename — user decision, keep `harness` for now
- Cross-compilation tooling beyond basic cargo build targets

## Approach

### Slice 1: Demo corpus + provider disclosure
- Create `demo/` directory with 3 sample .txt files (10+ facts total)
- Reuse the same content from golden evaluation corpus (architecture.txt, api-reference.md, security-policy.txt) but as real readable documents, not test fixtures
- Add engine-level check: if provider is not `eval`/`mock`/`golden`, emit provider disclosure banner on first retrieval/context call
- Add `--no-banner` flag to suppress

### Slice 2: README overhaul
- Add "All Commands" reference table with every subcommand, one-liner, and example
- Add "Demo" section with quick-start for both modes
- Add "Storage Paths" table documenting every CLI-managed directory
- Expand "Privacy and Compliance" to reference PRD doc
- Fix `.env.example` outdated comment

### Slice 3: Demo script
- Create `docs/demo.md` with step-by-step 5-minute flow
- Two tracks: packaged demo (no Rust) and local/private (with Rust)
- Each step shows exact command and expected output snippet
- Validates all acceptance criteria from PRD §Demo acceptance

### Slice 4: Release CI
- Add `.github/workflows/release.yml`
- Trigger on tag push (`v*`)
- Build matrix: linux-x86_64, windows-x86_64, macos-arm64
- Upload binaries as release artifacts
- Basic smoke test (run `harness health --json`) before upload

## Risks

| Risk | Mitigation |
|---|---|
| Demo corpus too simple to impress | Use the 3 eval docs (12 chunks, 10+ facts) — enough for meaningful retrieval |
| Provider disclosure annoying for eval users | Only show for real providers, suppress for eval/golden/mock |
| Release CI hard to test locally | Use `act` or manual tag push to test; keep simple |

## Estimated lines

~300-400 lines total (mostly docs, thin CI yaml, small provider disclosure check)

## Dependencies

- None — all phases 1-6 are complete

## Acceptance criteria

- [ ] `cargo run -- evaluate` still passes 8/8
- [ ] `cargo test` all green
- [ ] `cargo clippy -- -D warnings` clean
- [ ] README has all 12 commands documented with examples
- [ ] `docs/demo.md` exists with 5-minute flow
- [ ] Provider disclosure shows for real provider, hidden for eval
- [ ] `.env.example` has no outdated phase references
- [ ] Release workflow exists and triggers on tag
