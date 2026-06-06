use std::path::PathBuf;
use std::process::Command;

/// Result of executing a command.
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Standard output.
    pub stdout: String,
    /// Standard error.
    pub stderr: String,
    /// Exit code.
    pub exit_code: i32,
}

/// Execute a cite command and capture its output.
///
/// The command is executed with `--json` flag if not already present,
/// to get machine-readable output for comparison.
pub fn execute_command(command: &str, binary_path: &PathBuf) -> ExecutionResult {
    // Parse the command to extract arguments
    let args = parse_command_args(command);

    // Add --json if not present and the command supports it
    let mut final_args = args;
    if !final_args.iter().any(|a| a == "--json") {
        // Check if the command supports --json
        let supports_json = !final_args.is_empty()
            && matches!(
                final_args[0].as_str(),
                "health"
                    | "search"
                    | "retrieve"
                    | "context"
                    | "list"
                    | "get"
                    | "trace"
                    | "evaluate"
            );
        if supports_json {
            final_args.push("--json".to_string());
        }
    }

    // Execute with timeout
    let output = Command::new(binary_path).args(&final_args).output();

    match output {
        Ok(output) => ExecutionResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        },
        Err(e) => ExecutionResult {
            stdout: String::new(),
            stderr: format!("Failed to execute command: {e}"),
            exit_code: -1,
        },
    }
}

/// Parse a command string into arguments, handling quoted strings.
fn parse_command_args(command: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    let mut quote_char = ' ';

    // Skip the "cite " prefix if present
    let cmd = command.strip_prefix("cite ").unwrap_or(command);

    for ch in cmd.chars() {
        match ch {
            '"' | '\'' if !in_quote => {
                in_quote = true;
                quote_char = ch;
            }
            c if c == quote_char && in_quote => {
                in_quote = false;
            }
            ' ' if !in_quote => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        args.push(current);
    }

    args
}

/// Get the path to the cite binary.
///
/// This attempts to find the binary in the target directory or PATH.
pub fn find_cite_binary() -> PathBuf {
    // Try to find in target/debug first (development)
    let debug_path = PathBuf::from("target/debug/cite.exe");
    if debug_path.exists() {
        return debug_path;
    }

    let debug_path = PathBuf::from("target/debug/cite");
    if debug_path.exists() {
        return debug_path;
    }

    // Try target/release
    let release_path = PathBuf::from("target/release/cite.exe");
    if release_path.exists() {
        return release_path;
    }

    let release_path = PathBuf::from("target/release/cite");
    if release_path.exists() {
        return release_path;
    }

    // Fall back to "cite" in PATH
    PathBuf::from("cite")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_args() {
        let args = parse_command_args("cite search \"test query\"");
        assert_eq!(args, vec!["search", "test query"]);
    }

    #[test]
    fn parse_args_with_flags() {
        let args = parse_command_args("cite search \"test\" --json --full");
        assert_eq!(args, vec!["search", "test", "--json", "--full"]);
    }

    #[test]
    fn parse_args_without_cite_prefix() {
        let args = parse_command_args("search \"test\"");
        assert_eq!(args, vec!["search", "test"]);
    }

    #[test]
    fn parse_args_with_single_quotes() {
        let args = parse_command_args("cite search 'test query'");
        assert_eq!(args, vec!["search", "test query"]);
    }
}
