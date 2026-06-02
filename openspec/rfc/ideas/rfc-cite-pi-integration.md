# RFC: CITE-Pi Integration (Extension / Skill)

## Status: Draft

This RFC defines how **CITE integrates with Pi** as a native extension or skill, keeping the embedding model loaded in memory for low-latency retrieval. It covers architecture, resource trade-offs, operation modes, and the extension structure needed to make CITE feel like a first-class tool inside Pi.

## Quick path

1. Choose embedding model (Nomic-embed-text-v1.5 recommended for balance).
2. Decide operation mode: Pi-integrated, standalone CLI, or hybrid.
3. Design the extension manifest and tool registration.
4. Implement lazy model loading on first skill invocation.
5. Measure latency before/after integration.

## Problem

CITE currently uses Gemini Embedding via API, which introduces ~1s latency per query. For a CLI-first engine meant to serve AI agents, this latency is unacceptable for interactive workflows. The ideal solution is a local embedding model loaded once and reused across queries.

Additionally, CITE needs a tight integration with Pi so that retrieval feels like a native capability, not an external CLI call.

## Goals

1. **Persistent model loading**: Load the embedding model once when the Pi extension activates, reuse across all queries.
2. **Low-latency retrieval**: Target 20-80ms per embedding (CPU) or 10-25ms (GPU).
3. **Native Pi integration**: Expose CITE as a skill/extension with tools like `cite-search`, `cite-ingest`, `cite-list`.
4. **Backward compatibility**: Standalone CLI mode continues to work independently.
5. **Lazy initialization**: Don't slow down Pi startup — load the model on first skill use.

## Non-goals

- Replace CITE's CLI with a daemon.
- Build a custom embedding training pipeline.
- Support remote/vector-database backends (SQLite remains the store).
- Add answer generation inside CITE (retrieval-only by design).

## Proposed approach

### Architecture

```
Pi starts
  └─ Extension loads (manifest.json)
       └─ Model NOT loaded yet (lazy)
            └─ User/agent calls cite-search
                 └─ model_manager loads embedding model (once)
                 └─ Subsequent queries reuse loaded model (~20-80ms)
```

### Extension structure

```
cite-pi-extension/
├── manifest.json
├── src/
│   ├── main.rs              # Extension registration with Pi
│   ├── cite_engine.rs       # Wrapper around CITE Rust engine
│   ├── model_manager.rs     # Embedding model lifecycle
│   └── tools/
│       ├── search.rs        # cite-search tool
│       └── ingest.rs        # cite-ingest tool
├── models/                  # Downloaded embedding models (optional)
└── Cargo.toml
```

### Key components

| Component | Responsibility |
|---|---|
| `model_manager.rs` | Load model once, expose `embed(text) -> Vec<f32>` |
| `cite_engine.rs` | Bridge between Pi tools and CITE's internal engine |
| `tools/search.rs` | Expose `cite-search` to Pi agents |
| `tools/ ingest.rs` | Expose `cite-ingest` to Pi agents |

### Operation modes

| Mode | Description | Use case |
|---|---|---|
| **Pi-integrated** (recommended) | Pi loads extension → model loads on first use → tools available | Daily use with Pi |
| **Standalone CLI** | `cite search` works as before, independent of Pi | Scripts, CI, non-Pi workflows |
| **Hybrid** | Extension detects if CITE daemon is running; if not, loads its own model | Transition period |

### Model selection

| Model | Dims | RAM | CPU latency | GPU latency | Notes |
|---|---|---|---|---|---|
| all-MiniLM-L6-v2 | 384 | 150-300 MB | 15-40 ms | - | Fast prototyping |
| **Nomic-embed-text-v1.5** | 768 | 400-700 MB | 30-70 ms | 10-25 ms | **Recommended balance** |
| BAAI/bge-m3 | 1024 | 700-1100 MB | 50-120 ms | 15-40 ms | Best multilingual |
| Snowflake-arctic-embed-l | 1024 | 800-1300 MB | 60-150 ms | 20-50 ms | Maximum quality |

**Default recommendation**: Start with **Nomic-embed-text-v1.5** (768 dims, ~400-700 MB RAM).

## Interfaces

### Pi tool registration (pseudo)

```json
{
  "name": "cite-search",
  "description": "Semantic search over CITE knowledge base",
  "parameters": {
    "query": { "type": "string", "required": true },
    "limit": { "type": "integer", "default": 5 },
    "min_score": { "type": "number", "default": 0.3 }
  }
}
```

### Internal call path

```
Pi agent calls cite-search(query)
  → extension receives call
  → model_manager.embed(query)  // model already loaded
  → cite_engine.search(vector, limit, min_score)
  → return results with citations
```

## Migration path

1. Current CLI remains untouched — no breaking changes.
2. Extension is additive: installs alongside existing CITE.
3. If extension is not installed, CLI works as before.
4. When model changes, all vectors in SQLite must be regenerated (same as today).

## Risks

| Risk | Mitigation |
|---|---|
| Model loading adds ~2-5s on first call | Lazy loading + user-facing message |
| 700 MB extra RAM on user machine | Document requirement; offer lighter model as fallback |
| Model re-ingest on upgrade | Version embeddings table; detect model mismatch |
| Extension maintenance burden | Keep thin — delegate to existing CITE engine |

## Open questions

1. Should the extension bundle the model or download it on first use?
2. Should we expose a `cite status` tool to check if the model is loaded?
3. How do we handle model upgrades — auto-re-ingest or warn-and-prompt?
4. Should the extension support GPU acceleration, or CPU-only for MVP?
5. Do we need a `cite unload` tool to free memory when not in use?

## Review plan

- [ ] Confirm Nomic-embed-text-v1.5 as default model.
- [ ] Confirm lazy loading strategy (first-call vs. explicit preload).
- [ ] Decide: bundle model or download-on-demand.
- [ ] Decide: GPU support in MVP or CPU-only.
- [ ] Design manifest.json schema for Pi extension registration.
- [ ] Validate backward compatibility with standalone CLI.
- [ ] Define model versioning and re-ingest policy.

## Related docs

- [RFC: Front-Lobe Engine](./rfc-front-lobe-engine.md) — orchestration layer that uses CITE as evidence store
- [RFC: Hybrid Notes Ingestion](./rfc-notes-hybrid.md) — notes as evidence in CITE
- [System Architecture](../../prd/07-system-architecture.md)
