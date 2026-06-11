use clap::Args;
use common::ExitCode;
use config::Config;
use std::path::PathBuf;

use super::exit_for_error;
use crate::output::print_json;

#[derive(Args)]
pub struct CheckDocsArgs {
    /// File or directory to verify
    pub path: String,

    /// Scan directory recursively
    #[arg(long)]
    pub recursive: bool,

    /// Machine-readable JSON output
    #[arg(long)]
    pub json: bool,

    /// Skip verification of dynamic values (latency, UUIDs, timestamps)
    #[arg(long)]
    pub skip_dynamic: bool,
}

pub fn execute(args: &CheckDocsArgs, _config: &Config, json: bool) -> i32 {
    let path = PathBuf::from(&args.path);

    if !path.exists() {
        let err = common::CiteError::InvalidParameter {
            message: format!("Path not found: {}", path.display()),
        };
        return exit_for_error(&err, json);
    }

    let binary_version = env!("CARGO_PKG_VERSION").to_string();
    let binary_path = check_docs::executor::find_cite_binary();

    if path.is_file() {
        match verify_file(&path, &binary_version, &binary_path, args.skip_dynamic) {
            Ok(report) => {
                if json || args.json {
                    print_json(&report);
                } else {
                    println!("{}", check_docs::report::format_human_report(&report));
                }
                if report.summary.outdated > 0 {
                    ExitCode::Validation as i32
                } else {
                    ExitCode::Success as i32
                }
            }
            Err(e) => exit_for_error(&e, json),
        }
    } else if path.is_dir() {
        match verify_directory(
            &path,
            &binary_version,
            &binary_path,
            args.recursive,
            args.skip_dynamic,
        ) {
            Ok(report) => {
                if json || args.json {
                    print_json(&report);
                } else {
                    println!("{}", check_docs::report::format_aggregate_human(&report));
                }
                if report.summary.outdated > 0 {
                    ExitCode::Validation as i32
                } else {
                    ExitCode::Success as i32
                }
            }
            Err(e) => exit_for_error(&e, json),
        }
    } else {
        let err = common::CiteError::InvalidParameter {
            message: format!("Path is not a file or directory: {}", path.display()),
        };
        exit_for_error(&err, json)
    }
}

fn verify_file(
    path: &PathBuf,
    binary_version: &str,
    binary_path: &PathBuf,
    skip_dynamic: bool,
) -> Result<check_docs::VerificationReport, common::CiteError> {
    let content = std::fs::read_to_string(path).map_err(|e| common::CiteError::StorageError {
        message: format!("Failed to read file: {e}"),
    })?;

    let code_blocks = check_docs::parser::parse_code_blocks(&content);
    let headings = check_docs::parser::extract_headings(&content);
    let cite_commands = check_docs::parser::extract_cite_commands(&code_blocks, &content);

    let mut results = Vec::new();

    for cmd in cite_commands {
        let section = check_docs::parser::nearest_heading(&headings, cmd.line);
        let result = verify_command(&cmd, binary_path, skip_dynamic, &section);
        results.push(result);
    }

    let ok = results
        .iter()
        .filter(|r| r.status == check_docs::CheckStatus::Ok)
        .count();
    let outdated = results
        .iter()
        .filter(|r| r.status == check_docs::CheckStatus::Outdated)
        .count();
    let warning = results
        .iter()
        .filter(|r| r.status == check_docs::CheckStatus::Warning)
        .count();
    let planned = results
        .iter()
        .filter(|r| r.status == check_docs::CheckStatus::Planned)
        .count();

    Ok(check_docs::VerificationReport {
        file: path.clone(),
        binary_version: binary_version.to_string(),
        results,
        summary: check_docs::ReportSummary {
            ok,
            outdated,
            warning,
            planned,
        },
    })
}

