# SDD Proposal — AI Cite CLI: MVP Implementation

## Change name

`mvp-scaffold` — Project scaffolding and core infrastructure

## Problem

The project is greenfield. No source code, no build system, no tests exist. Before any feature work can begin, the Rust workspace, module structure, config system, storage layer, CLI surface, and CI pipeline must be established.

## Proposed change

Set up the complete Rust workspace with the 9-crate module structure defined in the PRD, implement the config and storage foundations, create the CLI skeleton with a working `health` command, establish the common types and error contract, and wire up CI.

## Scope

### In scope
- Cargo workspace with 9 crates (engine, storage, config, graph, retrieval, ingest, providers, cli, common)
- Config crate: env vars, config file (TOML), CLI flags, precedence rules
- Storage crate: SQLite via rusqlite, WAL mode, migration system, schema for documents/chunks/embeddings
- CLI crate: clap-based command parsing, `health` command, `--json` output, exit codes
- Common crate: shared types (Document, Chunk, Citation, Error), error format, exit code enum
- CI: GitHub Actions for `cargo test`, `cargo clippy`, `cargo fmt --check`
- README skeleton with setup instructions, env vars, config paths
- `.env.example` with documented environment variables

### Out of scope
- Ingest pipeline (extraction, chunking, embedding)
- Retrieval pipeline (vector search, context packs)
- Provider integrations (OpenAI, local embeddings)
- Durable locks, rate limits, backlog records
- Golden dataset, sample corpus
- Packaging/distribution

## Acceptance criteria

1. `cargo build` compiles all 9 crates without errors
2. `cargo test` runs and passes (even if tests are minimal)
3. `cargo clippy -- -D warnings` passes
4. `cargo fmt --check` passes
5. `cite health --json` returns valid health JSON
6. Config loads from env vars, config file, and CLI flags with correct precedence
7. SQLite database initializes with migration system
8. CI pipeline runs on push/PR
9. README documents setup, env vars, config paths

## Risks

| Risk | Mitigation |
|---|---|
| Scope creep into feature work | Strict boundary: scaffolding only, no retrieval/ingest logic |
| Over-engineering config system | Start with env vars + TOML file, add complexity later |
| SQLite migration complexity | Simple version table + numbered migrations |

## Estimated size

~800-1200 lines of Rust + TOML + YAML + Markdown

## Sequencing

This is Phase 1 of the MVP. Subsequent phases:
- Phase 2: Ingest pipeline (ingest, providers, engine)
- Phase 3: Retrieval pipeline (retrieval, graph, engine)
- Phase 4: Context packs + citations
- Phase 5: Durability (locks, rate limits, backlog)
- Phase 6: Evaluation (golden dataset, sample corpus)
- Phase 7: Packaging + docs
