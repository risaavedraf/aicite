pub mod comparator;
pub mod executor;
pub mod parser;
pub mod report;

use std::path::PathBuf;

/// Status of a verification check.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    /// Command succeeded and output matches.
    Ok,
    /// Command or output is outdated.
    Outdated,
    /// Dynamic value changed (not necessarily wrong).
    Warning,
    /// Command is planned; verification skipped.
    Planned,
}

impl std::fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckStatus::Ok => write!(f, "✅ OK"),
            CheckStatus::Outdated => write!(f, "❌ OUTDATED"),
            CheckStatus::Warning => write!(f, "⚠️  WARNING"),
            CheckStatus::Planned => write!(f, "⏭️ PLANNED"),
        }
    }
}

/// Result of verifying a single command.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommandResult {
    /// Section or context where the command was found.
    pub section: String,
    /// Line number in the source file.
    pub line: usize,
    /// Verification status.
    pub status: CheckStatus,
    /// Human-readable detail.
    pub detail: String,
}

/// Report for a single file.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VerificationReport {
    /// Path to the verified file.
    pub file: PathBuf,
    /// Version of the cite binary used.
    pub binary_version: String,
    /// Individual command results.
    pub results: Vec<CommandResult>,
    /// Summary counts.
    pub summary: ReportSummary,
}

/// Summary counts for a verification report.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReportSummary {
    pub ok: usize,
    pub outdated: usize,
    pub warning: usize,
    #[serde(default)]
    pub planned: usize,
}

/// Aggregate report for directory scans.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AggregateReport {
    /// Individual file reports.
    pub files: Vec<VerificationReport>,
    /// Total summary.
    pub summary: ReportSummary,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_status_planned_serializes_lowercase() {
        let planned = CheckStatus::Planned;
        let json = serde_json::to_string(&planned).unwrap();
        assert_eq!(json, "\"planned\"");
        let deserialized: CheckStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, CheckStatus::Planned);
    }
}