fn verify_directory(
    dir: &PathBuf,
    binary_version: &str,
    binary_path: &PathBuf,
    recursive: bool,
    skip_dynamic: bool,
) -> Result<check_docs::AggregateReport, common::CiteError> {
    let mut files = Vec::new();

    if recursive {
        for entry in walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        {
            let report = verify_file(
                &entry.path().to_path_buf(),
                binary_version,
                binary_path,
                skip_dynamic,
            )?;
            files.push(report);
        }
    } else {
        for entry in std::fs::read_dir(dir).map_err(|e| common::CiteError::StorageError {
            message: format!("Failed to read directory: {e}"),
        })? {
            let entry = entry.map_err(|e| common::CiteError::StorageError {
                message: format!("Failed to read entry: {e}"),
            })?;

            if entry.path().extension().is_some_and(|ext| ext == "md") {
                let report = verify_file(&entry.path(), binary_version, binary_path, skip_dynamic)?;
                files.push(report);
            }
        }
    }

    let ok = files.iter().map(|f| f.summary.ok).sum();
    let outdated = files.iter().map(|f| f.summary.outdated).sum();
    let warning = files.iter().map(|f| f.summary.warning).sum();
    let planned = files.iter().map(|f| f.summary.planned).sum();

    Ok(check_docs::AggregateReport {
        files,
        summary: check_docs::ReportSummary {
            ok,
            outdated,
            warning,
            planned,
        },
    })
}

fn verify_command(
    cmd: &check_docs::parser::CiteCommand,
    binary_path: &PathBuf,
    skip_dynamic: bool,
    section: &str,
) -> check_docs::CommandResult {
    // PR 6: skip planned commands without execution
    if cmd.status_tag.as_deref() == Some("planned") {
        return check_docs::CommandResult {
            section: section.to_string(),
            line: cmd.line,
            status: check_docs::CheckStatus::Planned,
            detail: "Planned command; verification skipped".to_string(),
        };
    }

    let output = check_docs::executor::execute_command(&cmd.command, binary_path);

    if let Some(expected) = &cmd.expected_output {
        let comparison = if skip_dynamic {
            check_docs::comparator::ComparisonResult {
                status: check_docs::CheckStatus::Ok,
                detail: "Skipped (dynamic)".to_string(),
            }
        } else {
            check_docs::comparator::compare_outputs(&output.stdout, expected)
        };

        check_docs::CommandResult {
            section: section.to_string(),
            line: cmd.line,
            status: comparison.status,
            detail: comparison.detail,
        }
    } else if output.exit_code == 0 {
        check_docs::CommandResult {
            section: section.to_string(),
            line: cmd.line,
            status: check_docs::CheckStatus::Ok,
            detail: "Command succeeded".to_string(),
        }
    } else {
        check_docs::CommandResult {
            section: section.to_string(),
            line: cmd.line,
            status: check_docs::CheckStatus::Outdated,
            detail: format!("Command failed: {}", output.stderr),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cmd(command: &str, status_tag: Option<&str>) -> check_docs::parser::CiteCommand {
        check_docs::parser::CiteCommand {
            command: command.to_string(),
            section: "Test".to_string(),
            line: 1,
            expected_output: None,
            status_tag: status_tag.map(|s| s.to_string()),
        }
    }

    #[test]
    fn test_verify_planned_command_skips_execution() {
        let cmd = make_cmd("cite nonexistent-command-for-test", Some("planned"));
        let binary_path = PathBuf::from("cite");
        let result = verify_command(&cmd, &binary_path, false, "Test");
        // RED: should be Planned once status_tag is checked before execution
        assert_eq!(result.status, check_docs::CheckStatus::Planned);
        assert!(
            result.detail.to_lowercase().contains("planned"),
            "Expected detail to mention 'planned', got: {}",
            result.detail
        );
    }

    #[test]
    fn test_verify_implemented_command_uses_normal_path() {
        let cmd = make_cmd("cite nonexistent-command-for-test", Some("implemented"));
        let binary_path = PathBuf::from("cite");
        let result = verify_command(&cmd, &binary_path, false, "Test");
        // Should NOT be Planned — implemented uses normal verification path
        assert_ne!(result.status, check_docs::CheckStatus::Planned);
    }

    #[test]
    fn test_verify_untagged_command_uses_normal_path() {
        let cmd = make_cmd("cite nonexistent-command-for-test", None);
        let binary_path = PathBuf::from("cite");
        let result = verify_command(&cmd, &binary_path, false, "Test");
        // Should NOT be Planned — untagged uses normal verification path
        assert_ne!(result.status, check_docs::CheckStatus::Planned);
    }
}
