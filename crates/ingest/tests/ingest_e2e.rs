use common::{CiteError, FileType};
use config::IngestConfig;
use ingest::chunker;
use ingest::extractor;
use ingest::validator;
use std::path::Path;

#[test]
fn test_ingest_txt_file_end_to_end() {
    let path = Path::new("tests/fixtures/sample.txt");
    let config = IngestConfig::default();

    // Validate
    let (file_type, size) = validator::validate_file(path, config.max_file_size_bytes).unwrap();
    assert_eq!(file_type, FileType::Txt);
    assert!(size > 0);

    // Extract
    let extraction = extractor::extract_text(path, &file_type).unwrap();
    assert!(!extraction.pages.is_empty());
    assert!(extraction.total_chars > 0);

    // Chunk
    let pages: Vec<chunker::PageText> = extraction
        .pages
        .iter()
        .map(|p| chunker::PageText {
            page: p.page,
            text: p.text.clone(),
        })
        .collect();
    let chunks = chunker::chunk_text(
        &pages,
        config.chunk_size_chars,
        config.chunk_overlap_chars,
        config.min_chunk_size_chars,
    )
    .unwrap();

    assert!(!chunks.is_empty());
    // Verify chunks have content
    for chunk in &chunks {
        assert!(!chunk.text.is_empty());
        assert!(chunk.offset_end > chunk.offset_start);
    }
}

#[test]
fn test_ingest_md_file_end_to_end() {
    let path = Path::new("tests/fixtures/sample.md");
    let config = IngestConfig::default();

    // Validate
    let (file_type, size) = validator::validate_file(path, config.max_file_size_bytes).unwrap();
    assert_eq!(file_type, FileType::Md);
    assert!(size > 0);

    // Extract
    let extraction = extractor::extract_text(path, &file_type).unwrap();
    assert!(!extraction.pages.is_empty());

    // Chunk
    let pages: Vec<chunker::PageText> = extraction
        .pages
        .iter()
        .map(|p| chunker::PageText {
            page: p.page,
            text: p.text.clone(),
        })
        .collect();
    let chunks = chunker::chunk_text(
        &pages,
        config.chunk_size_chars,
        config.chunk_overlap_chars,
        config.min_chunk_size_chars,
    )
    .unwrap();

    assert!(!chunks.is_empty());
}

#[test]
fn test_ingest_unsupported_file_type() {
    let path = Path::new("tests/fixtures/unsupported.csv");
    // Create a temp CSV file
    std::fs::write(path, "a,b,c").unwrap();

    let result = validator::validate_file(path, 1024 * 1024);
    assert!(matches!(result, Err(CiteError::UnsupportedFileType { .. })));

    // Cleanup
    let _ = std::fs::remove_file(path);
}

#[test]
fn test_ingest_file_too_large() {
    let path = Path::new("tests/fixtures/sample.txt");
    // Use a very small max size
    let result = validator::validate_file(path, 10);
    assert!(matches!(result, Err(CiteError::FileTooLarge { .. })));
}

#[test]
fn test_ingest_missing_file() {
    let path = Path::new("tests/fixtures/nonexistent.txt");
    let result = validator::validate_file(path, 1024 * 1024);
    assert!(matches!(result, Err(CiteError::FileNotFound { .. })));
}

#[test]
fn test_display_name_derivation() {
    let path = Path::new("tests/fixtures/sample.txt");

    // With override
    let name = validator::derive_display_name(path, Some("My Document"), false);
    assert_eq!(name, "My Document");

    // From path
    let name = validator::derive_display_name(path, None, false);
    assert_eq!(name, "sample.txt");

    // Production mode
    let name = validator::derive_display_name(path, None, true);
    assert_eq!(name, "document");
}

#[test]
fn test_chunker_overlap() {
    let pages = vec![chunker::PageText {
        page: 1,
        text: "A".repeat(3000),
    }];

    let chunks = chunker::chunk_text(&pages, 1000, 200, 100).unwrap();
    assert!(chunks.len() > 1);

    // Verify overlap exists between consecutive chunks
    for i in 1..chunks.len() {
        let prev_end = chunks[i - 1].offset_end;
        let curr_start = chunks[i].offset_start;
        assert!(
            curr_start < prev_end,
            "Expected overlap between chunks {} and {}",
            i - 1,
            i
        );
    }
}
