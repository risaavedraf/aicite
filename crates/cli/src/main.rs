use clap::{Parser, Subcommand};
use common::{CiteError, ExitCode};
use config::{Config, RuntimeMode};
use std::{path::PathBuf, process};

mod commands;
mod output;

/// AI Cite CLI — semantic document context engine for AI agents
#[derive(Parser)]
#[command(name = "cite", version, about)]
struct Cli {
    /// Output format
    #[arg(long, global = true)]
    json: bool,

    /// Config file path
    #[arg(long, global = true)]
    config: Option<String>,

    /// Data directory
    #[arg(long, global = true)]
    data_dir: Option<String>,

    /// Runtime mode
    #[arg(long, global = true)]
    runtime_mode: Option<String>,

    /// Suppress provider disclosure banner
    #[arg(long, global = true)]
    no_banner: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check CLI runtime, storage, and provider health
    Health,
    /// Configure API keys and provider settings
    Setup(commands::setup::SetupArgs),
    /// Ingest a document into the corpus
    Ingest(commands::ingest::IngestArgs),
    /// List documents in the corpus
    List,
    /// Get document metadata
    Get(commands::get::GetArgs),
    /// Retry a failed document
    Retry(commands::retry::RetryArgs),
    /// Search the ready corpus using vector similarity
    Search(commands::search::SearchArgs),
    /// Retrieve top-ranked chunks with full text
    Retrieve(commands::retrieve::RetrieveArgs),
    /// Build an agent-consumable context pack with citations
    Context(commands::context::ContextArgs),
    /// Read a citation or chunk by ID
    Read(commands::read::ReadArgs),
    /// Look up trace metadata for a context/retrieval request
    Trace(commands::trace::TraceArgs),
    /// Refresh corpus with atomic snapshot swap
    Refresh,
    /// Run golden dataset evaluation to verify retrieval quality
    Evaluate(commands::evaluate::EvaluateArgs),
}

fn main() {
    // Load .env file if present (silently ignore if not found)
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();

    let config_path = cli.config.as_deref().map(std::path::Path::new);
    let mut config = match Config::load_from(config_path) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Configuration error: {e}");
            process::exit(ExitCode::Validation as i32);
        }
    };

    if let Err(e) = apply_cli_overrides(
        &mut config,
        cli.data_dir.as_deref(),
        cli.runtime_mode.as_deref(),
    ) {
        eprintln!("Configuration error: {e}");
        process::exit(ExitCode::Validation as i32);
    }

    // Show provider disclosure banner for real (non-eval) providers
    if !cli.no_banner && !cli.json && is_retrieval_command(&cli.command) {
        show_provider_disclosure(&config);
    }

    if should_run_startup_recovery(&cli.command) {
        if let Err(e) = run_startup_recovery(&config, cli.json) {
            if cli.json {
                output::print_json(&e.to_json_response());
            } else {
                eprintln!("Error: {e}");
            }
            process::exit(e.exit_code() as i32);
        }
    }

    let exit_code = match cli.command {
        Commands::Health => {
            let cfg_path = cli.config.as_deref().map(std::path::Path::new);
            commands::health::execute(&config, cli.json, cfg_path)
        }
        Commands::Setup(args) => commands::setup::execute(&args, &config, cli.json),
        Commands::Ingest(args) => commands::ingest::execute(&args, &config, cli.json),
        Commands::List => commands::list::execute(&config, cli.json),
        Commands::Get(args) => commands::get::execute(&args, &config, cli.json),
        Commands::Retry(args) => commands::retry::execute(&args, &config, cli.json),
        Commands::Search(args) => commands::search::execute(&args, &config, cli.json),
        Commands::Retrieve(args) => commands::retrieve::execute(&args, &config, cli.json),
        Commands::Context(args) => commands::context::execute(&args, &config, cli.json),
        Commands::Read(args) => commands::read::execute(&args, &config, cli.json),
        Commands::Trace(args) => commands::trace::execute(&args, &config, cli.json),
        Commands::Refresh => commands::refresh::execute(&config, cli.json),
        Commands::Evaluate(args) => commands::evaluate::execute(&args, &config, cli.json),
    };

    process::exit(exit_code);
}

