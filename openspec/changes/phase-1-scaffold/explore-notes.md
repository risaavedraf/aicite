# SDD Explore Notes — AI Cite CLI

## Project thesis

CLI-first semantic document engine that ingests private documents, creates vector embeddings, and exposes cited retrieval through stable CLI commands. External agents consume context packs; the engine does NOT generate answers.

## Current state

- **Greenfield**: No Cargo.toml, no source code, no tests
- **PRD**: Complete with 17 documents covering product brief, users, MVP scope, functional/non-functional requirements, UX flows, system architecture, AI/retrieval design, API contract, acceptance criteria, risks, legal, ethics, future roadmap, market landscape
- **Decision**: Rust engine, SQLite persistence, Tokio async, vector-first retrieval

## Key architectural decisions (from PRD)

1. **9-crate module structure**: engine, storage, config, graph, retrieval, ingest, providers, cli, common
2. **Single-shot durable process model**: No daemon. Each command starts, does work, persists state, exits
3. **SQLite WAL mode**: Concurrent reads, sequential writes with durable locks
4. **Vector-first retrieval**: Cosine similarity, no keyword fallback, no reranking in MVP
5. **Context pack contract**: Agent-consumable JSON with citations, scores, trace IDs
6. **Runtime modes**: public_packaged_demo (uploads disabled), local_private_demo (imports allowed), production (blocked)
7. **Durable locks + backlog**: Sequential ingestion, atomic backlog upsert on lock conflict
8. **Rate limiting**: Durable counters keyed by runtime_mode + corpus_id + provider_id + retrieval_scope

## MVP scope boundaries

**In scope:**
- Ingest PDF, TXT, MD files
- Build semantic structure (document → section → chunk)
- Generate embeddings via configurable provider
- Vector search with cosine similarity
- Context packs with citations, scores, trace IDs
- CLI commands: health, ingest, refresh, list, get, search, retrieve, context, read, trace, retry
- Durable locks, rate limits, backlog records
- Structured logging with safe-field allowlist
- Golden dataset for retrieval evaluation (8 fixtures minimum)
- Sample corpus (3+ docs, 10+ facts, structured content)

**Out of scope (post-MVP):**
- Built-in answer generation / LLM adapter
- MCP server/access wrapper
- Native app / desktop UI
- Full hybrid vector + keyword ranking
- Reranking model
- Multi-user auth
- Multiple named corpora
- Full delete API with retention
- Agent workflows

## Critical success factors

1. **Agent-consumable JSON**: Stable schema, machine-readable errors, exit codes
2. **Citations first-class**: Every chunk traceable to source with page/offset
3. **No hallucination**: Explicit no_results/insufficient_context instead of fabricating
4. **Provider abstraction**: Swappable embedding providers without product changes
5. **Durable state**: No process-local sessions; all state survives CLI restarts
6. **Privacy guardrails**: Chile privacy law baseline, provider disclosure, data minimization

## Open questions / risks

1. **Embedding provider choice**: Which provider for MVP? OpenAI-compatible? Local/mock for testing?
2. **Vector index**: SQLite-based or separate? SQLite-vss? Raw cosine in-memory?
3. **Chunking strategy**: Token-based or character-based? Overlap implementation details?
4. **PDF extraction**: Which Rust crate? Quality tradeoffs?
5. **Sample corpus**: What documents to include? Need 3+ docs, 10+ facts, structured content
6. **Golden dataset**: 8 fixtures minimum, must be co-authored with sample corpus
7. **Config format**: TOML? YAML? Env-only?
8. **Schema migrations**: How to version SQLite schema with CLI versions?

## Suggested first change

Given greenfield status, the first SDD change should be **project scaffolding + core infrastructure**:

- Cargo workspace with 9 crates
- Config crate with env/file/flag precedence
- Storage crate with SQLite + migrations
- CLI crate with clap + basic health command
- Common crate with shared types, errors, exit codes
- CI pipeline (GitHub Actions)
- README skeleton

This establishes the foundation for all subsequent work.

## Dependencies / sequencing

```
Phase 1: Scaffolding (config, storage, cli, common)
Phase 2: Ingest pipeline (ingest, providers, engine)
Phase 3: Retrieval pipeline (retrieval, graph, engine)
Phase 4: Context packs + citations (engine, cli)
Phase 5: Durability (locks, rate limits, backlog)
Phase 6: Evaluation (golden dataset, sample corpus)
Phase 7: Packaging + docs
```
