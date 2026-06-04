# Phase 9 Verification Evidence — Installation Experience

Date: 2026-05-28  
Scope: Slice C closeout evidence (with Slice A/B command re-validation)

## Acceptance gate summary

- [x] Canonical install/run pathways documented and executable
- [x] Release artifact naming/usage references are consistent in docs
- [x] Runtime naming migration policy is explicit and non-contradictory
- [x] Migration checklist includes validation + rollback commands
- [x] Evidence is auditable with command + result + outcome

## Command evidence

### 1) Local dev run help check

Command:
```bash
cargo run --bin cite -- --help
```

Exit: `0`  
Result: **PASS**

Key output:
```text
Usage: cite.exe [OPTIONS] <COMMAND>
```

---

### 2) Local release build check

Command:
```bash
cargo build --release
```

Exit: `0`  
Result: **PASS**

Key output:
```text
Finished `release` profile [optimized] target(s) in ...
```

---

### 3) Local release binary health check

Command:
```bash
./target/release/cite health --json
```

Exit: `0`  
Result: **PASS**

Key output:
```json
{
  "status": "ok",
  "version": "0.1.0",
  "schema_version": "context-v1"
}
```

---

### 4) Pathway consistency across canonical docs

Command:
```bash
rg -n "cargo run --bin cite|target/release/cite|cite.exe|Path A|Path B|Path C" README.md docs/installation.md docs/demo.md docs/agent-usage-guide.md
```

Exit: `0`  
Result: **PASS**

Evidence highlights:
- `README.md`: run matrix + Path A/B/C sections
- `docs/installation.md`: canonical pathway table + Path A/B/C sections
- `docs/demo.md`: dev/build/installed pathway markers
- `docs/agent-usage-guide.md`: dev + built + installed invocation examples

---

### 5) Runtime policy consistency across docs

Command:
```bash
rg -n "Runtime naming policy|CITE_\*|HARNESS_\*|cite\.db|not auto|GEMINI_API_KEY|OPENAI_API_KEY" README.md docs/installation.md docs/rename-to-cite.md .env.example
```

Exit: `0`  
Result: **PASS**

Evidence highlights:
- Canonical namespace documented as `CITE_*`
- Canonical local DB naming documented as `cite.db`
- Legacy `HARNESS_*` and legacy `harness` path names documented as manual migration (no auto-alias)
- API key fallback exception documented (`GEMINI_API_KEY` / `OPENAI_API_KEY`)

---

### 6) Contradictory self-mapping removal

Command:
```bash
rg -n "CITE_\*\s*[-=]?>\s*CITE_\*|cite\.db\s*[-=]?>\s*cite\.db|harness\s*[-=]?>\s*harness" README.md docs/installation.md docs/rename-to-cite.md .env.example
```

Exit: `1` (expected no matches)  
Result: **PASS**

---

### 7) Code alignment: env-var namespace usage

Command:
```bash
rg -n "std::env::var\(\"CITE_" crates/config crates/cli
```

Exit: `0`  
Result: **PASS**

Evidence highlights:
- `crates/config/src/lib.rs` reads `CITE_RUNTIME_MODE`, `CITE_DATA_DIR`, etc.
- `crates/cli/src/commands/*` reads `CITE_EMBEDDING_API_KEY`

---

### 8) Code alignment: canonical path/db naming

Command:
```bash
rg -n "join\(\"cite\"\)|cite\.db" crates/cli crates/storage
```

Exit: `0`  
Result: **PASS**

Evidence highlights:
- CLI path joins use `join("cite")`
- Storage uses `data_dir.join("cite.db")`

---

### 9) Release artifact naming references in docs

Command:
```bash
rg -n "releases/download|cite-(linux|macos|windows)|cite\.exe|bin\.install \"cite\"" docs/installation.md README.md docs/demo.md docs/agent-usage-guide.md
```

Exit: `0`  
Result: **PASS**

Evidence highlights:
- Download references use `cite-linux-amd64`, `cite-macos-*`, `cite-windows-amd64.exe`
- Installed executable references use `cite` / `cite.exe`
- Homebrew formula snippet installs binary as `cite`

## Rollback sanity checks

Rollback commands documented in:
- `docs/sdd/phase-9-installation-experience/migration-checklist.md`

Sanity validation commands (same as runtime checks):
- `cargo run --bin cite -- --help` → PASS
- `./target/release/cite health --json` → PASS