fn should_run_startup_recovery(command: &Commands) -> bool {
    !matches!(command, Commands::Health | Commands::Setup(_))
}

fn run_startup_recovery(config: &Config, _json: bool) -> Result<(), common::CiteError> {
    let data_dir = commands::resolve_data_dir(config);
    std::fs::create_dir_all(&data_dir).map_err(|e| common::CiteError::StorageError {
        message: format!("Failed to create data directory: {e}"),
    })?;

    let db = storage::Database::open(&data_dir)?;
    let _ = engine::recovery::recover_interrupted_processing(&db)?;
    Ok(())
}

/// Check if the command is a retrieval/context command that may send data to providers.
fn is_retrieval_command(command: &Commands) -> bool {
    matches!(
        command,
        Commands::Search(_)
            | Commands::Retrieve(_)
            | Commands::Context(_)
            | Commands::Read(_)
            | Commands::Trace(_)
    )
}

/// Show provider disclosure banner to stderr when using a real external provider.
fn show_provider_disclosure(config: &Config) {
    let provider_id = &config.embedding.provider;
    if engine::runtime_guard::is_real_provider(provider_id) {
        eprintln!(
            "⚠ Provider disclosure: Document snippets, query text, or embeddings may be sent\n  to your configured AI provider ({provider_id} / {}).\n  See README for privacy details.\n",
            config.embedding.model
        );
    }
}

fn apply_cli_overrides(
    config: &mut Config,
    data_dir: Option<&str>,
    runtime_mode: Option<&str>,
) -> Result<(), CiteError> {
    if let Some(dir) = data_dir {
        config.paths.data_dir = Some(PathBuf::from(dir));
    }

    if let Some(mode) = runtime_mode {
        config.runtime.mode = parse_runtime_mode(mode)?;
    }

    Ok(())
}

fn parse_runtime_mode(value: &str) -> Result<RuntimeMode, CiteError> {
    value.parse::<RuntimeMode>().map_err(|_| CiteError::ConfigError {
        message: format!(
            "Invalid --runtime-mode '{value}'. Expected one of: public_packaged_demo, local_private_demo, production"
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::{apply_cli_overrides, parse_runtime_mode};
    use config::{Config, RuntimeMode};
    use std::path::PathBuf;

    #[test]
    fn parse_runtime_mode_accepts_supported_values() {
        assert!(matches!(
            parse_runtime_mode("public_packaged_demo").unwrap(),
            RuntimeMode::PublicPackagedDemo
        ));
        assert!(matches!(
            parse_runtime_mode("local_private_demo").unwrap(),
            RuntimeMode::LocalPrivateDemo
        ));
        assert!(matches!(
            parse_runtime_mode("production").unwrap(),
            RuntimeMode::Production
        ));
    }

    #[test]
    fn apply_cli_overrides_sets_data_dir_and_runtime_mode() {
        let mut config = Config::load_from(None).unwrap();
        let expected_dir = PathBuf::from("/tmp/cite-test-dir");

        apply_cli_overrides(
            &mut config,
            Some(expected_dir.to_string_lossy().as_ref()),
            Some("production"),
        )
        .unwrap();

        assert_eq!(config.paths.data_dir, Some(expected_dir));
        assert!(matches!(config.runtime.mode, RuntimeMode::Production));
    }

    #[test]
    fn apply_cli_overrides_rejects_invalid_runtime_mode() {
        let mut config = Config::load_from(None).unwrap();

        let err = apply_cli_overrides(&mut config, None, Some("prod")).unwrap_err();

        assert!(format!("{err}").contains("Invalid --runtime-mode"));
    }
}
