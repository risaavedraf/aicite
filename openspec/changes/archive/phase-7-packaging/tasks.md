# Phase 7: Packaging + Docs — Tasks

## Slice 1: Demo corpus + provider disclosure

### Task 1.1: Create demo corpus files
- [ ] Create `demo/architecture.txt` (~400 words, system architecture)
- [ ] Create `demo/api-reference.md` (~350 words, API endpoints)
- [ ] Create `demo/security-policy.txt` (~450 words, security policy)
- [ ] Verify total word count ≥ 1000, facts ≥ 10

### Task 1.2: Provider disclosure in engine
- [ ] Add `is_real_provider(provider_id: &str) -> bool` to `crates/engine/src/runtime_guard.rs`
- [ ] Unit test: eval/golden/mock return false
- [ ] Unit test: openai-compatible/gemini return true

### Task 1.3: Provider disclosure in CLI
- [ ] Add `--no-banner` flag to global CLI args in `crates/cli/src/main.rs`
- [ ] Add `disclosure_shown` tracking to CLI session/execution flow
- [ ] Print disclosure banner to stderr on first retrieval/context command when provider is real
- [ ] Skip banner when `--no-banner` is set
- [ ] Integration: verify banner appears for real provider, hidden for eval

### Task 1.4: Verify all tests pass
- [ ] `cargo test` — all 160+ tests green
- [ ] `cargo clippy -- -D warnings` clean
- [ ] `cargo fmt --check` clean

---

## Slice 2: README overhaul + .env cleanup

### Task 2.1: README command reference
- [ ] Add "All Commands" table with all 12 commands, one-liner, and example
- [ ] Add brief usage example per command (not just the table)

### Task 2.2: README demo section
- [ ] Add "Demo" section with two tracks (packaged + local/private)
- [ ] Packaged demo: download → run → query → inspect
- [ ] Local/private demo: clone → build → ingest → query → evaluate

### Task 2.3: README storage paths
- [ ] Add "Storage Paths" table documenting all CLI-managed directories
- [ ] Include manual reset instructions per path

### Task 2.4: README compliance section
- [ ] Expand "Privacy and Compliance" with summary of Chile law requirements
- [ ] Link to `docs/prd/12-legal-privacy-compliance.md`
- [ ] Add "designed with Chilean privacy requirements in mind" disclaimer

### Task 2.5: .env.example cleanup
- [ ] Remove "Phase 3 — not yet implemented" comment
- [ ] Verify all documented vars match config crate
- [ ] Add note that all phases are complete

### Task 2.6: Verify README accuracy
- [ ] Every command in table works with `--help`
- [ ] Every env var in table exists in config
- [ ] Every path in table is accurate

---

## Slice 3: Demo script

### Task 3.1: Create docs/demo.md
- [ ] Write Track A: Packaged Demo (7 steps, < 5 minutes)
- [ ] Write Track B: Local/Private Demo (8 steps, < 5 minutes)
- [ ] Each step: command + expected output snippet
- [ ] Map each step to PRD acceptance criterion

### Task 3.2: Validate demo flow
- [ ] Run Track B locally end-to-end
- [ ] Verify all commands produce expected output
- [ ] Verify provider disclosure appears
- [ ] Verify evaluation passes

---

## Slice 4: Release CI

### Task 4.1: Create release workflow
- [ ] Create `.github/workflows/release.yml`
- [ ] Trigger on tag push `v*`
- [ ] Build matrix: linux-x86_64, windows-x86_64, macos-arm64
- [ ] Smoke test each binary (`cite health --json`)
- [ ] Upload artifacts

### Task 4.2: Release creation
- [ ] Use `softprops/action-gh-release` to create release
- [ ] Attach all 3 binaries
- [ ] Auto-generate release notes

### Task 4.3: Verify CI
- [ ] Workflow YAML is valid
- [ ] Matrix targets are correct
- [ ] Artifact names follow convention

---

## Final verification

- [ ] `cargo test` — all tests green
- [ ] `cargo clippy -- -D warnings` — clean
- [ ] `cargo fmt --check` — clean
- [ ] `cargo run -- evaluate` — 8/8 pass
- [ ] README complete and accurate
- [ ] Demo script works end-to-end
- [ ] Release workflow valid
- [ ] All acceptance criteria from proposal met

## Estimated lines per slice

| Slice | Code | Docs | Total |
|---|---|---|---|
| 1. Demo + disclosure | ~50 | ~240 | ~290 |
| 2. README + env | ~5 | ~200 | ~205 |
| 3. Demo script | 0 | ~150 | ~150 |
| 4. Release CI | ~80 | 0 | ~80 |
| **Total** | **~135** | **~590** | **~725** |

Review workload: docs-heavy, minimal code risk. Single PR acceptable.
