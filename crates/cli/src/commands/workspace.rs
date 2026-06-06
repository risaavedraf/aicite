use clap::{Args, Subcommand};
use common::ExitCode;
use config::Config;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

use super::{exit_for_error, resolve_data_dir};
use crate::output::print_json;

#[derive(Args)]
pub struct WorkspaceArgs {
    #[command(subcommand)]
    pub command: WorkspaceCommand,
}

#[derive(Subcommand)]
pub enum WorkspaceCommand {
    /// Initialize a project workspace in the current directory
    Init,
    /// Show workspace configuration and statistics
    Status,
}

#[derive(Serialize)]
struct WorkspaceStatusOutput {
    active_workspace: String,
    detection_method: String,
    resolution_strategy: String,
    global_db: DbInfo,
    project_db: Option<DbInfo>,
}

#[derive(Serialize)]
struct DbInfo {
    path: String,
    document_count: i64,
    chunk_count: i64,
}

#[derive(Serialize)]
struct WorkspaceInitOutput {
    status: String,
    project_path: String,
    global_db: DbInfo,
    project_db: DbInfo,
}

pub fn execute(args: &WorkspaceArgs, config: &Config, json: bool) -> i32 {
    match &args.command {
        WorkspaceCommand::Init => execute_init(config, json),
        WorkspaceCommand::Status => execute_status(config, json),
    }
}

fn execute_init(config: &Config, json: bool) -> i32 {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Check if workspace already exists
    let cite_dir = cwd.join(".cite");
    let db_path = cite_dir.join("cite.db");

    if db_path.exists() {
        let global_data_dir = resolve_data_dir(config);
        let global_db = storage::Database::open(&global_data_dir).ok();
        let project_db = storage::Database::open(&cite_dir).ok();

        if json {
            let output = WorkspaceInitOutput {
                status: "already_initialized".to_string(),
                project_path: cite_dir.display().to_string(),
                global_db: db_info_from_path(&global_data_dir, global_db.as_ref()),
                project_db: db_info_from_path(&cite_dir, project_db.as_ref()),
            };
            print_json(&output);
        } else {
            println!("Workspace already initialized at {}", cite_dir.display());
            print_db_stats("Global DB", &global_data_dir, global_db.as_ref());
            print_db_stats("Project DB", &cite_dir, project_db.as_ref());
        }
        return ExitCode::Success as i32;
    }

    // Create .cite directory
    if let Err(e) = fs::create_dir_all(&cite_dir) {
        let err = common::CiteError::StorageError {
            message: format!("Failed to create .cite directory: {e}"),
        };
        return exit_for_error(&err, json);
    }

    // Open database (this creates it with migrations)
    let project_db = match storage::Database::open(&cite_dir) {
        Ok(db) => db,
        Err(e) => return exit_for_error(&e, json),
    };

    let global_data_dir = resolve_data_dir(config);
    let global_db = storage::Database::open(&global_data_dir).ok();

    if json {
        let output = WorkspaceInitOutput {
            status: "initialized".to_string(),
            project_path: cite_dir.display().to_string(),
            global_db: db_info_from_path(&global_data_dir, global_db.as_ref()),
            project_db: db_info_from_path(&cite_dir, Some(&project_db)),
        };
        print_json(&output);
    } else {
        println!("✓ Workspace initialized at {}", cite_dir.display());
        print_db_stats("Global DB", &global_data_dir, global_db.as_ref());
        print_db_stats("Project DB", &cite_dir, Some(&project_db));
    }

    ExitCode::Success as i32
}

fn execute_status(config: &Config, json: bool) -> i32 {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let global_data_dir = resolve_data_dir(config);

    let ws_config = storage::workspace::resolve_workspace(&cwd, global_data_dir.clone(), false);

    let active = match &ws_config.active_workspace {
        storage::workspace::WorkspaceType::GlobalOnly => "global-only",
        storage::workspace::WorkspaceType::Project { .. } => "project",
    };

    let detection = match ws_config.detection_method {
        storage::workspace::DetectionMethod::AutoDetected => "auto",
        storage::workspace::DetectionMethod::ExplicitFlag => "explicit",
        storage::workspace::DetectionMethod::NoProjectFound => "none",
    };

    let global_db = storage::Database::open(&global_data_dir).ok();
    let project_db = ws_config
        .project_data_dir
        .as_ref()
        .and_then(|p| storage::Database::open(p).ok());

    if json {
        let output = WorkspaceStatusOutput {
            active_workspace: active.to_string(),
            detection_method: detection.to_string(),
            resolution_strategy: if ws_config.has_project() {
                "project_first"
            } else {
                "global_only"
            }
            .to_string(),
            global_db: db_info_from_path(&global_data_dir, global_db.as_ref()),
            project_db: ws_config
                .project_data_dir
                .as_ref()
                .map(|p| db_info_from_path(p, project_db.as_ref())),
        };
        print_json(&output);
    } else {
        println!("Workspace Status");
        println!("────────────────");
        println!("Active: {}", active);
        println!("Detection: {}", detection);
        println!(
            "Resolution: {}",
            if ws_config.has_project() {
                "project-first"
            } else {
                "global-only"
            }
        );
        println!();
        print_db_stats("Global DB", &global_data_dir, global_db.as_ref());
        println!();
        match &ws_config.project_data_dir {
            Some(p) => print_db_stats("Project DB", p, project_db.as_ref()),
            None => println!("Project DB: (none)"),
        }
    }

    ExitCode::Success as i32
}

fn db_info_from_path(data_dir: &std::path::Path, db: Option<&storage::Database>) -> DbInfo {
    DbInfo {
        path: data_dir.join("cite.db").display().to_string(),
        document_count: db.map(|d| d.document_count()).unwrap_or(0),
        chunk_count: db.map(|d| d.chunk_count()).unwrap_or(0),
    }
}

fn print_db_stats(label: &str, data_dir: &std::path::Path, db: Option<&storage::Database>) {
    let db_path = data_dir.join("cite.db");
    match db {
        Some(d) => {
            println!(
                "{}: {} ({} documents, {} chunks)",
                label,
                db_path.display(),
                d.document_count(),
                d.chunk_count()
            );
        }
        None => {
            println!("{}: {} (not accessible)", label, db_path.display());
        }
    }
}
