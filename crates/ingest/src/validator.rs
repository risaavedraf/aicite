use common::CiteError;
use common::FileType;
use std::path::Path;

/// Validate a file for ingestion: path safety, existence, type, and size.
///
/// Returns `(FileType, file_size_bytes)` on success.
pub fn validate_file(path: &Path, max_file_size_bytes: u64) -> Result<(FileType, u64), CiteError> {
    // Path policy checks before any filesystem access
    is_path_safe(path)?;

    // Resolve symlinks to prevent symlink escape
    let canonical = std::fs::canonicalize(path).map_err(|_| CiteError::FileNotFound {
        path: path.to_path_buf(),
    })?;

    // Re-check the canonical path against policy (canonicalize resolves .. but we
    // still need to reject network and device paths on the resolved form)
    reject_network_path(&canonical)?;
    reject_device_path(&canonical)?;

    // Must be a regular file
    if !canonical.is_file() {
        return Err(CiteError::FileNotFound {
            path: path.to_path_buf(),
        });
    }

    // Extension → FileType
    let file_type = canonical
        .extension()
        .and_then(|ext| ext.to_str())
        .and_then(FileType::from_extension)
        .ok_or_else(|| {
            let file_type = canonical
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("<none>")
                .to_string();
            CiteError::UnsupportedFileType { file_type }
        })?;

    // Size check
    let metadata = std::fs::metadata(&canonical).map_err(|_| CiteError::FileNotFound {
        path: path.to_path_buf(),
    })?;
    let size = metadata.len();
    if size > max_file_size_bytes {
        return Err(CiteError::FileTooLarge {
            size_bytes: size,
            max_bytes: max_file_size_bytes,
        });
    }

    Ok((file_type, size))
}

/// Derive a display name for a document.
///
/// Priority: override_name → production generic → path filename.
pub fn derive_display_name(
    path: &Path,
    override_name: Option<&str>,
    production_mode: bool,
) -> String {
    if let Some(name) = override_name {
        return sanitize_display_name(name);
    }
    if production_mode {
        return "document".to_string();
    }
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document")
        .to_string()
}

/// Sanitize a display name for safe use.
///
/// Removes path separators, null bytes, and control characters.
/// Trims whitespace and truncates to 255 characters.
pub fn sanitize_display_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .filter(|c| {
            // Keep everything except path separators, null bytes, and control chars
            !c.is_control() && *c != '\0' && *c != '/' && *c != '\\' && *c != ':'
        })
        .collect();
    // Strip leading dots (removes path traversal residue like ".." "/../")
    let stripped = sanitized.trim_start_matches('.');
    let trimmed = stripped.trim();
    if trimmed.is_empty() {
        return "document".to_string();
    }
    if trimmed.len() > 255 {
        trimmed[..255].to_string()
    } else {
        trimmed.to_string()
    }
}

/// Check that a path is safe for file access.
///
/// Rejects path traversal (`..`), UNC paths (`\\`), and device file paths (`/dev/`).
pub fn is_path_safe(path: &Path) -> Result<(), CiteError> {
    let path_str = path.to_string_lossy();

    // Reject UNC / network paths (\\server or //server)
    // but allow Windows extended-length paths (\\?\...)
    if path_str.starts_with("\\\\") && !path_str.starts_with("\\\\?\\") {
        return Err(CiteError::PathRejected {
            message: "Network paths are not allowed".to_string(),
        });
    }
    if path_str.starts_with("//") {
        return Err(CiteError::PathRejected {
            message: "Network paths are not allowed".to_string(),
        });
    }

    // Reject device file paths
    if path_str.starts_with("/dev/") || path_str.starts_with("\\\\.\\") {
        return Err(CiteError::PathRejected {
            message: "Device file paths are not allowed".to_string(),
        });
    }

    // Reject .. components (path traversal)
    for component in path.components() {
        if let std::path::Component::ParentDir = component {
            return Err(CiteError::PathRejected {
                message: "Path traversal (..) is not allowed".to_string(),
            });
        }
    }

    Ok(())
}

/// Reject network/UNC paths on an already-resolved path.
///
/// On Windows, `std::fs::canonicalize` returns extended-length paths like
/// `\\?\C:\...` which are local, not network. We allow `\\?\` but reject
/// `\\?\UNC\` (extended-length UNC) and bare `\\server` (standard UNC).
fn reject_network_path(path: &Path) -> Result<(), CiteError> {
    let s = path.to_string_lossy();
    // Reject extended-length UNC paths
    if s.starts_with("\\\\?\\UNC\\") || s.starts_with("\\\\?\\unc\\") {
        return Err(CiteError::PathRejected {
            message: "Network paths are not allowed".to_string(),
        });
    }
    // Reject bare UNC paths but allow \\?\ extended-length local paths
    if s.starts_with("\\\\") && !s.starts_with("\\\\?\\") {
        return Err(CiteError::PathRejected {
            message: "Network paths are not allowed".to_string(),
        });
    }
    if s.starts_with("//") {
        return Err(CiteError::PathRejected {
            message: "Network paths are not allowed".to_string(),
        });
    }
    Ok(())
}

