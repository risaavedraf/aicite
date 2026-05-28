# Phase 9 Migration Checklist — Installation + Runtime Naming

## Scope

This checklist covers local migration to the Phase 9 canonical setup:
- Runtime env namespace: `CITE_*`
- Local naming: `.../cite/` data dir with `cite.db`
- Canonical command pathways: dev run, local built binary, installed release binary

## 1) Pre-checks

1. Save a backup of your current env/config values.
2. Check current runtime variables:
   - `env | grep -E "^(CITE_|HARNESS_)"` (Linux/macOS)
   - `Get-ChildItem Env: | ? Name -match '^(CITE_|HARNESS_)'` (PowerShell)
3. Confirm current data directory and DB location (if custom).
4. Confirm CLI is callable in your chosen pathway:
   - Dev run: `cargo run --bin cite -- --help`
   - Local built binary: `./target/release/cite --help`
   - Installed binary: `cite --help` or `cite.exe --help`

## 2) Migration steps

1. Update shell scripts, `.env`, and CI snippets to `CITE_*` variables.
2. If you used legacy `HARNESS_*`, replace with equivalent `CITE_*` keys.
3. Ensure config/data naming points to `cite` paths and `cite.db`.
4. Keep one canonical invocation style per context:
   - Dev: `cargo run --bin cite -- <command> ...`
   - Built local binary: `./target/release/cite <command> ...`
   - Installed release binary: `cite <command> ...` / `cite.exe <command> ...`

## 3) Validation commands and expected outcomes

### Runtime + build checks

- `cargo run --bin cite -- --help`
  - Expected: usage shows `cite` / `cite.exe`, command exits 0.
- `cargo build --release`
  - Expected: build success, exit 0.
- `./target/release/cite health --json`
  - Expected: JSON includes `"status": "ok"`, exit 0.

### Docs/policy consistency checks

- `rg -n "cargo run --bin cite|target/release/cite|cite.exe|Path A|Path B|Path C" README.md docs/installation.md docs/demo.md docs/agent-usage-guide.md`
  - Expected: hits show explicit canonical pathway references.
- `rg -n "Runtime naming policy|CITE_\*|HARNESS_\*|cite\.db|not auto|GEMINI_API_KEY|OPENAI_API_KEY" README.md docs/installation.md docs/rename-to-cite.md .env.example`
  - Expected: one coherent runtime policy statement set.
- `rg -n "CITE_\*\s*[-=]?>\s*CITE_\*|cite\.db\s*[-=]?>\s*cite\.db|harness\s*[-=]?>\s*harness" README.md docs/installation.md docs/rename-to-cite.md .env.example`
  - Expected: no matches (exit 1), proving contradictory self-mappings were removed.

### Code-policy alignment checks

- `rg -n "std::env::var\(\"CITE_" crates/config crates/cli`
  - Expected: matches present (runtime reads canonical `CITE_*`).
- `rg -n "join\(\"cite\"\)|cite\.db" crates/cli crates/storage`
  - Expected: matches present (`cite` dir + `cite.db` naming in code).

## 4) Rollback steps

If migration breaks local workflows:

1. Restore previous `.env` / shell exports from backup.
2. Restore previous script aliases/automation references.
3. Re-run health in last-known-good pathway.
4. If needed, pin to previous docs/commit and re-apply migration incrementally.

### Rollback validation

- `cargo run --bin cite -- --help` (or your prior pathway equivalent)
- `./target/release/cite health --json`

Expected: command exits 0 and health status is `ok`.
