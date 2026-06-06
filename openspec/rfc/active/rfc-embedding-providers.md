# RFC: Pluggable Embedding Provider System

**Status:** Draft
**Author:** el Gentleman + rikar
**Created:** 2026-06-06
**Related:** rfc-tags-and-note-add.md, EVALUACION_CITE.md, CITE_Pi_Integration.md

---

## Problem

### 1. Hardcoded provider dependency

Cite's embedding pipeline is locked to cloud APIs (Gemini, OpenAI-compatible). Users can't choose local models, alternative APIs, or self-hosted solutions. This creates:

- **Rate limit dependency** — Gemini 429 errors block ingestion (5 docs failed)
- **Cost dependency** — per-token API costs scale with corpus size
- **Latency dependency** — ~1000ms+ per query via network
- **Quality ceiling** — stuck on general-purpose models, can't use domain-specific ones

### 2. GPU underutilization

Users with capable GPUs (RTX 3070 8GB, etc.) can't leverage them for embedding. Local models that would run in <50ms on GPU are forced through CPU or cloud APIs.

### 3. Model lock-in

When a better embedding model emerges (e.g., qwen3-embedding, future models), there's no clean migration path. Users must wait for a Cite release that explicitly adds support.

---

## Current State

The architecture already has the right abstractions:

```rust
// crates/providers/src/lib.rs
pub trait EmbeddingProvider {
    fn embed(&self, text: &str) -> Result<Embedding, CiteError>;
    fn model_id(&self) -> &str;
    fn provider_id(&self) -> &str;
}
```

**Existing implementations:**
- `GeminiProvider` — Google Gemini API (gemini-embedding-001)
- `OpenAICompatibleProvider` — Any OpenAI-compatible API (/v1/embeddings)

**Existing config:**
```toml
[provider]
type = "gemini"              # or "openai-compatible"
model = "gemini-embedding-001"
api_key = "..."
```

**Missing:**
- Local model providers (ONNX, Ollama, Candle)
- Device selection (CPU/GPU)
- Batch embedding (current trait is single-text only)
- Re-embed migration command
- Provider auto-detection and validation

---

## Proposal

### Part 1: Extend the Provider Trait

The current `embed(&self, text: &str)` is single-text only. Add batch support and capability metadata:

```rust
pub trait EmbeddingProvider: Send + Sync {
    /// Embed a single text
    fn embed(&self, text: &str) -> Result<Embedding, CiteError>;

    /// Embed multiple texts in one call (batch optimization)
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, CiteError> {
        // Default: sequential fallback
        texts.iter().map(|t| self.embed(t)).collect()
    }

    /// Model identifier
    fn model_id(&self) -> &str;

    /// Provider identifier
    fn provider_id(&self) -> &str;

    /// Embedding dimensions
    fn dimensions(&self) -> usize;

    /// Max input tokens (None = no limit / provider handles truncation)
    fn max_tokens(&self) -> Option<usize>;

    /// Whether this provider requires network access
    fn is_local(&self) -> bool {
        false
    }
}
```

**Why batch matters:** Ollama and ONNX can process multiple texts in one forward pass, which is 3-10x faster than sequential for ingestion workloads.

### Part 2: New Provider Implementations

#### 2a. OllamaProvider (HTTP to local Ollama)

Ollama is the easiest path to local GPU embedding. Install Ollama, pull a model, and Cite connects via HTTP.

```rust
pub struct OllamaProvider {
    client: Client,
    model: String,
    endpoint: String,    // default: http://localhost:11434
    dimensions: usize,   // queried from Ollama or configured
}
```

**Config:**
```toml
[provider]
type = "ollama"
model = "nomic-embed-text"          # or "qwen3-embedding:4b", "bge-m3"
endpoint = "http://localhost:11434" # default
```

**API:**
```
POST http://localhost:11434/api/embed
{
  "model": "nomic-embed-text",
  "input": ["search_document: text to embed", ...]
}
```

