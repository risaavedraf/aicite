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
    let cite_commands = check_docs::parser::extract_cite_commands(&code_blocks);

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

    Ok(check_docs::VerificationReport {
        file: path.clone(),
        binary_version: binary_version.to_string(),
        results,
        summary: check_docs::ReportSummary {
            ok,
            outdated,
            warning,
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

    Ok(check_docs::AggregateReport {
        files,
        summary: check_docs::ReportSummary {
            ok,
            outdated,
            warning,
        },
    })
}

fn verify_command(
    cmd: &check_docs::parser::CiteCommand,
    binary_path: &PathBuf,
    skip_dynamic: bool,
    section: &str,
) -> check_docs::CommandResult {
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
