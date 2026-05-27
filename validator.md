# File Validator Implementation — Findings

## Status: COMPLETE

## Summary

Implemented file validation module for the AI Harness CLI ingest crate. The validator provides path safety checks, file type/size validation, display name derivation, and input sanitization.

## Changes

### New file: `crates/ingest/src/validator.rs`

**Public functions:**

| Function | Signature | Description |
|---|---|---|
| `validate_file` | `(path: &Path, max_file_size_bytes: u64) -> Result<(FileType, u64), HarnessError>` | Full validation pipeline: path safety → canonicalize → network/device rejection → existence → type → size |
| `derive_display_name` | `(path: &Path, override_name: Option<&str>, production_mode: bool) -> String` | Display name with priority: override → production generic → path filename |
| `sanitize_display_name` | `(name: &str) -> String` | Remove path separators, null bytes, control chars; strip leading dots; trim; truncate to 255 chars |
| `is_path_safe` | `(path: &Path) -> Result<(), HarnessError>` | Pre-access path policy: reject `..`, UNC, `/dev/`, `\\.\` |

**Private helpers:**
- `reject_network_path` — post-canonicalize UNC check (handles Windows `\\?\` extended-length paths correctly)
- `reject_device_path` — post-canonicalize device file check

### Modified file: `crates/ingest/src/lib.rs`

Added `pub mod validator;` to expose the module.

## Validation pipeline (validate_file)

```
1. is_path_safe(path)           — reject traversal, UNC, devices BEFORE any I/O
2. std::fs::canonicalize(path)  — resolve symlinks; map error to FileNotFound
3. reject_network_path(&canonical) — re-check after resolve (handles \\?\ on Windows)
4. reject_device_path(&canonical)  — re-check after resolve
5. canonical.is_file()           — must be regular file
6. FileType::from_extension()    — map extension to Pdf/Txt/Md; UnsupportedFileType if None
7. metadata.len() > max          — FileTooLarge if exceeds max_file_size_bytes
```

## Windows compatibility note

`std::fs::canonicalize` on Windows returns extended-length paths like `\\?\C:\Users\...`. The validator explicitly allows this `\\?\` prefix while still rejecting actual network paths (`\\server\share`, `\\?\UNC\server\share`). This was a real test failure discovered during implementation.

## Tests (17 tests, all passing)

| Test | What it verifies |
|---|---|
| `test_validate_txt_file` | Valid .txt file passes with correct type and size |
| `test_validate_unsupported_type` | .csv file returns UnsupportedFileType |
| `test_validate_missing_file` | Nonexistent path returns FileNotFound |
| `test_validate_path_traversal` | Path with `..` returns PathRejected |
| `test_validate_file_too_large` | File exceeding max returns FileTooLarge with correct sizes |
| `test_derive_display_name_with_override` | Override name takes priority |
| `test_derive_display_name_with_override_sanitized` | Override is sanitized (seps removed, dots stripped) |
| `test_derive_display_name_from_path` | Falls back to path filename |
| `test_derive_display_name_production_mode` | Returns generic "document" in production |
| `test_derive_display_name_production_overridden_by_override` | Override wins even in production |
| `test_sanitize_display_name` | Various cases: normal, traversal, null bytes, empty result |
| `test_sanitize_display_name_truncation` | Long names truncated to 255 chars |
| `test_is_path_safe_normal` | Normal paths pass |
| `test_is_path_safe_traversal` | `..` components rejected |
| `test_is_path_safe_unc` | UNC `//server` rejected |
| `test_is_path_safe_device` | `/dev/` rejected |
| `test_derive_display_name_production_overridden_by_override` | Edge case |

## Build & test results

```
cargo check -p ingest   → OK
cargo test -p ingest    → 32 passed (17 new validator + 15 existing)
```

## Open items / Risks

- **No `tempfile` dependency**: Tests create temp files with hardcoded names in `std::env::temp_dir()`. Adequate for CI but not parallel-safe. Consider adding `tempfile` dev-dependency if parallel test flakiness appears.
- **Windows UNC detection**: The `is_path_safe` function checks `\\` prefix for UNC. On Windows, `\\?\` extended-length local paths are allowed through. Extended-length UNC (`\\?\UNC\`) is rejected by the post-canonicalize check in `reject_network_path`.
- **Symlink cycle**: `canonicalize` will error on broken symlinks, which we map to `FileNotFound`. Symlink cycles are caught by the OS and also produce `FileNotFound`. No special handling needed.
- **No `.md`/`.markdown` direct test**: The `from_extension` logic for `.md`/`.markdown` is tested indirectly through the common crate; validator tests only cover `.txt` and `.csv`.

## Next steps

- Wire `validate_file` into the ingest pipeline (call before extraction/chunking)
- Consider adding `tempfile` as a dev-dependency for cleaner test isolation
- Add `.md` file validation test if desired for completeness
