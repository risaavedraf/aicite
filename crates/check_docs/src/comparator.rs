use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::CheckStatus;

/// Result of comparing actual vs expected output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    pub status: CheckStatus,
    pub detail: String,
}

/// Compare actual command output against expected output.
///
/// Returns Ok if outputs match, Outdated if they don't, Warning for dynamic values.
pub fn compare_outputs(actual: &str, expected: &str) -> ComparisonResult {
    // Normalize whitespace for comparison
    let actual_norm = normalize(actual);
    let expected_norm = normalize(expected);

    // Exact match
    if actual_norm == expected_norm {
        return ComparisonResult {
            status: CheckStatus::Ok,
            detail: "Output matches expected".to_string(),
        };
    }

    // Check for dynamic values that change between runs
    if is_dynamic_difference(&actual_norm, &expected_norm) {
        return ComparisonResult {
            status: CheckStatus::Warning,
            detail: "Dynamic values differ (latency, UUIDs, timestamps)".to_string(),
        };
    }

    // Try semantic JSON comparison if both look like JSON
    if let (Ok(actual_json), Ok(expected_json)) = (
        serde_json::from_str::<serde_json::Value>(&actual_norm),
        serde_json::from_str::<serde_json::Value>(&expected_norm),
    ) {
        return compare_json_values(&actual_json, &expected_json);
    }

    ComparisonResult {
        status: CheckStatus::Outdated,
        detail: format!(
            "Output differs. Expected: {}..., Got: {}...",
            truncate(&expected_norm, 100),
            truncate(&actual_norm, 100)
        ),
    }
}

