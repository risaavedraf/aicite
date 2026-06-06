use crate::{AggregateReport, CheckStatus, VerificationReport};

/// Format a verification report for human reading.
pub fn format_human_report(report: &VerificationReport) -> String {
    let mut output = String::new();

    output.push_str(&format!(
        "Checking {} against cite {}...\n\n",
        report.file.display(),
        report.binary_version
    ));

    for result in &report.results {
        let icon = match &result.status {
            CheckStatus::Ok => "✅",
            CheckStatus::Outdated => "❌",
            CheckStatus::Warning => "⚠️",
        };
        output.push_str(&format!("{} {}: {}\n", icon, result.section, result.detail));
        output.push_str(&format!("   Line: {}\n\n", result.line));
    }

    output.push_str(&format!(
        "Results: {} ok, {} outdated, {} warnings\n",
        report.summary.ok, report.summary.outdated, report.summary.warning
    ));

    output
}

/// Format a verification report as JSON.
pub fn format_json_report(report: &VerificationReport) -> String {
    serde_json::to_string_pretty(report).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}

/// Format an aggregate report for human reading (directory scan).
pub fn format_aggregate_human(report: &AggregateReport) -> String {
    let mut output = String::new();

    for file_report in &report.files {
        output.push_str(&format_human_report(file_report));
        output.push_str("---\n\n");
    }

    output.push_str(&format!(
        "Aggregate: {} files, {} ok, {} outdated, {} warnings\n",
        report.files.len(),
        report.summary.ok,
        report.summary.outdated,
        report.summary.warning
    ));

    output
}

/// Format an aggregate report as JSON.
pub fn format_aggregate_json(report: &AggregateReport) -> String {
    serde_json::to_string_pretty(report).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}