**Advantages:**
- Zero build dependencies (just HTTP)
- GPU automatic (Ollama manages CUDA)
- Supports all major embedding models
- Batch support native

**Disadvantages:**
- Requires Ollama installed and running
- HTTP overhead (~5-10ms per call)

#### 2b. OnnxProvider (local ONNX Runtime)

For users who want embedding without external services. ONNX Runtime runs models directly in-process.

```rust
pub struct OnnxProvider {
    session: ort::Session,
    tokenizer: Tokenizer,
    model_name: String,
    dimensions: usize,
    max_tokens: usize,
}
```

**Config:**
```toml
[provider]
type = "onnx"
model_path = "./models/nomic-embed-text-v1.5/model.onnx"
tokenizer_path = "./models/nomic-embed-text-v1.5/tokenizer.json"
device = "cuda"     # or "cpu"
dimensions = 768
```

**Advantages:**
- In-process, no external dependencies
- GPU via CUDA execution provider
- Fastest possible latency (~10-30ms GPU, ~30-70ms CPU)

**Disadvantages:**
- User must download model files manually
- ONNX models not available for all architectures
- Adds `ort` crate dependency (~50MB binary size increase)

#### 2c. HuggingFace API Provider (free tier available)

For users who want better models than Gemini without running local.

```toml
[provider]
type = "huggingface"
model = "nomic-ai/nomic-embed-text-v1.5"
api_key = "YOUR_HF_KEY"
```

Uses the HuggingFace Inference API. Free tier available with rate limits.

### Part 3: Configuration Schema

Extend `EmbeddingConfig` in `crates/config/src/lib.rs`:

```rust
pub struct EmbeddingConfig {
    pub provider: String,           // "gemini", "openai", "ollama", "onnx", "huggingface"
    pub model: String,              // model identifier
    pub api_key: Option<String>,    // for API providers
    pub endpoint: Option<String>,   // custom endpoint URL
    pub device: Option<String>,     // "cpu", "cuda", "auto" (for local providers)
    pub dimensions: Option<usize>,  // override model defaults (Matryoshka)
    pub batch_size: Option<usize>,  // batch size for embed_batch (default: 32)
}
```

**TOML config:**
```toml
# Option A: Cloud API (existing)
[provider]
type = "gemini"
model = "gemini-embedding-001"
api_key = "YOUR_GEMINI_KEY"

# Option B: Ollama local (recommended for GPU users)
[provider]
type = "ollama"
model = "qwen3-embedding:4b"

# Option C: ONNX local (no external deps)
[provider]
type = "onnx"
model_path = "./models/nomic-embed-text-v1.5/model.onnx"
device = "cuda"

# Option D: OpenAI-compatible (any API)
[provider]
type = "openai"
model = "text-embedding-3-small"
api_key = "YOUR_OPENAI_KEY"
endpoint = "https://api.openai.com/v1/embeddings"
```

**Environment variables:**
```bash
CITE_EMBEDDING_PROVIDER=ollama
CITE_EMBEDDING_MODEL=qwen3-embedding:4b
CITE_EMBEDDING_ENDPOINT=http://localhost:11434
CITE_EMBEDDING_DEVICE=cuda
CITE_EMBEDDING_DIMENSIONS=768
```

### Part 4: Provider Registry and Auto-Detection

```rust
pub fn create_provider(config: &EmbeddingConfig) -> Result<Box<dyn EmbeddingProvider>, CiteError> {
    match config.provider.as_str() {
        "gemini" => Ok(Box::new(GeminiProvider::new(...)?)),
        "openai" | "openai-compatible" => Ok(Box::new(OpenAICompatibleProvider::new(...)?)),
        "ollama" => Ok(Box::new(OllamaProvider::new(...)?)),
        "onnx" => Ok(Box::new(OnnxProvider::new(...)?)),
        "huggingface" => Ok(Box::new(HuggingFaceProvider::new(...)?)),
        other => Err(CiteError::ConfigError {
            message: format!(
                "Unknown embedding provider '{}'. Supported: gemini, openai, ollama, onnx, huggingface",
                other
            ),
        }),
    }
}
```

