use clap::{Parser, Subcommand};
use common::ExitCode;
use config::Config;
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

    let exit_code = match cli.command {
        Commands::Health => commands::health::execute(&config, cli.json),
        Commands::Ingest(args) => commands::ingest::execute(&args, &config, cli.json),
        Commands::List => commands::list::execute(&config, cli.json),
        Commands::Get(args) => commands::get::execute(&args, &config, cli.json),
        Commands::Retry(args) => commands::retry::execute(&args, &config, cli.json),
    };

    process::exit(exit_code);
}