/// Reject device file paths on an already-resolved path.
fn reject_device_path(path: &Path) -> Result<(), CiteError> {
    let s = path.to_string_lossy();
    if s.starts_with("/dev/") || s.starts_with("\\\\.\\") {
        return Err(CiteError::PathRejected {
            message: "Device file paths are not allowed".to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_dir() -> PathBuf {
        std::env::temp_dir()
    }

    #[test]
    fn test_validate_txt_file() {
        let dir = temp_dir();
        let file_path = dir.join("aicite_test_validator.txt");
        fs::write(&file_path, b"hello world").expect("write temp file");

        let result = validate_file(&file_path, 1024);
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result.err());
        let (file_type, size) = result.unwrap();
        assert_eq!(file_type, FileType::Txt);
        assert_eq!(size, 11);

        let _ = fs::remove_file(&file_path);
    }

    #[test]
    fn test_validate_unsupported_type() {
        let dir = temp_dir();
        let file_path = dir.join("aicite_test_validator.csv");
        fs::write(&file_path, b"a,b,c").expect("write temp file");

        let result = validate_file(&file_path, 1024);
        assert!(result.is_err());
        match result.unwrap_err() {
            CiteError::UnsupportedFileType { file_type } => {
                assert_eq!(file_type, "csv");
            }
            other => panic!("Expected UnsupportedFileType, got: {:?}", other),
        }

        let _ = fs::remove_file(&file_path);
    }

    #[test]
    fn test_validate_missing_file() {
        let file_path = temp_dir().join("aicite_nonexistent_12345.txt");
        let result = validate_file(&file_path, 1024);
        assert!(result.is_err());
        match result.unwrap_err() {
            CiteError::FileNotFound { .. } => {}
            other => panic!("Expected FileNotFound, got: {:?}", other),
        }
    }

    #[test]
    fn test_validate_path_traversal() {
        let file_path = PathBuf::from("/some/path/../../../etc/passwd");
        let result = validate_file(&file_path, 1024);
        assert!(result.is_err());
        match result.unwrap_err() {
            CiteError::PathRejected { .. } => {}
            other => panic!("Expected PathRejected, got: {:?}", other),
        }
    }

    #[test]
    fn test_validate_file_too_large() {
        let dir = temp_dir();
        let file_path = dir.join("aicite_test_validator_large.txt");
        fs::write(&file_path, b"hello world").expect("write temp file");

        // Set max_file_size_bytes to 5, which is smaller than 11 bytes
        let result = validate_file(&file_path, 5);
        assert!(result.is_err());
        match result.unwrap_err() {
            CiteError::FileTooLarge {
                size_bytes,
                max_bytes,
            } => {
                assert_eq!(size_bytes, 11);
                assert_eq!(max_bytes, 5);
            }
            other => panic!("Expected FileTooLarge, got: {:?}", other),
        }

        let _ = fs::remove_file(&file_path);
    }

    #[test]
    fn test_derive_display_name_with_override() {
        let path = PathBuf::from("/some/path/document.txt");
        let name = derive_display_name(&path, Some("My Custom Name"), false);
        assert_eq!(name, "My Custom Name");
    }

    #[test]
    fn test_derive_display_name_with_override_sanitized() {
        let path = PathBuf::from("/some/path/document.txt");
        let name = derive_display_name(&path, Some("../evil/path:name"), false);
        // Path separators and traversal characters should be removed
        assert_eq!(name, "evilpathname");
    }

    #[test]
    fn test_derive_display_name_from_path() {
        let path = PathBuf::from("/some/path/document.txt");
        let name = derive_display_name(&path, None, false);
        assert_eq!(name, "document.txt");
    }

    #[test]
    fn test_derive_display_name_production_mode() {
        let path = PathBuf::from("/some/path/document.txt");
        let name = derive_display_name(&path, None, true);
        assert_eq!(name, "document");
    }

    #[test]
    fn test_derive_display_name_production_overridden_by_override() {
        let path = PathBuf::from("/some/path/document.txt");
        let name = derive_display_name(&path, Some("custom"), true);
        assert_eq!(name, "custom");
    }

    #[test]
    fn test_sanitize_display_name() {
        assert_eq!(sanitize_display_name("hello.txt"), "hello.txt");
        assert_eq!(sanitize_display_name("../path/to/file"), "pathtofile");
        assert_eq!(sanitize_display_name("file\0name"), "filename");
        assert_eq!(sanitize_display_name("  spaced  "), "spaced");
        assert_eq!(
            sanitize_display_name("normal-name_v2.md"),
            "normal-name_v2.md"
        );
        // Empty after sanitization
        assert_eq!(sanitize_display_name("///:::\0"), "document");
        // Null byte removal
        assert_eq!(sanitize_display_name("file\x00name"), "filename");
    }

    #[test]
    fn test_sanitize_display_name_truncation() {
        let long_name = "a".repeat(300);
        let result = sanitize_display_name(&long_name);
        assert_eq!(result.len(), 255);
    }

    #[test]
    fn test_is_path_safe_normal() {
        assert!(is_path_safe(Path::new("/home/user/document.txt")).is_ok());
        assert!(is_path_safe(Path::new("relative/path/file.txt")).is_ok());
    }

    #[test]
    fn test_is_path_safe_traversal() {
        let result = is_path_safe(Path::new("/home/user/../../../etc/passwd"));
        assert!(result.is_err());
        match result.unwrap_err() {
            CiteError::PathRejected { .. } => {}
            other => panic!("Expected PathRejected, got: {:?}", other),
        }
    }

    #[test]
    fn test_is_path_safe_unc() {
        // UNC paths on Windows start with \\
        let result = is_path_safe(Path::new("//server/share/file.txt"));
        assert!(result.is_err());
        match result.unwrap_err() {
            CiteError::PathRejected { message } => {
                assert!(message.contains("Network"));
            }
            other => panic!("Expected PathRejected, got: {:?}", other),
        }
    }

    #[test]
    fn test_is_path_safe_device() {
        let result = is_path_safe(Path::new("/dev/null"));
        assert!(result.is_err());
        match result.unwrap_err() {
            CiteError::PathRejected { message } => {
                assert!(message.contains("Device"));
            }
            other => panic!("Expected PathRejected, got: {:?}", other),
        }
    }
}
