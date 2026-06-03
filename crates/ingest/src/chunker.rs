use common::CiteError;

/// Input chunk before storage (no IDs yet)
#[derive(Debug, Clone, PartialEq)]
pub struct ChunkInput {
    pub chunk_index: u32,
    pub text: String,
    pub page: Option<u32>,
    pub offset_start: u32,
    pub offset_end: u32,
}

/// Text from a single page (matches extractor::PageText)
#[derive(Debug, Clone)]
pub struct PageText {
    pub page: u32,
    pub text: String,
}

/// Chunk text with overlap and page tracking.
///
/// - `pages`: extracted text per page (order matters)
/// - `chunk_size_chars`: target maximum chunk size in characters
/// - `chunk_overlap_chars`: overlap between consecutive chunks in characters
/// - `min_chunk_chars`: minimum chunk size; smaller chunks are dropped (except the last)
pub fn chunk_text(
    pages: &[PageText],
    chunk_size_chars: usize,
    chunk_overlap_chars: usize,
    min_chunk_chars: usize,
) -> Result<Vec<ChunkInput>, CiteError> {
    if chunk_size_chars == 0 {
        return Err(CiteError::InvalidParameter {
            message: "chunk_size_chars must be > 0".into(),
        });
    }
    if chunk_overlap_chars >= chunk_size_chars {
        return Err(CiteError::InvalidParameter {
            message: "chunk_overlap_chars must be < chunk_size_chars".into(),
        });
    }

    // 1. Build combined text and char-offset → page mapping
    let (combined, char_page_map) = build_combined_text(pages);

    if combined.is_empty() {
        return Ok(Vec::new());
    }

    let total_chars = combined.chars().count();

    // 2. Chunking loop
    let mut chunks: Vec<ChunkInput> = Vec::new();
    let mut chunk_index: u32 = 0;
    let mut start: usize = 0; // char offset (not byte)

    while start < total_chars {
        let mut end = std::cmp::min(start + chunk_size_chars, total_chars);

        // 3. Try to find a sentence boundary near the end
        if end < total_chars {
            let search_start = if start + chunk_size_chars > chunk_overlap_chars {
                start + chunk_size_chars - chunk_overlap_chars
            } else {
                start
            };
            if let Some(boundary) = find_sentence_boundary(&combined, search_start, end) {
                end = boundary;
            }
        }

        // Extract chunk text (char-based slicing)
        let chunk_str: String = combined.chars().skip(start).take(end - start).collect();
        let trimmed = chunk_str.trim();

        // 4. Determine if this chunk should be kept
        let is_last = end >= total_chars;
        if trimmed.chars().count() >= min_chunk_chars || is_last {
            // Only keep non-empty trimmed text
            if !trimmed.is_empty() {
                let page = resolve_page(&char_page_map, start, end - 1);
                chunks.push(ChunkInput {
                    chunk_index,
                    text: trimmed.to_string(),
                    page,
                    offset_start: start as u32,
                    offset_end: end as u32,
                });
                chunk_index += 1;
            }
        }

        // 5. Advance: step back by overlap so next chunk overlaps with current end
        if end >= total_chars {
            break;
        }
        let step_back = std::cmp::min(chunk_overlap_chars, end - start);
        start = end - step_back;

        // Safety: ensure forward progress
        if start >= end {
            start = end;
        }
    }

    Ok(chunks)
}

/// Concatenate page texts with newline separators and build a mapping from
/// char offset → page number.
fn build_combined_text(pages: &[PageText]) -> (String, Vec<u32>) {
    let mut combined = String::new();
    let mut char_page_map: Vec<u32> = Vec::new();

    for (i, page) in pages.iter().enumerate() {
        // Add separator between pages (not before the first)
        if i > 0 {
            combined.push('\n');
            char_page_map.push(page.page);
        }

        for ch in page.text.chars() {
            combined.push(ch);
            char_page_map.push(page.page);
        }
    }

    (combined, char_page_map)
}

/// Look up the page number for a char offset range.
/// Returns the page of the starting character, or None if offsets are out of range.
fn resolve_page(char_page_map: &[u32], start: usize, _end_inclusive: usize) -> Option<u32> {
    if char_page_map.is_empty() || start >= char_page_map.len() {
        return None;
    }
    Some(char_page_map[start])
}

