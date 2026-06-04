# Migration Checklist — Phase 8 (`cite` → `cite`)

## Scope

Phase 8 migrates CLI command identity to `cite`.

Runtime naming is intentionally unchanged in this phase:
- keep `CITE_*` environment variables
- keep existing local data/db path naming

Deferred to Phase 9:
- `CITE_*` → `CITE_*`
- data/db path rename (for example `cite.db` → `cite.db`)

## Local checklist (single-user)

1. Update aliases/scripts from `cite` command calls to `cite`.
2. Run `cargo run --bin cite -- --help` and confirm `Usage:` shows `cite`.
3. Verify your `.env` or shell still uses `CITE_*` variables.
4. Verify local data directory/database paths remain unchanged for now.
5. If a command fails after rename, roll back command aliases/scripts to previous state and re-run health checks.

## Rollback

- Revert CLI command aliases from `cite` back to `cite` (if needed).
- Restore previous docs/scripts from version control.
- Keep runtime env/data naming untouched (`CITE_*` still valid in Phase 8).
