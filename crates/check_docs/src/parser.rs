use regex::Regex;

/// A code block extracted from markdown.
#[derive(Debug, Clone)]
pub struct CodeBlock {
    /// Language tag (e.g., "bash", "json").
    pub language: Option<String>,
    /// Content of the code block.
    pub content: String,
    /// Line number where the block starts.
    pub start_line: usize,
}

/// A cite command extracted from a code block, with optional expected output.
#[derive(Debug, Clone)]
pub struct CiteCommand {
    /// The command string (e.g., "cite search \"test\" --json").
    pub command: String,
    /// Section context (e.g., heading text or surrounding paragraph).
    pub section: String,
    /// Line number in the source file.
    pub line: usize,
    /// Expected output from the next code block (if it's JSON/output).
    pub expected_output: Option<String>,
}

/// Extract all code blocks from markdown content.
pub fn parse_code_blocks(content: &str) -> Vec<CodeBlock> {
    let re = Regex::new(r"(?m)^```(\w*)\s*$").unwrap();
    let mut blocks = Vec::new();
    let mut in_block = false;
    let mut current_lang = None;
    let mut current_content = String::new();
    let mut start_line = 0;

    for (line_idx, line) in content.lines().enumerate() {
        let line_num = line_idx + 1; // 1-indexed

        if let Some(caps) = re.captures(line) {
            if in_block {
                // End of block
                blocks.push(CodeBlock {
                    language: current_lang.take(),
                    content: current_content.trim().to_string(),
                    start_line,
                });
                current_content.clear();
                in_block = false;
            } else {
                // Start of block
                in_block = true;
                start_line = line_num;
                let lang = caps.get(1).map(|m| m.as_str().to_string());
                current_lang = if lang.as_deref() == Some("") {
                    None
                } else {
                    lang
                };
            }
        } else if in_block {
            if !current_content.is_empty() {
                current_content.push('\n');
            }
            current_content.push_str(line);
        }
    }

    blocks
}

/// Extract cite commands from code blocks.
///
/// Only extracts commands that start with "cite ". Ignores other commands.
/// If the next code block is JSON, it's attached as expected output.
pub fn extract_cite_commands(blocks: &[CodeBlock]) -> Vec<CiteCommand> {
    let mut commands = Vec::new();

    for (i, block) in blocks.iter().enumerate() {
        // Only process bash/shell blocks
        let is_bash = block
            .language
            .as_deref()
            .map(|l| l == "bash" || l == "sh" || l == "shell")
            .unwrap_or(false);

        if !is_bash {
            continue;
        }

        // Extract cite commands from the block
        for (line_offset, line) in block.content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("cite ") {
                // Check if next block is JSON (expected output)
                let expected_output = blocks.get(i + 1).and_then(|next| {
                    if next
                        .language
                        .as_deref()
                        .map(|l| l == "json" || l == "output")
                        .unwrap_or(false)
                    {
                        Some(next.content.clone())
                    } else {
                        None
                    }
                });

                commands.push(CiteCommand {
                    command: trimmed.to_string(),
                    section: String::new(), // Will be populated by caller
                    line: block.start_line + line_offset,
                    expected_output,
                });
            }
        }
    }

    commands
}

/// Extract section headings from markdown for context.
pub fn extract_headings(content: &str) -> Vec<(usize, String)> {
    let re = Regex::new(r"(?m)^#{1,6}\s+(.+)$").unwrap();
    re.captures_iter(content)
        .map(|caps| {
            let line = content[..caps.get(0).unwrap().start()].lines().count() + 1;
            (line, caps[1].trim().to_string())
        })
        .collect()
}

/// Find the nearest heading for a given line number.
pub fn nearest_heading(headings: &[(usize, String)], line: usize) -> String {
    headings
        .iter()
        .rev()
        .find(|(h_line, _)| *h_line < line)
        .map(|(_, title)| title.clone())
        .unwrap_or_else(|| "(no section)".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_code_block() {
        let md = r#"Some text

```bash
cite search "test"
```

More text"#;
        let blocks = parse_code_blocks(md);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].language, Some("bash".to_string()));
        assert!(blocks[0].content.contains("cite search"));
    }

    #[test]
    fn parse_code_block_with_json() {
        let md = r#"```bash
cite health --json
```

```json
{"status": "ok"}
```"#;
        let blocks = parse_code_blocks(md);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].language, Some("bash".to_string()));
        assert_eq!(blocks[1].language, Some("json".to_string()));
    }

    #[test]
    fn extract_cite_commands_ignores_non_cite() {
        let blocks = vec![
            CodeBlock {
                language: Some("bash".to_string()),
                content: "cargo build --release\ncite search \"test\"".to_string(),
                start_line: 5,
            },
            CodeBlock {
                language: Some("bash".to_string()),
                content: "npm install".to_string(),
                start_line: 10,
            },
        ];

        let commands = extract_cite_commands(&blocks);
        assert_eq!(commands.len(), 1);
        assert!(commands[0].command.contains("cite search"));
    }

    #[test]
    fn extract_cite_commands_with_expected_output() {
        let blocks = vec![
            CodeBlock {
                language: Some("bash".to_string()),
                content: "cite health --json".to_string(),
                start_line: 1,
            },
            CodeBlock {
                language: Some("json".to_string()),
                content: "{\"status\": \"ok\"}".to_string(),
                start_line: 3,
            },
        ];

        let commands = extract_cite_commands(&blocks);
        assert_eq!(commands.len(), 1);
        assert!(commands[0].expected_output.is_some());
        assert!(commands[0].expected_output.as_ref().unwrap().contains("ok"));
    }

    #[test]
    fn test_extract_headings() {
        let md = r#"# Title

## Section 1

Some text

### Subsection

More text"#;
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 3);
        assert_eq!(headings[0].1, "Title");
        assert_eq!(headings[1].1, "Section 1");
        assert_eq!(headings[2].1, "Subsection");
    }

    #[test]
    fn nearest_heading_finds_correct() {
        let headings = vec![
            (1, "Title".to_string()),
            (5, "Section 1".to_string()),
            (10, "Section 2".to_string()),
        ];

        assert_eq!(nearest_heading(&headings, 3), "Title");
        assert_eq!(nearest_heading(&headings, 7), "Section 1");
        assert_eq!(nearest_heading(&headings, 15), "Section 2");
    }

    #[test]
    fn nearest_heading_no_section() {
        let headings = vec![];
        assert_eq!(nearest_heading(&headings, 5), "(no section)");
    }
}