**Health check with provider validation:**

```bash
cite health
# Output:
# Provider: ollama (qwen3-embedding:4b)
# Endpoint: http://localhost:11434
# Dimensions: 2560
# Device: cuda (RTX 3070)
# Status: OK (12ms latency)
```

### Part 5: Re-embed Command

When switching providers/models, all existing embeddings must be regenerated:

```bash
# Re-embed everything with current provider config
cite ingest --reembed

# Re-embed with a specific provider (overrides config temporarily)
cite ingest --reembed --provider ollama --model qwen3-embedding:4b

# Re-embed only documents that failed previously
cite ingest --reembed --failed-only
```

**Migration flow:**
1. User changes provider in config
2. `cite ingest --reembed` reads all documents from DB
3. Re-chunks if chunking strategy changed
4. Re-embeds with new provider
5. Updates vectors in-place (or alongside with swap)
6. Reports: "Re-embedded 186 chunks in 4.2s (was 180s with Gemini)"

**Safety:** keep old vectors during migration, swap atomically on success.

### Part 6: Embedding Dimension Flexibility

Models with Matryoshka support (nomic, qwen3, gemini) can output smaller dimensions:

```toml
[provider]
type = "ollama"
model = "qwen3-embedding:4b"
dimensions = 512  # truncate from native 2560
```

**Trade-off table (user-facing in docs):**

| Dimensions | MTEB impact | Storage per 1K chunks | Retrieval quality |
|---|---|---|---|
| 2560 (native) | baseline | 10MB | best |
| 1024 | -0.5% | 4MB | excellent |
| 768 | -1% | 3MB | very good |
| 512 | -2% | 2MB | good |
| 256 | -4% | 1MB | acceptable |

### Part 7: `cite doctor` — Full Pipeline Diagnostics

A single command that checks everything and reports actionable status:

```bash
cite doctor
```

**Output:**
```
Cite Diagnostics
================

Config
  ✓ Config found: ~/.config/cite/config.toml
  ✓ Provider: ollama (qwen3-embedding:4b)

Provider
  ✓ Ollama running at localhost:11434
  ✓ Model loaded: qwen3-embedding:4b
  ✓ VRAM usage: 2.5GB / 8GB (RTX 3070)
  ✓ Embed latency: 12ms (measured)

Database
  ✓ Database: ~/AppData/Roaming/cite/cite.db
  ✓ Documents: 20 ready, 0 failed
  ✓ Chunks: 186
  ✓ Topics: 152, Concepts: 70

Embeddings
  ⚠ Model mismatch: DB has gemini-embedding-001 vectors
    → Run 'cite ingest --reembed' to unify with current provider
  ✓ Dimensions: 768 (consistent)

Retrieval
  ✓ FTS5 index: active
  ✓ Hierarchy: enabled
  ✗ 5 docs have stale embeddings (different model)
    → Run 'cite ingest --reembed --failed-only'

Overall: 2 warnings, 1 error
Run 'cite doctor --fix' to attempt automatic fixes.
```

**Flags:**
- `--fix` — attempt automatic fixes (rebuild FTS, retry failed docs)
- `--json` — machine-readable output
- `--provider-only` — check only provider connectivity

### Part 8: Actionable Error Messages

Every provider error should guide the user toward a solution:

```rust
// Current:
Err(CiteError::EmbeddingProviderError {
    message: "Gemini API returned HTTP 429: Too Many Requests".to_string()
})

// Proposed:
Err(CiteError::EmbeddingProviderError {
    message: "Gemini rate limit hit (HTTP 429).\n\
        → Wait 60s and retry, or\n\
        → Switch to local provider: CITE_EMBEDDING_PROVIDER=ollama\n\
        → See 'cite doctor' for provider status".to_string()
})
```

**Error catalog:**

