# Rename and Runtime Naming Policy

## CLI identity

The CLI command is `cite`.

- Product name: **CITE**
- Command name: `cite`

## Runtime naming policy (Phase 9)

This project now documents one canonical runtime naming set:

- Environment namespace: `CITE_*`
- Config directory name: `cite`
- Data directory name: `cite`
- SQLite file name: `cite.db`

### Code-aligned compatibility policy

- Legacy `HARNESS_*` runtime environment variables are **not** auto-aliased in runtime config loading.
- Legacy `harness` data/db path names are **not** auto-migrated by runtime code.
- Migration from legacy names is manual (documented in Phase 9 migration checklist).
- Embedding API key fallbacks `GEMINI_API_KEY` and `OPENAI_API_KEY` are still accepted by CLI embedding commands, but `CITE_EMBEDDING_API_KEY` remains canonical.

## Canonical env examples

```bash
CITE_CONFIG=/path/to/config.toml
CITE_DATA_DIR=/path/to/data
CITE_CACHE_DIR=/path/to/cache
CITE_RUNTIME_MODE=local_private_demo
CITE_EMBEDDING_PROVIDER=gemini
CITE_EMBEDDING_MODEL=gemini-embedding-001
CITE_EMBEDDING_API_KEY=your-key
CITE_TOP_K=5
```

## Canonical storage naming

- Data directory root: `.../cite/`
- SQLite database: `cite.db`
- SQLite WAL/SHM: `cite.db-wal`, `cite.db-shm`

## Migration note

If you previously used `HARNESS_*` env vars or `harness` path names, update your local shell/scripts/configs to the canonical names above. Runtime will not rewrite or alias those names automatically.
