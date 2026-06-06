use std::path::{Path, PathBuf};

/// How the workspace was determined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectionMethod {
    /// Automatically found .cite/cite.db or .cite.db in cwd or ancestors.
    AutoDetected,
    /// User passed --global flag to force global-only mode.
    ExplicitFlag,
    /// No project workspace found; using global only.
    NoProjectFound,
}

/// Which workspace tier is active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceType {
    /// Only the global DB is in use.
    GlobalOnly,
    /// A project DB is active at the given path.
    Project { path: PathBuf },
}

/// Where to direct an operation (e.g., ingest).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestTarget {
    /// Ingest into the project DB (default when workspace active).
    Project,
    /// Ingest into the global DB.
    Global,
}

/// Resolved workspace configuration.
#[derive(Debug, Clone)]
pub struct WorkspaceConfig {
    /// Path to the global data directory (contains cite.db).
    pub global_data_dir: PathBuf,
    /// Path to the project data directory (contains cite.db), if detected.
    pub project_data_dir: Option<PathBuf>,
    /// Active workspace type.
    pub active_workspace: WorkspaceType,
    /// How the workspace was determined.
    pub detection_method: DetectionMethod,
}

impl WorkspaceConfig {
    /// Whether a project workspace is active.
    pub fn has_project(&self) -> bool {
        matches!(self.active_workspace, WorkspaceType::Project { .. })
    }

    /// Get the project data directory if active.
    pub fn project_dir(&self) -> Option<&Path> {
        self.project_data_dir.as_deref()
    }
}

/// Resolve the workspace configuration for the given working directory.
///
/// Detection order:
/// 1. If `force_global` is true, return global-only.
/// 2. Walk up from `cwd` looking for `.cite/cite.db` (preferred) or `.cite.db` in root.
/// 3. Stop at git root or filesystem root.
/// 4. If nothing found, return global-only.
pub fn resolve_workspace(
    cwd: &Path,
    global_data_dir: PathBuf,
    force_global: bool,
) -> WorkspaceConfig {
    if force_global {
        return WorkspaceConfig {
            global_data_dir,
            project_data_dir: None,
            active_workspace: WorkspaceType::GlobalOnly,
            detection_method: DetectionMethod::ExplicitFlag,
        };
    }

    // Walk up from cwd looking for project workspace
    let mut dir = cwd.to_path_buf();
    loop {
        // Check for .cite/cite.db (preferred)
        let cite_dir = dir.join(".cite").join("cite.db");
        if cite_dir.exists() {
            let project_dir = dir.join(".cite");
            return WorkspaceConfig {
                global_data_dir,
                project_data_dir: Some(project_dir),
                active_workspace: WorkspaceType::Project {
                    path: dir.join(".cite"),
                },
                detection_method: DetectionMethod::AutoDetected,
            };
        }

        // Check for .cite.db in root (fallback)
        let root_db = dir.join(".cite.db");
        if root_db.exists() {
            // For .cite.db in root, we treat the directory itself as the data dir
            // since Database::open expects a directory and appends "cite.db"
            return WorkspaceConfig {
                global_data_dir,
                project_data_dir: Some(dir.clone()),
                active_workspace: WorkspaceType::Project { path: dir.clone() },
                detection_method: DetectionMethod::AutoDetected,
            };
        }

        // Stop at git root
        if dir.join(".git").exists() && dir != cwd {
            break;
        }

        // Walk up
        if !dir.pop() {
            break;
        }
    }

    WorkspaceConfig {
        global_data_dir,
        project_data_dir: None,
        active_workspace: WorkspaceType::GlobalOnly,
        detection_method: DetectionMethod::NoProjectFound,
    }
}

/// Check if a workspace has been initialized in the given directory.
///
/// Returns true if `.cite/cite.db` or `.cite.db` exists.
pub fn workspace_exists(dir: &Path) -> bool {
    dir.join(".cite").join("cite.db").exists() || dir.join(".cite.db").exists()
}

/// Get the preferred workspace DB path for a directory.
///
/// Returns `.cite/cite.db` if `.cite/` exists, otherwise `.cite.db` in root.
pub fn preferred_workspace_path(dir: &Path) -> PathBuf {
    if dir.join(".cite").exists() {
        dir.join(".cite")
    } else {
        dir.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn force_global_returns_global_only() {
        let cwd = PathBuf::from("/some/project");
        let global = PathBuf::from("/home/user/.local/share/cite");
        let config = resolve_workspace(&cwd, global.clone(), true);

        assert_eq!(config.active_workspace, WorkspaceType::GlobalOnly);
        assert_eq!(config.detection_method, DetectionMethod::ExplicitFlag);
        assert_eq!(config.global_data_dir, global);
        assert!(config.project_data_dir.is_none());
    }

    #[test]
    fn detect_cite_dir_in_cwd() {
        let tmp = tempfile::tempdir().unwrap();
        let cite_dir = tmp.path().join(".cite");
        fs::create_dir_all(&cite_dir).unwrap();
        // Create a dummy file to simulate cite.db
        fs::write(cite_dir.join("cite.db"), b"").unwrap();

        let global = PathBuf::from("/home/user/.local/share/cite");
        let config = resolve_workspace(tmp.path(), global, false);

        assert_eq!(config.detection_method, DetectionMethod::AutoDetected);
        assert!(config.has_project());
        assert_eq!(config.project_data_dir, Some(cite_dir));
    }

    #[test]
    fn detect_cite_db_in_root() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join(".cite.db"), b"").unwrap();

        let global = PathBuf::from("/home/user/.local/share/cite");
        let config = resolve_workspace(tmp.path(), global, false);

        assert_eq!(config.detection_method, DetectionMethod::AutoDetected);
        assert!(config.has_project());
    }

    #[test]
    fn no_workspace_found() {
        let tmp = tempfile::tempdir().unwrap();
        let global = PathBuf::from("/home/user/.local/share/cite");
        let config = resolve_workspace(tmp.path(), global, false);

        assert_eq!(config.detection_method, DetectionMethod::NoProjectFound);
        assert!(!config.has_project());
    }

    #[test]
    fn workspace_exists_checks_correctly() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!workspace_exists(tmp.path()));

        let cite_dir = tmp.path().join(".cite");
        fs::create_dir_all(&cite_dir).unwrap();
        fs::write(cite_dir.join("cite.db"), b"").unwrap();
        assert!(workspace_exists(tmp.path()));
    }

    #[test]
    fn preferred_workspace_path_prefers_cite_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let cite_dir = tmp.path().join(".cite");
        fs::create_dir_all(&cite_dir).unwrap();

        assert_eq!(preferred_workspace_path(tmp.path()), cite_dir);
    }

    #[test]
    fn preferred_workspace_path_falls_back_to_root() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(preferred_workspace_path(tmp.path()), tmp.path());
    }
}