| Error | Current message | Actionable message |
|---|---|---|
| 429 rate limit | "HTTP 429" | "Rate limit hit. Wait 60s or switch to local: CITE_EMBEDDING_PROVIDER=ollama" |
| Connection refused | "Request failed" | "Cannot reach {endpoint}. Is the provider running? For Ollama: 'ollama serve'" |
| Model not found | "HTTP 404" | "Model '{model}' not found. For Ollama: 'ollama pull {model}'. See 'cite doctor'" |
| Invalid API key | "HTTP 401" | "Invalid API key. Set CITE_EMBEDDING_API_KEY or check config.toml" |
| Timeout | "Request timed out" | "Provider timeout ({timeout}s). Increase CITE_EMBEDDING_TIMEOUT or check network" |
| OOM on GPU | "CUDA out of memory" | "Model too large for GPU. Try: dimensions=512 or device=cpu. See 'cite doctor'" |
| Lock stale | "durable_lock is stale" | "Stale lock from crashed process. Run 'cite ingest --force' to clear" |

### Part 9: Resumable Ingestion

Current state: if ingestion fails mid-way (429, crash, OOM), the user must manually clean locks or re-ingest everything.

#### `--resume` flag

```bash
# Resume from where ingestion stopped
cite ingest openspec/ --recursive --resume

# Output:
# Resuming ingestion... found 186/200 chunks already embedded
# Processing remaining 14 chunks... done (2.3s)
# Total: 200 chunks across 20 documents
```

**Logic:**
1. Check DB for existing chunks with valid embeddings
2. Skip documents that are fully ingested
3. For partially ingested docs, re-chunk and embed only missing chunks
4. Clear stale locks automatically (with warning)

#### `--retry-failed` flag

```bash
# Retry only documents that failed previous ingestion
cite ingest --retry-failed

# Output:
# Found 5 failed documents:
#   - 03-mvp-scope.md (error: HTTP 429)
#   - 06-ux-flows.md (error: HTTP 429)
#   - prd_changelog.md (error: HTTP 429)
#   - installation.md (error: HTTP 429)
#   - v0.2.0-hierarchical-graph.md (error: HTTP 429)
# Retrying with current provider (ollama)... 5/5 succeeded (1.8s)
```

#### `--force` flag (lock cleanup)

```bash
# Clear stale locks and retry
cite ingest --force openspec/

# Equivalent to the manual SQL cleanup:
# DELETE FROM durable_locks; DELETE FROM ingest_backlog;
```

---

## Provider Comparison Matrix

| Provider | Latency | GPU | Batch | Cost | Setup effort |
|---|---|---|---|---|---|
| **gemini** | ~1000ms | N/A | no | $0.15/1M tokens | API key |
| **openai** | ~200ms | N/A | yes | $0.02-0.13/1M tokens | API key |
| **ollama** | ~15-50ms | auto | yes | $0 | install ollama + pull model |
| **onnx** | ~10-70ms | manual | yes | $0 | download model files |
| **huggingface** | ~300ms | N/A | yes | free tier | API key |

---

## Implementation Plan

### Phase 1: Ollama Provider (v0.3.2) — Highest impact/effort ratio

- [ ] `OllamaProvider` struct + `EmbeddingProvider` impl
- [ ] Batch support in trait (`embed_batch` with default fallback)
- [ ] Config extension: `endpoint`, `device`, `dimensions`
- [ ] `cite health` shows provider details + latency
- [ ] Provider factory (`create_provider`)
- [ ] Tests: unit + integration (requires Ollama running)

**Why first:** Ollama is the easiest path for users. No model files to manage, GPU automatic, HTTP-based. Covers 80% of the "I want local embeddings" use case.

### Phase 2: Re-embed Migration (v0.3.2)

- [ ] `cite ingest --reembed` command
- [ ] Atomic vector swap (old → new on success)
- [ ] Progress reporting: "Re-embedded 45/186 chunks..."
- [ ] `--failed-only` flag for retry
- [ ] Benchmark: old vs new model on golden dataset

### Phase 3: ONNX Provider (v0.3.3) — Zero-dependency local

- [ ] `OnnxProvider` with `ort` crate
- [ ] CUDA execution provider support
- [ ] Model auto-download from HuggingFace (optional)
- [ ] Binary size impact assessment

