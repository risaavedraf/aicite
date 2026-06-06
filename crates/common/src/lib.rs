pub mod error;
pub mod exit;
pub mod types;

pub use error::CiteError;
pub use exit::ExitCode;
pub use types::{
    Chunk, ChunkId, Citation, ConceptId, ContextMetadata, ContextMetadataScaffold, ContextResponse,
    Document, DocumentId, DocumentStatus, FileType, ReadResponse, ReadSelector, ResultKind,
    TopicId, TraceCitationRecord, TraceEnvelope, TraceHeaderInput, TraceHeaderRecord, TraceId,
    TraceResponse,
};

/// Count Unicode characters (not bytes) in a string.
pub fn char_len(s: &str) -> usize {
    s.chars().count()
}

/// Truncate a string to at most `max_chars` Unicode characters.
/// Returns an owned String to avoid lifetime issues.
pub fn char_truncate(s: &str, max_chars: usize) -> String {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => s[..idx].to_string(),
        None => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_len_ascii() {
        assert_eq!(char_len("hello"), 5);
    }

    #[test]
    fn test_char_len_cjk() {
        assert_eq!(char_len("日本語"), 3);
    }

    #[test]
    fn test_char_len_emoji() {
        assert_eq!(char_len("🎉🎊"), 2);
    }

    #[test]
    fn test_char_len_mixed() {
        // H,e,l,l,o + sp + 日,本,語 + sp + 🎉 + sp + c,a,f,é = 16
        assert_eq!(char_len("Hello 日本語 🎉 café"), 16);
    }

    #[test]
    fn test_char_len_empty() {
        assert_eq!(char_len(""), 0);
    }

    #[test]
    fn test_char_truncate_cjk() {
        assert_eq!(char_truncate("日本語テスト", 3), "日本語");
    }

    #[test]
    fn test_char_truncate_empty() {
        assert_eq!(char_truncate("", 100), "");
    }

    #[test]
    fn test_char_truncate_exact() {
        let s = "abcde";
        assert_eq!(char_truncate(s, 5), "abcde");
    }

    #[test]
    fn test_char_truncate_ascii_noop() {
        let s = "hi";
        assert_eq!(char_truncate(s, 100), "hi");
    }

    #[test]
    fn test_char_truncate_zero_returns_empty() {
        assert_eq!(char_truncate("hello", 0), "");
        assert_eq!(char_truncate("日本語", 0), "");
        assert_eq!(char_truncate("", 0), "");
    }

    #[test]
    fn test_char_truncate_one() {
        assert_eq!(char_truncate("hello", 1), "h");
        assert_eq!(char_truncate("日本語", 1), "日");
    }
}
