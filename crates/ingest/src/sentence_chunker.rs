/// A chunk produced by sentence-based splitting.
#[derive(Debug, Clone, PartialEq)]
pub struct SentenceChunk {
    pub text: String,
    pub offset_start: usize,
    pub offset_end: usize,
}

/// Split text into sentence-based chunks.
///
/// - Splits on sentence boundaries (`.`, `!`, `?` followed by whitespace or EOF)
/// - Each sentence becomes its own chunk
/// - If a chunk is shorter than min_chars, it merges with the next sentence
/// - Tracks char offsets via char_indices()
pub fn chunk_by_sentence(text: &str, min_chars: usize) -> Vec<SentenceChunk> {
    if text.is_empty() {
        return Vec::new();
    }

    let sentences = split_sentences(text);
    let mut chunks = Vec::new();
    let mut current_text = String::new();
    let mut current_offset_start: usize = 0;

    for sentence in &sentences {
        let sentence_text = sentence.text.trim();
        if sentence_text.is_empty() {
            continue;
        }

        if current_text.is_empty() {
            // Starting a new chunk
            current_offset_start = sentence.offset_start;
            current_text = sentence_text.to_string();
        } else if current_text.len() < min_chars {
            // Current chunk too short — must merge with next sentence
            current_text.push(' ');
            current_text.push_str(sentence_text);
        } else {
            // Current chunk is big enough to stand alone — flush it
            let offset_end = current_offset_start + current_text.len();
            chunks.push(SentenceChunk {
                text: current_text.clone(),
                offset_start: current_offset_start,
                offset_end,
            });
            current_offset_start = sentence.offset_start;
            current_text = sentence_text.to_string();
        }
    }

    // Flush remaining
    if !current_text.is_empty() {
        let offset_end = current_offset_start + current_text.len();
        chunks.push(SentenceChunk {
            text: current_text,
            offset_start: current_offset_start,
            offset_end,
        });
    }

    chunks
}

struct SentenceInfo {
    text: String,
    offset_start: usize,
}

fn split_sentences(text: &str) -> Vec<SentenceInfo> {
    let mut sentences = Vec::new();
    let mut current = String::new();
    let mut current_offset: usize = 0;
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];
        current.push(ch);

        // Check for sentence boundary
        if is_sentence_end(ch) {
            let next_is_whitespace = i + 1 >= chars.len()
                || chars[i + 1] == ' '
                || chars[i + 1] == '\t'
                || chars[i + 1] == '\n'
                || chars[i + 1] == '\r';

            if next_is_whitespace && !is_abbreviation(&current) {
                sentences.push(SentenceInfo {
                    text: current.trim().to_string(),
                    offset_start: current_offset,
                });
                current = String::new();
                current_offset = i + 1;
            }
        }

        i += 1;
    }

    // Flush remaining
    if !current.trim().is_empty() {
        sentences.push(SentenceInfo {
            text: current.trim().to_string(),
            offset_start: current_offset,
        });
    }

    sentences
}

fn is_sentence_end(ch: char) -> bool {
    ch == '.' || ch == '!' || ch == '?'
}

fn is_abbreviation(text: &str) -> bool {
    let abbreviations = [
        "Dr.", "Mr.", "Mrs.", "Ms.", "Prof.", "Sr.", "Sra.", "St.", "e.g.", "i.e.", "etc.", "vs.",
        "approx.",
    ];
    let trimmed = text.trim();
    abbreviations.iter().any(|abbr| trimmed.ends_with(abbr))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_sentences() {
        let text = "Hello world. This is a test! How are you?";
        let chunks = chunk_by_sentence(text, 5);
        // Each sentence >= min_chars (5), so each is its own chunk
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].text, "Hello world.");
        assert_eq!(chunks[1].text, "This is a test!");
        assert_eq!(chunks[2].text, "How are you?");
    }

    #[test]
    fn test_short_sentences_merged() {
        let text = "Hi. OK. This is a longer sentence that should stand alone.";
        let chunks = chunk_by_sentence(text, 20);
        // "Hi." < 20 → merge with "OK." → "Hi. OK." (7) < 20 → merge with next
        // Result: 1 chunk with all three merged
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].text.contains("Hi."));
        assert!(chunks[0].text.contains("OK."));
        assert!(chunks[0].text.contains("longer sentence"));
    }

    #[test]
    fn test_short_then_long() {
        let text = "Hi. This is a longer sentence that should stand alone. Another long one here.";
        let chunks = chunk_by_sentence(text, 20);
        // "Hi." < 20 → merge with next → "Hi. This is a longer sentence..." (52) >= 20 → flush
        // "Another long one here." (23) >= 20 → flush
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].text.starts_with("Hi."));
        assert!(chunks[0].text.contains("longer sentence"));
        assert!(chunks[1].text.contains("Another long one"));
    }

    #[test]
    fn test_utf8_text() {
        let text = "Hola mundo. Café con leche.";
        let chunks = chunk_by_sentence(text, 5);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].text, "Hola mundo.");
        assert_eq!(chunks[1].text, "Café con leche.");
    }

    #[test]
    fn test_empty_text() {
        let chunks = chunk_by_sentence("", 30);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_abbreviations_not_split() {
        let text = "Dr. Smith went to Washington. He was happy.";
        let chunks = chunk_by_sentence(text, 10);
        // "Dr. Smith went to Washington." (29) >= 10 → flush
        // "He was happy." (13) >= 10 → flush
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].text.contains("Dr. Smith"));
        assert!(chunks[1].text.contains("He was happy"));
    }

    #[test]
    fn test_single_sentence() {
        let text = "Just one sentence.";
        let chunks = chunk_by_sentence(text, 5);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "Just one sentence.");
    }

    #[test]
    fn test_offset_tracking() {
        let text = "First sentence. Second sentence. Third.";
        let chunks = chunk_by_sentence(text, 5);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].offset_start, 0);
        assert_eq!(chunks[1].offset_start, 15); // "First sentence." = 15 chars, space at 15
        assert_eq!(chunks[2].offset_start, 32); // " Second sentence." = 17 chars → 15+17=32
    }

    #[test]
    fn test_min_chars_boundary() {
        // Exactly min_chars should NOT merge
        let text = "ABCDE. FGHIJ. KLMNO."; // 5 chars each, min=5
        let chunks = chunk_by_sentence(text, 5);
        assert_eq!(chunks.len(), 3);
    }

    #[test]
    fn test_multiline_text() {
        let text = "First line here. Second line there.\nThird line somewhere.";
        let chunks = chunk_by_sentence(text, 10);
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert!(!chunk.text.is_empty());
        }
    }
}
