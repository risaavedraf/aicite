use crate::types::HeadingSpan;

/// Extract markdown headings (##, ###, etc.) with their positions.
/// Returns headings ordered by appearance.
/// Ignores headings inside code blocks (``` ... ```).
pub fn extract_headings(markdown: &str) -> Vec<HeadingSpan> {
    let mut headings = Vec::new();
    let mut in_code_block = false;
    let mut char_offset = 0;

    for line in markdown.lines() {
        let trimmed = line.trim();

        // Toggle code blocks
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
        } else if !in_code_block && trimmed.starts_with('#') {
            // Count heading level
            let level = trimmed.chars().take_while(|&c| c == '#').count();
            let title = trimmed[level..].trim().to_string();

            if !title.is_empty() && (1..=6).contains(&level) {
                headings.push(HeadingSpan {
                    level,
                    title,
                    char_offset,
                });
            }
        }

        char_offset += line.chars().count() + 1; // +1 for newline
    }

    headings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_headings_basic() {
        let md = r#"# Title

## Section 1

Some content here.

### Subsection 1.1

More content.

## Section 2

Final content."#;

        let headings = extract_headings(md);
        assert_eq!(headings.len(), 4);
        assert_eq!(headings[0].level, 1);
        assert_eq!(headings[0].title, "Title");
        assert_eq!(headings[1].level, 2);
        assert_eq!(headings[1].title, "Section 1");
        assert_eq!(headings[2].level, 3);
        assert_eq!(headings[2].title, "Subsection 1.1");
        assert_eq!(headings[3].level, 2);
        assert_eq!(headings[3].title, "Section 2");
    }

    #[test]
    fn test_no_headings() {
        let md = "Just plain text\nwith no headings.";
        let headings = extract_headings(md);
        assert!(headings.is_empty());
    }

    #[test]
    fn test_headings_in_code_blocks_ignored() {
        let md = r#"## Real Heading

```
## Not a Heading
```

## Another Real"#;

        let headings = extract_headings(md);
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].title, "Real Heading");
        assert_eq!(headings[1].title, "Another Real");
    }

    #[test]
    fn test_empty_heading_ignored() {
        let md = "## \n## Valid\n## ";
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 1);
        assert_eq!(headings[0].title, "Valid");
    }

    #[test]
    fn test_char_offsets() {
        let md = "## First\n\nSome text\n\n## Second";
        let headings = extract_headings(md);
        assert_eq!(headings[0].char_offset, 0);
        // "## First" = 8 chars, "\n" = 1, "" = 0, "\n" = 1, "Some text" = 9, "\n" = 1, "" = 0, "\n" = 1
        // Total offset = 8 + 1 + 0 + 1 + 9 + 1 + 0 + 1 = 21
        assert_eq!(headings[1].char_offset, 21);
    }

    #[test]
    fn test_extract_headings_utf8_offsets() {
        let md = "## 🎉\n\nSome text\n\n## 日本語";
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].title, "🎉");
        assert_eq!(headings[0].char_offset, 0);
        // "## 🎉" = 4 chars, then "\n\nSome text\n\n" = 13 chars, total = 17
        assert_eq!(headings[1].title, "日本語");
        assert_eq!(headings[1].char_offset, 17);
    }

    #[test]
    fn test_extract_headings_accented_offsets() {
        let md = "## Café\n\ncontent\n\n## Résumé";
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].title, "Café");
        assert_eq!(headings[0].char_offset, 0);
        // "## Café" = 7 chars, then "\n\ncontent\n\n" = 11 chars, total = 18
        assert_eq!(headings[1].title, "Résumé");
        assert_eq!(headings[1].char_offset, 18);
    }

    #[test]
    fn test_indented_code_fence_toggled() {
        // Indented code fences should still be detected and toggle the state
        let md = "## Real\n\n    ```rust\n## Inside Fenced Block\n    ```\n\n## After Block";
        let headings = extract_headings(md);
        // "## Inside Fenced Block" is inside a code block so should be ignored
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].title, "Real");
        assert_eq!(headings[1].title, "After Block");
    }

    #[test]
    fn test_deep_headings_h4_h5_h6() {
        let md = "## Topic\n\n#### Deep\n\n##### Deeper\n\n###### Deepest";
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 4);
        assert_eq!(headings[0].level, 2);
        assert_eq!(headings[0].title, "Topic");
        assert_eq!(headings[1].level, 4);
        assert_eq!(headings[1].title, "Deep");
        assert_eq!(headings[2].level, 5);
        assert_eq!(headings[2].title, "Deeper");
        assert_eq!(headings[3].level, 6);
        assert_eq!(headings[3].title, "Deepest");
    }
}