### Phase 4: HuggingFace API Provider (v0.3.3)

- [ ] `HuggingFaceProvider` using Inference API
- [ ] Free tier rate limit handling
- [ ] Model auto-discovery (validate model exists)

### Phase 5: Diagnostics & UX (v0.3.2, alongside Phase 1)

- [ ] `cite doctor` command — full pipeline diagnostics
- [ ] Actionable error messages for all provider errors
- [ ] `cite ingest --resume` — resumable ingestion
- [ ] `cite ingest --retry-failed` — retry only failed docs

**Why alongside Phase 1:** These are small, independent changes that dramatically improve UX. `cite doctor` is especially important for the Ollama provider — users need to verify their setup works.

### Phase 6: Setup Wizard (v0.3.3)

- [ ] `cite setup` interactive wizard
- [ ] GPU/hardware auto-detection
- [ ] Ollama auto-install offer
- [ ] Provider recommendation based on hardware

### Phase 7: Advanced (v0.4.0+)

- [ ] `cite benchmark` — compare providers on current corpus
- [ ] Provider fallback chain (ollama → gemini if local unavailable)
- [ ] Embedding cache (skip re-embed for unchanged chunks)
- [ ] Model download wizard (`cite setup --provider ollama`)
- [ ] `cite ingest --rebuild-fts` — rebuild FTS5 index

---

## Migration Path

### Current users (Gemini)

```toml
# Before (works as-is, no change required)
[provider]
type = "gemini"
model = "gemini-embedding-001"
api_key = "YOUR_GEMINI_KEY"
```

No breaking changes. Gemini continues to work exactly as before.

### Users switching to Ollama

```bash
# 1. Install Ollama
# 2. Pull model
ollama pull nomic-embed-text

# 3. Update config
# cite.toml:
# [provider]
# type = "ollama"
# model = "nomic-embed-text"

# 4. Re-embed
cite ingest --reembed
```

### Users switching to ONNX

```bash
# 1. Download model (one-time)
mkdir -p models
# ... download nomic-embed-text-v1.5 ONNX files

# 2. Update config
# cite.toml:
# [provider]
# type = "onnx"
# model_path = "./models/nomic-embed-text-v1.5/model.onnx"
# device = "cuda"

# 3. Re-embed
cite ingest --reembed
```

---

## Open Questions

1. **ONNX model distribution:** Should Cite auto-download ONNX models from HuggingFace on first use, or require manual download?
2. **Binary size:** ONNX Runtime adds ~50MB to the binary. Should it be a feature flag (`cargo build --features onnx`)?
3. **Provider fallback:** If Ollama is down, should Cite fall back to a configured cloud provider automatically?
4. **Embedding cache:** Should we cache embeddings to avoid re-embedding unchanged chunks? What invalidation strategy?
5. **Dimension migration:** If a user reduces dimensions (e.g., 768→512), do we need a full re-embed or can we truncate existing vectors?
6. **Doctor auto-fix:** Should `cite doctor --fix` be destructive (clear locks, re-embed) or only suggest commands?
7. **Setup wizard scope:** Should `cite setup` install Ollama automatically, or just guide the user through manual steps?

---

## References

- CITE_Pi_Integration.md — Nomic recommendation, resource tables
- EVALUACION_CITE.md — Current Gemini quality issues (scores 0.63-0.76)
- rfc-tags-and-note-add.md — Retrieval quality roadmap (v0.3.2-0.5.0)
- [Ollama Embedding Models](https://www.morphllm.com/ollama-embedding-models) — Provider comparison benchmarks
- [Nomic Embed Text v1.5](https://huggingface.co/nomic-ai/nomic-embed-text-v1.5) — Model card, MTEB scores
- [Qwen3 Embedding](https://github.com/QwenLM/Qwen3-Embedding) — State-of-the-art local model
- [Gemini Embedding Technical Report](https://arxiv.org/abs/2503.07891) — MTEB 68.32 benchmark details
