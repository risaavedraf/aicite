# Verification Evidence — Phase 8 Rename `harness` → `cite`

## Slice A

### Command
`cargo run --bin cite -- --help`

### Exit status
0

### Expected outcome
Help output shows `cite` as primary command name (`Usage:` starts with `cite`).

### Observed output (excerpt)
```text
Usage: cite.exe [OPTIONS] <COMMAND>
```

### Verdict
PASS

---

## Slice D closeout suite

### 1) CLI identity check

- Command: `cargo run --bin cite -- --help`
- Exit status: `0`
- Expected: primary command identity is `cite`
- Observed excerpt: `Usage: cite.exe [OPTIONS] <COMMAND>`
- Verdict: **PASS**

### 2) Regression tests

- Command: `cargo test`
- Exit status: `0`
- Expected: all tests pass
- Observed excerpt: `test result: ok` across workspace crates
- Verdict: **PASS**

### 3) Canonical docs command-surface grep

- Command:
  `rg -n "harness\s+(context|search|retrieve|ingest|list|get|trace|read|evaluate|refresh|retry)" README.md docs/demo.md docs/installation.md docs/agent-usage-guide.md docs/rename-to-cite.md`
- Exit status: `1`
- Expected: no matches
- Observed: no matches
- Verdict: **PASS**

### 4) Runtime naming still uses HARNESS_ in code

- Command: `rg -n "HARNESS_" crates/config crates/storage`
- Exit status: `0`
- Expected: one or more matches
- Observed excerpt:
  - `crates/config/src/lib.rs:193: runtime_mode: std::env::var("HARNESS_RUNTIME_MODE")...`
- Verdict: **PASS**

### 5) Runtime CITE_ naming not introduced in code

- Command: `rg -n "CITE_" crates/config crates/storage`
- Exit status: `1`
- Expected: no matches
- Observed: no matches
- Verdict: **PASS**

### 6) Deferral policy documented in checklist/docs

- Command: `rg -n "CITE_|HARNESS_" docs/sdd/phase-8-rename-cite/migration-checklist.md docs/installation.md`
- Exit status: `0`
- Expected: explicit Phase 8/Phase 9 deferral mentions
- Observed excerpt:
  - `docs/installation.md:316: ... HARNESS_* ... CITE_* ... deferred to Phase 9`
  - `docs/sdd/phase-8-rename-cite/migration-checklist.md:12: HARNESS_* -> CITE_*`
- Verdict: **PASS**

### 7) Data/db naming remains deferred (no runtime rename)

- Command: `rg -n "harness\.db" README.md crates/storage/src/lib.rs`
- Exit status: `0`
- Expected: runtime/docs still reference `harness.db` in Phase 8
- Observed excerpt:
  - `crates/storage/src/lib.rs:24: let db_path = data_dir.join("harness.db")`
  - `README.md:186: harness.db ...`
- Verdict: **PASS**

## Phase 8 acceptance gate

All required verification checks passed.

**Gate verdict: PASS**
