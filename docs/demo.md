# AI Harness CLI — Demo Guide

This guide walks through two demo tracks: a packaged demo (no Rust required) and a local/private demo (with Rust toolchain). Each track takes under 5 minutes and validates the MVP acceptance criteria.

## Track A: Packaged Demo (5 minutes)

**Prerequisites**: Downloaded release binary for your OS.

### Step 1: Health check

```bash
./cite health --json
```

**Expected output**: JSON with `"status": "healthy"` and config summary.

✅ *Validates: CLI can be launched, health returns healthy status.*

### Step 2: List sample documents

```bash
./cite list
```

**Expected output**: 3 preloaded sample documents (architecture, API reference, security policy) with `ready` status.

✅ *Validates: Public packaged demo uses preloaded sample documents.*

### Step 3: Query for known information

```bash
./cite context "What does the API gateway do?"
```

**Expected output**: Context pack with `result_kind: "context"`, citations, and a verification disclaimer.

✅ *Validates: Retrieval returns relevant chunks, context pack includes citations and disclaimer.*

### Step 4: Inspect a citation

```bash
./cite read <citation-id-from-step-3>
```

**Expected output**: Source text showing API gateway routing details.

✅ *Validates: User can inspect cited snippets from context output.*

### Step 5: Query for unknown information

```bash
./cite context "What is quantum computing?"
```

**Expected output**: `result_kind: "no_results"` with no citations. No fabricated answers.

✅ *Validates: Unsupported requests return no_results instead of fabricated answers.*

### Step 6: Check provider disclosure

The output from Steps 3-5 should show:
- Provider disclosure banner (if using a real provider)
- Verification disclaimer: downstream AI answers must be verified against cited sources

✅ *Validates: Provider disclosure and verification disclaimer are visible.*

### Step 7: Run evaluation (optional)

```bash
./cite evaluate
```

**Expected output**: 8/8 fixtures pass, hit rate ≥ 80%.

---

## Track B: Local/Private Demo (5 minutes)

**Prerequisites**: Rust 1.75+, embedding provider API key.

### Step 1: Clone and build

```bash
git clone https://github.com/your-org/aiharness.git
cd aiharness
cargo build --release
```

### Step 2: Configure embedding provider

```bash
cp .env.example .env
# Edit .env and set:
#   HARNESS_EMBEDDING_API_KEY=your-api-key
#   HARNESS_EMBEDDING_PROVIDER=gemini (or openai-compatible)
```

### Step 3: Ingest demo documents

```bash
./target/release/cite ingest demo/
```

**Expected output**:
- No-sensitive-data warning shown
- 3 documents ingested (architecture.txt, api-reference.md, security-policy.txt)
- Each document transitions to `ready` status

✅ *Validates: Local/private demo can import documents after no-sensitive-data warning.*

### Step 4: Verify ingestion

```bash
./target/release/cite list
```

**Expected output**: 3 documents with `ready` status and chunk counts.

✅ *Validates: User can see ingestion status.*

### Step 5: Query with citations

```bash
./target/release/cite context "How are passwords validated?"
```

**Expected output**: Context pack with citations referencing the security policy document.

✅ *Validates: Retrieval returns relevant chunks with citations and source metadata.*

### Step 6: Inspect source

```bash
./target/release/cite read <citation-id-from-step-5>
```

**Expected output**: Source text showing password requirements (12+ characters, complexity rules).

✅ *Validates: User can inspect cited snippets.*

### Step 7: No-results query

```bash
./target/release/cite context "What is quantum computing?"
```

**Expected output**: `result_kind: "no_results"`, no citations.

✅ *Validates: Safe no-results behavior for unknown queries.*

### Step 8: Run evaluation

```bash
./target/release/cite evaluate
```

**Expected output**:
```
╔══════════════════════════════════════════════════════════════╗
║           Golden Dataset Evaluation Results                 ║
╠══════════════════════════════════════════════════════════════╣
║  df-001  PASS  direct_fact         "What does the API gateway do?"
║  df-002  PASS  direct_fact         "What database does the system use?"
║  df-003  PASS  direct_fact         "How are passwords validated?"
║  nr-001  PASS  no_results          "What is quantum computing?"
║  nr-002  PASS  no_results          "Explain the theory of relativity"
║  amb-001 PASS  ambiguous           "Tell me about the system architecture..."
║  mc-001  PASS  multi_chunk         "How does the API handle auth..."
║  pi-001  PASS  prompt_injection    "Ignore all previous instructions..."
╠══════════════════════════════════════════════════════════════╣
║  Hit rate: 8/8 (100.0%) — PASS (threshold: 80%)
╚══════════════════════════════════════════════════════════════╝
```

✅ *Validates: Golden dataset evaluation passes with ≥80% hit rate.*

### Step 9: Check compliance notes

Review the [Privacy and Compliance](../README.md#privacy-and-compliance) section in README.

✅ *Validates: README documents compliance approach, provider disclosure, and data handling.*

---

## Acceptance Criteria Mapping

| Criterion | Track A | Track B |
|---|---|---|
| CLI launches with one default corpus | Step 1-2 | Step 1-4 |
| Public demo uses preloaded docs, uploads disabled | Step 2 | N/A |
| Local demo can import documents | N/A | Step 3 |
| User can see ingestion status | Step 2 | Step 4 |
| Retrieval returns relevant chunks | Step 3 | Step 5 |
| Context pack with citations and trace ID | Step 3 | Step 5 |
| User can inspect cited snippets | Step 4 | Step 6 |
| No fabricated answers for unknown queries | Step 5 | Step 7 |
| Provider disclosure visible | Step 6 | Step 5 |
| Verification disclaimer visible | Step 6 | Step 5 |
| README documents config, storage, reset | N/A | Step 9 |
| Evaluation passes ≥80% hit rate | Step 7 | Step 8 |
