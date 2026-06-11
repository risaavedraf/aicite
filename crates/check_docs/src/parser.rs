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
    /// Status tag from adjacent HTML comment (e.g., "planned", "implemented").
    pub status_tag: Option<String>,
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
///
/// `content` is the raw markdown source, used to scan for adjacent
/// `<!-- tag:status=VALUE -->` HTML comments before each code block.
pub fn extract_cite_commands(blocks: &[CodeBlock], content: &str) -> Vec<CiteCommand> {
    let mut commands = Vec::new();
    let tag_re = Regex::new(r"<!--\s*tag:status=(\w+)\s*-->").unwrap();
    let lines: Vec<&str> = content.lines().collect();

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

        let status_tag = find_adjacent_status_tag(block.start_line, &lines, &tag_re);

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
                    status_tag: status_tag.clone(),
                });
            }
        }
    }

    commands
}

fn find_adjacent_status_tag(start_line: usize, lines: &[&str], tag_re: &Regex) -> Option<String> {
    let fence_idx = start_line.saturating_sub(1);
    let mut status_tag = None;
    let mut scan = fence_idx.checked_sub(1)?;

    loop {
        let candidate = lines.get(scan)?.trim();
        if candidate.is_empty() {
            break;
        }

        if let Some(caps) = tag_re.captures(candidate) {
            status_tag = Some(caps[1].to_string());
        } else if !candidate.starts_with("<!--") || !candidate.ends_with("-->") {
            break;
        }

        if scan == 0 {
            break;
        }
        scan -= 1;
    }

    status_tag
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

    // ── PR 6 RED tests: status_tag adjacency detection ──

    #[test]
    fn test_extract_cite_commands_with_planned_tag_before_block() {
        let md = "Some text\n\n<!-- tag:status=planned -->\n```bash\ncite search \"test\"\n```";
        let blocks = parse_code_blocks(md);
        let commands = extract_cite_commands(&blocks, md);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].status_tag, Some("planned".to_string()));
    }

    #[test]
    fn test_extract_cite_commands_with_implemented_tag() {
        let md = "<!-- tag:status=implemented -->\n```bash\ncite health --json\n```";
        let blocks = parse_code_blocks(md);
        let commands = extract_cite_commands(&blocks, md);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].status_tag, Some("implemented".to_string()));
    }

    #[test]
    fn test_extract_cite_commands_with_unknown_tag() {
        let md = "<!-- tag:status=experimental -->\n```bash\ncite search \"foo\"\n```";
        let blocks = parse_code_blocks(md);
        let commands = extract_cite_commands(&blocks, md);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].status_tag, Some("experimental".to_string()));
    }

    #[test]
    fn test_extract_cite_commands_without_tag() {
        let md = "```bash\ncite search \"test\"\n```";
        let blocks = parse_code_blocks(md);
        let commands = extract_cite_commands(&blocks, md);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].status_tag, None);
    }

    #[test]
    fn test_extract_cite_commands_tag_adjacency_blank_line() {
        // Tag is separated by a blank line from the block — should NOT be adjacent
        let md = "<!-- tag:status=planned -->\n\n\n```bash\ncite search \"test\"\n```";
        let blocks = parse_code_blocks(md);
        let commands = extract_cite_commands(&blocks, md);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].status_tag, None);
    }

    #[test]
    fn test_extract_cite_commands_tag_after_block_ignored() {
        // Tag AFTER the closing fence — should not count
        let md = "```bash\ncite search \"test\"\n```\n<!-- tag:status=planned -->";
        let blocks = parse_code_blocks(md);
        let commands = extract_cite_commands(&blocks, md);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].status_tag, None);
    }

    #[test]
    fn test_extract_cite_commands_multiple_tags_first_wins() {
        // Two tags before one block — first one should be used
        let md = "<!-- tag:status=planned -->\n<!-- tag:status=implemented -->\n```bash\ncite search \"test\"\n```";
        let blocks = parse_code_blocks(md);
        let commands = extract_cite_commands(&blocks, md);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].status_tag, Some("planned".to_string()));
    }

    #[test]
    fn test_extract_cite_commands_status_tag_in_html_comment_block() {
        let md = "<!-- tag:status=planned -->\n<!-- docs-only example -->\n```bash\ncite search \"test\"\n```";
        let blocks = parse_code_blocks(md);
        let commands = extract_cite_commands(&blocks, md);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].status_tag, Some("planned".to_string()));
    }

    // ── existing tests below ──

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

        let commands = extract_cite_commands(&blocks, "");
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

        let commands = extract_cite_commands(&blocks, "");
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