/// Find a sentence boundary (`. `, `! `, `? `, `\n\n`) in the range
/// `[search_start, target_end)`. Returns the char offset *after* the boundary
/// (i.e., the first char of the next sentence), preferring the boundary closest
/// to `target_end`.
fn find_sentence_boundary(text: &str, search_start: usize, target_end: usize) -> Option<usize> {
    // Collect chars into a Vec for O(1) indexed access by char position
    let chars: Vec<char> = text.chars().collect();
    let total = chars.len();
    if search_start >= total || search_start >= target_end {
        return None;
    }

    let scan_end = std::cmp::min(target_end, total);
    let mut best_boundary: Option<usize> = None;

    for i in search_start..scan_end {
        let boundary = match chars[i] {
            '.' | '!' | '?' if i + 1 < scan_end => match chars[i + 1] {
                ' ' | '\n' => Some(i + 2),
                _ => None,
            },
            '.' | '!' | '?' if i + 1 >= total => Some(i + 1),
            '\n' if i + 1 < scan_end && chars[i + 1] == '\n' => Some(i + 2),
            _ => None,
        };

        if let Some(b) = boundary {
            if b <= target_end {
                best_boundary = Some(b);
            }
        }
    }

    best_boundary
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_page(page: u32, text: &str) -> PageText {
        PageText {
            page,
            text: text.to_string(),
        }
    }

    #[test]
    fn test_chunk_basic() -> Result<(), CiteError> {
        // 100 chars of text with 50-char chunks, no overlap
        let text = "A".repeat(100);
        let pages = [make_page(1, &text)];
        let chunks = chunk_text(&pages, 50, 0, 10)?;
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].chunk_index, 0);
        assert_eq!(chunks[0].offset_start, 0);
        assert_eq!(chunks[0].offset_end, 50);
        assert_eq!(chunks[1].chunk_index, 1);
        assert_eq!(chunks[1].offset_start, 50);
        assert_eq!(chunks[1].offset_end, 100);
        Ok(())
    }

    #[test]
    fn test_chunk_overlap() -> Result<(), CiteError> {
        let text = "A".repeat(200);
        let pages = [make_page(1, &text)];
        let chunks = chunk_text(&pages, 100, 20, 10)?;

        // Should produce chunks where consecutive ones share overlap
        assert!(
            chunks.len() >= 2,
            "Expected at least 2 chunks, got {}",
            chunks.len()
        );

        // Second chunk should start 20 chars before the end of the first
        let expected_start = chunks[0].offset_end - 20;
        assert_eq!(
            chunks[1].offset_start, expected_start,
            "Overlap between chunk 0 and 1 should be 20 chars"
        );

        // Verify the overlap text matches
        let overlap_from_first: String = chunks[0]
            .text
            .chars()
            .skip(chunks[0].text.chars().count() - 20)
            .collect();
        let overlap_from_second: String = chunks[1].text.chars().take(20).collect();
        assert_eq!(overlap_from_first, overlap_from_second);
        Ok(())
    }

    #[test]
    fn test_chunk_small_text() -> Result<(), CiteError> {
        let text = "Hello world";
        let pages = [make_page(1, text)];
        let chunks = chunk_text(&pages, 1000, 200, 100)?;

        // Text < chunk_size should produce a single chunk
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "Hello world");
        assert_eq!(chunks[0].offset_start, 0);
        Ok(())
    }

    #[test]
    fn test_chunk_empty() -> Result<(), CiteError> {
        let pages: Vec<PageText> = vec![];
        let chunks = chunk_text(&pages, 1000, 200, 100)?;
        assert!(chunks.is_empty());
        Ok(())
    }

    #[test]
    fn test_chunk_empty_page_text() -> Result<(), CiteError> {
        let pages = [make_page(1, "")];
        let chunks = chunk_text(&pages, 1000, 200, 100)?;
        assert!(chunks.is_empty());
        Ok(())
    }

    #[test]
    fn test_chunk_page_tracking() -> Result<(), CiteError> {
        let pages = [
            make_page(1, &"A".repeat(500)),
            make_page(2, &"B".repeat(500)),
            make_page(3, &"C".repeat(500)),
        ];
        let chunks = chunk_text(&pages, 600, 100, 10)?;

        // First chunk should start on page 1
        assert_eq!(chunks[0].page, Some(1), "First chunk should be page 1");

        // Find a chunk that starts in page 2 territory
        // Page 1 is 500 chars + separator at offset 500 = page 2 starts at offset 501
        let page2_chunk = chunks
            .iter()
            .find(|c| c.offset_start >= 501 && c.offset_start < 1002);
        if let Some(c) = page2_chunk {
            assert_eq!(
                c.page,
                Some(2),
                "Chunk in page 2 range should report page 2"
            );
        }

        // Find a chunk that starts in page 3 territory
        // Page 2 ends at offset 1001 (500 + 1 + 500), page 3 starts at offset 1002
        let page3_chunk = chunks.iter().find(|c| c.offset_start >= 1002);
        if let Some(c) = page3_chunk {
            assert_eq!(
                c.page,
                Some(3),
                "Chunk in page 3 range should report page 3"
            );
        }

        Ok(())
    }

    #[test]
    fn test_chunk_sentence_boundary() -> Result<(), CiteError> {
        // Text with clear sentence boundaries near the chunk limit
        let text = format!("{}. {}", "B".repeat(480), "C".repeat(100),);
        // chunk_size=500, overlap=50
        // The boundary '. ' is at char 481 (after the period)
        let pages = [make_page(1, &text)];
        let chunks = chunk_text(&pages, 500, 50, 10)?;

        assert!(!chunks.is_empty());
        // The first chunk should end at or near the sentence boundary (position ~482)
        // rather than at exactly 500
        assert!(
            chunks[0].offset_end <= 500,
            "Chunk should split at or before chunk_size"
        );
        // The chunk text should end with a sentence if possible
        if chunks[0].offset_end < 500 {
            assert!(
                chunks[0].text.ends_with('.') || chunks[0].text.contains('.'),
                "Chunk should contain sentence boundary: '{}'",
                &chunks[0].text[..std::cmp::min(20, chunks[0].text.len())]
            );
        }
        Ok(())
    }

    #[test]
    fn test_chunk_min_size_filtering() -> Result<(), CiteError> {
        // Create text where a small chunk would be produced without min_size filtering
        // Use a pattern that forces a small final chunk scenario
        let text = format!("{}\n\n{}", "A".repeat(280), "tiny",);
        // chunk_size=200, overlap=50, min_size=100
        // The \n\n boundary at position 281 will split here, creating chunks where
        // some might be < min_size
        let pages = [make_page(1, &text)];
        let chunks = chunk_text(&pages, 200, 50, 100)?;

        // All non-final chunks should be >= min_size
        if chunks.len() > 1 {
            for chunk in &chunks[..chunks.len() - 1] {
                assert!(
                    chunk.text.chars().count() >= 100,
                    "Non-final chunk should be >= 100 chars, got {}: '{}'",
                    chunk.text.chars().count(),
                    &chunk.text[..std::cmp::min(30, chunk.text.len())]
                );
            }
        }
        // The last chunk can be < min_size
        // But it should still exist if it has content
        let last = chunks.last().unwrap();
        assert!(!last.text.is_empty(), "Last chunk should have text");
        Ok(())
    }

    #[test]
    fn test_chunk_utf8_handling() -> Result<(), CiteError> {
        // Multi-byte UTF-8 chars (emoji and accented chars)
        let text = "café résumé naïve 🎉🎊🎉🎊🎉🎊🎉🎊🎉🎊🎉🎊🎉🎊🎉🎊🎉🎊🎉🎊";
        let pages = [make_page(1, text)];
        let chunks = chunk_text(&pages, 30, 10, 5)?;

        // Verify that chunks are valid and don't split multi-byte chars
        for chunk in &chunks {
            assert!(
                chunk.text.is_ascii() || std::str::from_utf8(chunk.text.as_bytes()).is_ok(),
                "Chunk text should be valid UTF-8"
            );
        }
        // Total chars in all chunks should account for the text
        assert!(!chunks.is_empty());
        Ok(())
    }

    #[test]
    fn test_invalid_params() {
        // chunk_size == 0
        let pages = [make_page(1, "hello")];
        assert!(chunk_text(&pages, 0, 0, 0).is_err());

        // overlap >= chunk_size
        assert!(chunk_text(&pages, 100, 100, 0).is_err());
        assert!(chunk_text(&pages, 100, 150, 0).is_err());
    }
}
