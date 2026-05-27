# Text Chunker Implementation — `crates/ingest/src/chunker.rs`

## Summary

Implemented text chunking with overlap and page tracking for the AI Harness ingest crate.

## Changed Files

| File | Change |
|------|--------|
| `crates/ingest/src/chunker.rs` | **New** — full chunker module (285 lines) |
| `crates/ingest/src/lib.rs` | Added `pub mod chunker;` declaration |

## Public API

### Types

- **`ChunkInput`** — Pre-storage chunk with `chunk_index`, `text`, `page`, `offset_start`, `offset_end`. Derives `Debug, Clone, PartialEq`.
- **`PageText`** — Per-page text input with `page: u32` and `text: String`. Derives `Debug, Clone`.

### Function

```rust
pub fn chunk_text(
    pages: &[PageText],
    chunk_size_chars: usize,
    chunk_overlap_chars: usize,
    min_chunk_size_chars: usize,
) -> Result<Vec<ChunkInput>, HarnessError>
```

## Algorithm

1. **Concatenation** — Page texts joined with `\n` separators; a parallel `Vec<u32>` maps each char offset to its source page number.
2. **Char-index loop** — Uses `chars().count()` / `.skip().take()` for all slicing — correct for multi-byte UTF-8.
3. **Sentence boundary detection** — Scans `[search_start, end)` for `. `, `! `, `? ` (followed by space/newline) or `\n\n`. Prefers the boundary closest to `target_end`.
4. **Overlap** — After emitting a chunk, `start = end - overlap`. If `overlap >= chunk_size`, parameter validation rejects it upfront.
5. **Min-size filtering** — Chunks shorter than `min_chunk_size_chars` are dropped, except the last chunk.

## Validation

- `chunk_size_chars == 0` → `InvalidParameter`
- `chunk_overlap_chars >= chunk_size_chars` → `InvalidParameter`
- Empty input / empty page text → empty `Vec` (no error)

## Test Results

```
running 10 tests
test chunker::tests::test_chunk_basic ... ok
test chunker::tests::test_chunk_overlap ... ok
test chunker::tests::test_chunk_small_text ... ok
test chunker::tests::test_chunk_empty ... ok
test chunker::tests::test_chunk_empty_page_text ... ok
test chunker::tests::test_chunk_page_tracking ... ok
test chunker::tests::test_chunk_sentence_boundary ... ok
test chunker::tests::test_chunk_min_size_filtering ... ok
test chunker::tests::test_chunk_utf8_handling ... ok
test chunker::tests::test_invalid_params ... ok

test result: ok. 10 passed; 0 failed; 0 ignored
```

All 10 chunker tests pass. `cargo check -p ingest` clean.

> **Note:** 5 pre-existing failures in `validator.rs` (Windows path handling — UNC/device path detection) are unrelated to this change.

## Design Decisions

| Decision | Rationale |
|----------|-----------|
| Char-based offsets (not byte) | Required by spec; handles UTF-8 emoji and accented chars correctly |
| Page separator = `\n` | Minimal, predictable; separator char is attributed to the next page |
| `resolve_page` returns start-of-chunk page | Most natural for citation display; the mapping array supports future range queries |
| Trim chunk text | Removes accidental leading/trailing whitespace from split points |
| O(n) char collection in `find_sentence_boundary` | Avoids O(n²) from repeated `.nth()` on `Chars` iterator |

## Open Risks

- **Large documents**: `build_combined_text` and `find_sentence_boundary` both allocate `Vec<char>` / `Vec<u32>` proportional to full text size. For very large docs (millions of chars), consider streaming or page-level chunking.
- **`resolve_page` unused parameter**: `_end_inclusive` is kept for API completeness; could be used for range-based page queries later.

## Recommended Next Step

Wire `chunk_text` into the ingest pipeline by calling it from the document processing flow in `Ingest`, using `IngestConfig` defaults for chunk parameters.
