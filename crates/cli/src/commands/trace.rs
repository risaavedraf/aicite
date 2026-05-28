use clap::Args;
use common::ExitCode;
use config::Config;
use engine::context;
use providers::gemini::GeminiProvider;
use providers::openai::OpenAICompatibleProvider;
use providers::EmbeddingProvider;
use std::path::PathBuf;

use crate::output::print_json;

#[derive(Args)]
pub struct TraceArgs {
    /// Trace ID to look up
    pub trace_id: String,
}

pub fn execute(args: &TraceArgs, config: &Config, json: bool) -> i32 {
    let data_dir = resolve_data_dir(config);
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

    match context::get_trace(&db, provider.as_ref(), &args.trace_id) {
        Ok(response) => {
            if json {
                print_json(&response);
            } else {
                println!("Trace ID: {}", response.trace_id);
                if let Some(query_id) = &response.query_id {
                    println!("Query ID: {}", query_id);
                }
                if let Some(ctx_id) = &response.context_pack_id {
                    println!("Context Pack ID: {}", ctx_id);
                }
                println!("Timestamp: {}", response.timestamp);
                println!("Provider: {}", response.provider);
                println!("Model: {}", response.embedding_model_registry_id);
                println!(
                    "Ranking: {}",
                    response.ranking_method.as_deref().unwrap_or("n/a")
                );
                println!("Top-K: {}", response.retrieval_top_k.unwrap_or(0));
                println!(
                    "Thresholds: floor={:.2}, confidence={:.2}",
                    response.evidence_floor.unwrap_or(0.0),
                    response.confidence_threshold.unwrap_or(0.0)
                );
                println!("Documents: {}", response.document_ids.len());
                println!("Citations: {}", response.citation_ids.len());
                println!(
                    "Disclaimer shown: {}",
                    response.user_visible_disclaimer_shown
                );
                if let Some(owner) = &response.responsible_owner {
                    println!("Responsible owner: {}", owner);
                }
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
