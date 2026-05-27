use std::path::Path;

use common::{FileType, HarnessError};
use lopdf::Document;

/// Text from a single page.
#[derive(Debug, Clone)]
pub struct PageText {
    pub page: u32,
    pub text: String,
}

/// Result of text extraction from a document.
#[derive(Debug, Clone)]
pub struct ExtractionResult {
    pub pages: Vec<PageText>,
    pub total_chars: usize,
}

/// Extract text from a file, dispatching by file type.
pub fn extract_text(path: &Path, file_type: &FileType) -> Result<ExtractionResult, HarnessError> {
    match file_type {
        FileType::Txt | FileType::Md => extract_plain_text(path),
        FileType::Pdf => extract_pdf_text(path),
    }
}

/// Extract text from a plain-text file (TXT or MD).
///
/// Returns a single page (page=1) containing the entire file content.
/// Fails with `InternalError` if the file is not valid UTF-8.
fn extract_plain_text(path: &Path) -> Result<ExtractionResult, HarnessError> {
    let content = std::fs::read_to_string(path).map_err(|e| HarnessError::InternalError {
        message: format!("Failed to read file {}: {}", path.display(), e),
    })?;

    let total_chars = content.len();

    if content.is_empty() {
        return Ok(ExtractionResult {
            pages: vec![],
            total_chars: 0,
        });
    }

    Ok(ExtractionResult {
        pages: vec![PageText {
            page: 1,
            text: content,
        }],
        total_chars,
    })
}

/// Extract text from a PDF file using lopdf.
///
/// Returns one `PageText` per page in the document. Pages with no extractable
/// text (e.g. scanned images without OCR) produce an empty string.
/// Fails with `InternalError` on corrupted or unreadable PDFs.
fn extract_pdf_text(path: &Path) -> Result<ExtractionResult, HarnessError> {
    let doc = Document::load(path).map_err(|e| HarnessError::InternalError {
        message: format!("Failed to load PDF {}: {}", path.display(), e),
    })?;

    let pages_map = doc.get_pages();
    let mut pages = Vec::with_capacity(pages_map.len());
    let mut total_chars = 0usize;

    for &page_num in pages_map.keys() {
        let text = doc.extract_text(&[page_num]).unwrap_or_default();
        total_chars += text.len();
        pages.push(PageText {
            page: page_num,
            text,
        });
    }

    // Sort by page number to guarantee ordering
    pages.sort_by_key(|p| p.page);

    Ok(ExtractionResult { pages, total_chars })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    /// Helper: write content to a temp file and return its path.
    fn create_temp_file(name: &str, content: &[u8]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("ingest_extractor_tests");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(content).unwrap();
        path
    }

    #[test]
    fn test_extract_txt() {
        let path = create_temp_file("hello.txt", b"Hello, world!\nLine two.");
        let result = extract_text(&path, &FileType::Txt).unwrap();

        assert_eq!(result.pages.len(), 1);
        assert_eq!(result.pages[0].page, 1);
        assert_eq!(result.pages[0].text, "Hello, world!\nLine two.");
        assert_eq!(result.total_chars, 23);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_extract_md() {
        let content = "# Title\n\nSome **bold** text.\n";
        let path = create_temp_file("doc.md", content.as_bytes());
        let result = extract_text(&path, &FileType::Md).unwrap();

        assert_eq!(result.pages.len(), 1);
        assert_eq!(result.pages[0].page, 1);
        assert_eq!(result.pages[0].text, content);
        assert_eq!(result.total_chars, content.len());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_extract_empty_file() {
        let path = create_temp_file("empty.txt", b"");
        let result = extract_text(&path, &FileType::Txt).unwrap();

        assert_eq!(result.pages.len(), 0);
        assert_eq!(result.total_chars, 0);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_extract_invalid_utf8() {
        // 0xFF 0xFE is not valid UTF-8
        let path = create_temp_file("bad.txt", &[0xFF, 0xFE, 0x00]);
        let result = extract_text(&path, &FileType::Txt);

        assert!(result.is_err(), "Expected error for invalid UTF-8");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("Failed to read file"),
            "Unexpected error message: {}",
            err
        );

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_extract_txt_whitespace_only() {
        let path = create_temp_file("spaces.txt", b"   \n\n  ");
        let result = extract_text(&path, &FileType::Txt).unwrap();

        // Non-empty content, even if only whitespace
        assert_eq!(result.pages.len(), 1);
        assert_eq!(result.pages[0].page, 1);
        assert_eq!(result.pages[0].text, "   \n\n  ");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_extract_md_empty() {
        let path = create_temp_file("empty.md", b"");
        let result = extract_text(&path, &FileType::Md).unwrap();

        assert_eq!(result.pages.len(), 0);
        assert_eq!(result.total_chars, 0);

        let _ = fs::remove_file(&path);
    }
}
