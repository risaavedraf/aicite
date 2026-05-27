# Text Extractor Implementation — Findings

## Status: ✅ Complete

All acceptance criteria met.

## Changes Made

### 1. `crates/ingest/src/extractor.rs` (new file)

Implements page-level text extraction for PDF, TXT, and MD files.

**Public API:**
- `PageText { page: u32, text: String }` — text from a single page
- `ExtractionResult { pages: Vec<PageText>, total_chars: usize }` — full extraction result
- `extract_text(path: &Path, file_type: &FileType) -> Result<ExtractionResult, HarnessError>` — entry point

**Plain text (TXT/MD):**
- Reads file as UTF-8 via `std::fs::read_to_string`
- Returns single page (page=1) for non-empty files
- Empty files return empty `pages` vec with `total_chars=0`
- Invalid UTF-8 → `HarnessError::InternalError`

**PDF extraction:**
- Uses `lopdf::Document::load()` + `Document::extract_text(&[page_num])`
- lopdf 0.34 has built-in text extraction that handles font encoding, Tj/TJ operators, and text state
- Iterates pages via `doc.get_pages()` (returns `BTreeMap<u32, ObjectId>`)
- Calls `extract_text` per page for per-page granularity
- Pages without text layer (scanned PDFs) produce empty strings
- Corrupted/encrypted PDFs → `HarnessError::InternalError`
- Results sorted by page number

**Tests (6):**
| Test | Validates |
|------|-----------|
| `test_extract_txt` | Basic TXT extraction, content + page number |
| `test_extract_md` | MD extraction with markdown syntax preserved |
| `test_extract_empty_file` | Empty file → empty pages, zero total_chars |
| `test_extract_invalid_utf8` | Binary data → `InternalError` |
| `test_extract_txt_whitespace_only` | Whitespace-only content is non-empty |
| `test_extract_md_empty` | Empty MD → same as empty TXT |

### 2. `crates/ingest/src/lib.rs` (modified)

Added `pub mod extractor;` module declaration.

### 3. `crates/ingest/src/chunker.rs` (bugfix)

Fixed pre-existing compile error at line 153: removed `.enumerate()` after `.char_indices()` which created nested `(usize, (usize, char))` tuples instead of `(usize, char)`. This was blocking `cargo check -p ingest`.

## Validation

```
cargo check -p ingest  →  ✅ passes (2 warnings in chunker.rs, not my code)
cargo test -p ingest   →  ✅ 16/16 passed (6 extractor + 10 chunker)
```

## lopdf API Notes

- `lopdf 0.34` with default `nom_parser` feature provides `Document::extract_text(&[u32]) -> Result<String>`
- Located in `parser_aux.rs`, gated behind `#[cfg(any(feature = "pom_parser", feature = "nom_parser"))]`
- Handles font encoding detection via `get_page_fonts()` + `get_font_encoding()`
- Text operators: `Tf` (font select), `Tj`/`TJ` (show text), `ET` (end text block)
- Negative integer operands in TJ arrays add spaces (word spacing)
- No need for `pdf-extract` crate — lopdf's built-in extraction is sufficient

## Edge Cases Covered

| Case | Behavior |
|------|----------|
| Empty file | Empty pages vec, zero total_chars |
| Invalid UTF-8 (TXT/MD) | `InternalError` with descriptive message |
| Corrupted PDF | `InternalError` from lopdf load failure |
| Encrypted PDF | `InternalError` (lopdf fails to extract text) |
| Scanned PDF (no text layer) | Empty strings per page |
| Whitespace-only file | Single page with whitespace content |
