use clap::{ArgGroup, Args};
use common::ExitCode;
use config::Config;
use engine::ingest;
use providers::gemini::GeminiProvider;
use providers::openai::OpenAICompatibleProvider;
use providers::EmbeddingProvider;
use serde::Serialize;
use std::path::PathBuf;

use crate::output::print_json;

#[derive(Args)]
#[command(group(
    ArgGroup::new("ingest_mode")
        .args(["path", "queued", "next"])
        .required(true)
        .multiple(false)
))]
pub struct IngestArgs {
    /// Path to the file to ingest immediately
    pub path: Option<String>,

    /// Queue a file for later ingest (no immediate processing)
    #[arg(long, value_name = "PATH")]
    pub queued: Option<String>,

    /// Process the next queued ingest item
    #[arg(long)]
    pub next: bool,

    /// Override display name
    #[arg(long)]
    pub display_name: Option<String>,
}

#[derive(Serialize)]
struct IngestOutput {
    document_id: String,
    display_name: String,
    status: String,
    chunk_count: u32,
}

#[derive(Serialize)]
struct QueuedIngestOutput {
    status: String,
    source_path: String,
    display_name: Option<String>,
}

#[derive(Serialize)]
struct NextIngestOutput {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    document_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    chunk_count: Option<u32>,
}

pub fn execute(args: &IngestArgs, config: &Config, json: bool) -> i32 {
    // Initialize database
    let data_dir = resolve_data_dir(config);
    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        eprintln!("Failed to create data directory: {e}");
        return ExitCode::Internal as i32;
    }
    let db = match storage::Database::open(&data_dir) {
        Ok(db) => db,
        Err(e) => {
            if json {
                print_json(&e.to_json_response());
            } else {
                eprintln!("Error: {e}");
            }
            return e.exit_code() as i32;
        }
    };

    // Create embedding provider based on config
    let provider: Box<dyn EmbeddingProvider> = match create_provider(config) {
        Ok(p) => p,
        Err(e) => {
            if json {
                print_json(&e.to_json_response());
            } else {
                eprintln!("Error: {e}");
            }
            return e.exit_code() as i32;
        }
    };

    let production_mode = config.runtime.mode == config::RuntimeMode::Production;

    if let Some(path) = args.queued.as_deref() {
        let path = std::path::Path::new(path);
        match ingest::enqueue_ingest(&db, &config.ingest, path, args.display_name.as_deref()) {
            Ok(()) => {
                let output = QueuedIngestOutput {
                    status: "queued".to_string(),
                    source_path: path.display().to_string(),
                    display_name: args.display_name.clone(),
                };
                if json {
                    print_json(&output);
                } else {
                    println!("✓ Queued ingest: {}", output.source_path);
                    if let Some(name) = output.display_name {
                        println!("  Display name: {}", name);
                    }
                }
                return ExitCode::Success as i32;
            }
            Err(e) => {
                if json {
                    print_json(&e.to_json_response());
                } else {
                    eprintln!("Error: {e}");
                }
                return e.exit_code() as i32;
            }
        }
    }

    if args.next {
        match ingest::ingest_next(&db, provider.as_ref(), &config.ingest, production_mode) {
            Ok(ingest::IngestNextResult::Empty) => {
                let output = NextIngestOutput {
                    status: "empty_queue".to_string(),
                    document_id: None,
                    display_name: None,
                    chunk_count: None,
                };
                if json {
                    print_json(&output);
                } else {
                    println!("✓ Ingest queue is empty");
                }
                return ExitCode::Success as i32;
            }
            Ok(ingest::IngestNextResult::Ingested(result)) => {
                let output = NextIngestOutput {
                    status: result.status.to_string(),
                    document_id: Some(result.document_id),
                    display_name: Some(result.display_name),
                    chunk_count: Some(result.chunk_count),
                };
                if json {
                    print_json(&output);
                } else {
                    println!("✓ Ingested next queued item");
                    if let Some(doc_id) = output.document_id {
                        println!("  Document ID: {}", doc_id);
                    }
                    if let Some(name) = output.display_name {
                        println!("  Display name: {}", name);
                    }
                    if let Some(chunks) = output.chunk_count {
                        println!("  Chunks: {}", chunks);
                    }
                }
                return ExitCode::Success as i32;
            }
            Err(e) => {
                if json {
                    print_json(&e.to_json_response());
                } else {
                    eprintln!("Error: {e}");
                }
                return e.exit_code() as i32;
            }
        }
    }

    let Some(path) = args.path.as_deref() else {
        eprintln!("Error: missing path");
        return ExitCode::Validation as i32;
    };
    let path = std::path::Path::new(path);

    match ingest::ingest(
        &db,
        provider.as_ref(),
        &config.ingest,
        path,
        args.display_name.as_deref(),
        production_mode,
    ) {
        Ok(result) => {
            let output = IngestOutput {
                document_id: result.document_id,
                display_name: result.display_name,
                status: result.status.to_string(),
                chunk_count: result.chunk_count,
            };
            if json {
                print_json(&output);
            } else {
                println!("✓ Ingested: {}", output.display_name);
                println!("  Document ID: {}", output.document_id);
                println!("  Status: {}", output.status);
                println!("  Chunks: {}", output.chunk_count);
            }
            ExitCode::Success as i32
        }
        Err(e) => {
            if json {
                print_json(&e.to_json_response());
            } else {
                eprintln!("Error: {e}");
            }
            e.exit_code() as i32
        }
    }
}

/// Create an embedding provider based on config.
///
/// Supported providers:
/// - `gemini`: Google Gemini API (free tier available)
/// - `openai-compatible` (default): Any OpenAI-compatible API
fn create_provider(config: &Config) -> Result<Box<dyn EmbeddingProvider>, common::CiteError> {
    let api_key = std::env::var("CITE_EMBEDDING_API_KEY")
        .or_else(|_| std::env::var("GEMINI_API_KEY"))
        .or_else(|_| std::env::var("OPENAI_API_KEY"))
        .unwrap_or_default();

    match config.embedding.provider.as_str() {
        "gemini" => {
            let provider = GeminiProvider::new(&config.embedding.model, &api_key)?;
            Ok(Box::new(provider))
        }
        _ => {
            // Default: OpenAI-compatible
            let endpoint = config
                .ingest
                .embedding_endpoint
                .as_deref()
                .unwrap_or("https://api.openai.com/v1/embeddings");

            let provider =
                OpenAICompatibleProvider::new(endpoint, &config.embedding.model, &api_key)?;
            Ok(Box::new(provider))
        }
    }
}

fn resolve_data_dir(config: &Config) -> PathBuf {
    config.paths.data_dir.clone().unwrap_or_else(|| {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("cite")
    })
}
