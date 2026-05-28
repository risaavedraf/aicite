use clap::{Parser, Subcommand};
use common::ExitCode;
use config::Config;
use std::path::PathBuf;
use std::process;

mod commands;
mod output;

/// AI Harness CLI — semantic document context engine for AI agents
#[derive(Parser)]
#[command(name = "harness", version, about)]
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

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check CLI runtime and local state health
    Health,
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

    let config = match Config::load() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Configuration error: {e}");
            process::exit(ExitCode::Validation as i32);
        }
    };

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
        Commands::Health => commands::health::execute(&config, cli.json),
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
    !matches!(command, Commands::Health)
}

fn run_startup_recovery(config: &Config, _json: bool) -> Result<(), common::HarnessError> {
    let data_dir = resolve_data_dir(config);
    std::fs::create_dir_all(&data_dir).map_err(|e| common::HarnessError::StorageError {
        message: format!("Failed to create data directory: {e}"),
    })?;

    let db = storage::Database::open(&data_dir)?;
    let _ = engine::recovery::recover_interrupted_processing(&db)?;
    Ok(())
}

fn resolve_data_dir(config: &Config) -> PathBuf {
    config.paths.data_dir.clone().unwrap_or_else(|| {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("harness")
    })
}