/// Normalize whitespace for comparison.
fn normalize(s: &str) -> String {
    s.trim()
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Truncate a string to max length with ellipsis.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

/// Check if differences are only in known dynamic values.
fn is_dynamic_difference(actual: &str, expected: &str) -> bool {
    // Patterns for dynamic values
    let patterns = vec![
        // Latency values (numbers followed by ms)
        (r"\d+\s*ms", "NUM ms"),
        // UUIDs
        (
            r"[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}",
            "UUID",
        ),
        // ISO timestamps
        (r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}", "TIMESTAMP"),
        // Unix timestamps (10+ digits)
        (r"\b\d{10,}\b", "UNIX_TS"),
    ];

    let mut actual_replaced = actual.to_string();
    let mut expected_replaced = expected.to_string();

    for (pattern, replacement) in &patterns {
        if let Ok(re) = Regex::new(pattern) {
            actual_replaced = re.replace_all(&actual_replaced, *replacement).to_string();
            expected_replaced = re.replace_all(&expected_replaced, *replacement).to_string();
        }
    }

    // If they match after replacing dynamic values, it's just a dynamic difference
    actual_replaced == expected_replaced
}

/// Compare JSON values recursively, allowing for dynamic value differences.
fn compare_json_values(
    actual: &serde_json::Value,
    expected: &serde_json::Value,
) -> ComparisonResult {
    match (actual, expected) {
        // Both objects: compare keys
        (serde_json::Value::Object(actual_map), serde_json::Value::Object(expected_map)) => {
            let mut missing_keys = Vec::new();
            let mut extra_keys = Vec::new();
            let mut value_diffs = Vec::new();

            // Check for missing keys
            for key in expected_map.keys() {
                if !actual_map.contains_key(key) {
                    missing_keys.push(key.clone());
                }
            }

            // Check for extra keys
            for key in actual_map.keys() {
                if !expected_map.contains_key(key) {
                    extra_keys.push(key.clone());
                }
            }

            // Compare common keys
            for (key, expected_val) in expected_map {
                if let Some(actual_val) = actual_map.get(key) {
                    let result = compare_json_values(actual_val, expected_val);
                    if result.status != CheckStatus::Ok {
                        value_diffs.push(format!("{}: {}", key, result.detail));
                    }
                }
            }

            if missing_keys.is_empty() && extra_keys.is_empty() && value_diffs.is_empty() {
                ComparisonResult {
                    status: CheckStatus::Ok,
                    detail: "JSON objects match".to_string(),
                }
            } else {
                let mut details = Vec::new();
                if !missing_keys.is_empty() {
                    details.push(format!("Missing keys: {:?}", missing_keys));
                }
                if !extra_keys.is_empty() {
                    details.push(format!("Extra keys: {:?}", extra_keys));
                }
                if !value_diffs.is_empty() {
                    details.push(format!("Value diffs: {}", value_diffs.join("; ")));
                }

                // If only value diffs and they look dynamic, warn instead of fail
                if missing_keys.is_empty() && extra_keys.is_empty() {
                    let all_dynamic = value_diffs.iter().all(|d| {
                        d.contains("Dynamic")
                            || d.contains("latency")
                            || d.contains("UUID")
                            || d.contains("timestamp")
                    });
                    if all_dynamic {
                        return ComparisonResult {
                            status: CheckStatus::Warning,
                            detail: format!("Dynamic values differ: {}", details.join("; ")),
                        };
                    }
                }

                ComparisonResult {
                    status: CheckStatus::Outdated,
                    detail: details.join("; "),
                }
            }
        }
        // Both arrays: compare element-wise
        (serde_json::Value::Array(actual_arr), serde_json::Value::Array(expected_arr)) => {
            if actual_arr.len() != expected_arr.len() {
                return ComparisonResult {
                    status: CheckStatus::Outdated,
                    detail: format!(
                        "Array length differs: expected {}, got {}",
                        expected_arr.len(),
                        actual_arr.len()
                    ),
                };
            }

            for (i, (a, e)) in actual_arr.iter().zip(expected_arr.iter()).enumerate() {
                let result = compare_json_values(a, e);
                if result.status != CheckStatus::Ok {
                    return ComparisonResult {
                        status: result.status,
                        detail: format!("Index {}: {}", i, result.detail),
                    };
                }
            }

            ComparisonResult {
                status: CheckStatus::Ok,
                detail: "Arrays match".to_string(),
            }
        }
        // Both strings: check for dynamic differences
        (serde_json::Value::String(actual_str), serde_json::Value::String(expected_str)) => {
            if actual_str == expected_str {
                ComparisonResult {
                    status: CheckStatus::Ok,
                    detail: "Strings match".to_string(),
                }
            } else if is_dynamic_difference(actual_str, expected_str) {
                ComparisonResult {
                    status: CheckStatus::Warning,
                    detail: "Dynamic string values differ".to_string(),
                }
            } else {
                ComparisonResult {
                    status: CheckStatus::Outdated,
                    detail: format!(
                        "String differs: expected '{}', got '{}'",
                        truncate(expected_str, 50),
                        truncate(actual_str, 50)
                    ),
                }
            }
        }
        // Both numbers
        (serde_json::Value::Number(actual_num), serde_json::Value::Number(expected_num)) => {
            if actual_num == expected_num {
                ComparisonResult {
                    status: CheckStatus::Ok,
                    detail: "Numbers match".to_string(),
                }
            } else {
                ComparisonResult {
                    status: CheckStatus::Outdated,
                    detail: format!(
                        "Number differs: expected {}, got {}",
                        expected_num, actual_num
                    ),
                }
            }
        }
        // Both booleans
        (serde_json::Value::Bool(actual_b), serde_json::Value::Bool(expected_b)) => {
            if actual_b == expected_b {
                ComparisonResult {
                    status: CheckStatus::Ok,
                    detail: "Booleans match".to_string(),
                }
            } else {
                ComparisonResult {
                    status: CheckStatus::Outdated,
                    detail: format!("Boolean differs: expected {}, got {}", expected_b, actual_b),
                }
            }
        }
        // Nulls
        (serde_json::Value::Null, serde_json::Value::Null) => ComparisonResult {
            status: CheckStatus::Ok,
            detail: "Both null".to_string(),
        },
        // Type mismatch
        _ => ComparisonResult {
            status: CheckStatus::Outdated,
            detail: format!(
                "Type mismatch: expected {:?}, got {:?}",
                value_type(expected),
                value_type(actual)
            ),
        },
    }
}

/// Get a human-readable type name for a JSON value.
fn value_type(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match_returns_ok() {
        let result = compare_outputs(r#"{"status": "ok"}"#, r#"{"status": "ok"}"#);
        assert_eq!(result.status, CheckStatus::Ok);
    }

    #[test]
    fn different_output_returns_outdated() {
        let result = compare_outputs(r#"{"status": "ok"}"#, r#"{"status": "error"}"#);
        assert_eq!(result.status, CheckStatus::Outdated);
    }

    #[test]
    fn latency_difference_returns_warning() {
        let actual = r#"{"latency_ms": 850}"#;
        let expected = r#"{"latency_ms": 1218}"#;
        let result = compare_outputs(actual, expected);
        assert_eq!(result.status, CheckStatus::Warning);
    }

    #[test]
    fn whitespace_normalization() {
        let result = compare_outputs("{\n  \"a\": 1\n}", "{\"a\": 1}");
        assert_eq!(result.status, CheckStatus::Ok);
    }

    #[test]
    fn missing_json_key_returns_outdated() {
        let result = compare_outputs(
            r#"{"status": "ok"}"#,
            r#"{"status": "ok", "version": "1.0"}"#,
        );
        assert_eq!(result.status, CheckStatus::Outdated);
    }

    #[test]
    fn extra_json_key_returns_outdated() {
        let result = compare_outputs(
            r#"{"status": "ok", "version": "1.0"}"#,
            r#"{"status": "ok"}"#,
        );
        assert_eq!(result.status, CheckStatus::Outdated);
    }
}
