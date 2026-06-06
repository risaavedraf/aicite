-- Track when the active snapshot pointer was last written.
-- Additive and nullable so older databases can migrate without table rebuild.
ALTER TABLE snapshot_pointer ADD COLUMN updated_at TEXT;

-- Backfill existing pointer rows with SQLite's UTC timestamp format, matching
-- storage::util::format_dt / parse_dt (`%Y-%m-%d %H:%M:%S`).
UPDATE snapshot_pointer
SET updated_at = datetime('now')
WHERE updated_at IS NULL;
