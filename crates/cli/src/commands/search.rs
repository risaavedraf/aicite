use clap::Args;
use common::ExitCode;
use config::Config;
use engine::retrieve;
use providers::gemini::GeminiProvider;
use providers::openai::OpenAICompatibleProvider;
use providers::EmbeddingProvider;
use serde::Serialize;
use std::path::PathBuf;

use crate::output::{print_json, to_compact_search};

#[derive(Args)]
pub struct SearchArgs {
    /// Natural-language query
    pub query: String,

    /// Number of results (1..10)
    #[arg(long)]
    pub k: Option<u32>,

    /// Use flat retrieval (v0.1.0 behavior, no hierarchy)
    #[arg(long)]
    pub flat: bool,

    /// Filter results to a specific topic by name or ID
    #[arg(long)]
    pub topic: Option<String>,

    /// Filter results to a specific concept by name or ID
    #[arg(long)]
    pub concept: Option<String>,

    /// Return full JSON response (default: compact when --json is used)
    #[arg(long)]
    pub full: bool,
}

#[derive(Serialize)]
struct SearchOutput {
    query: String,
    top_k: u32,
    hit_count: usize,
    results: Vec<SearchResultItem>,
}

#[derive(Serialize)]
struct SearchResultItem {
    chunk_id: String,
    document_id: String,
    display_name: String,
    section_id: Option<String>,
    chunk_index: u32,
    page: Option<u32>,
    offset_start: Option<u32>,
    offset_end: Option<u32>,
    score: f32,
    preview: String,
    /// Topic name from hierarchy (Phase 11)
    topic_name: Option<String>,
    /// Concept name from hierarchy (Phase 11)
    concept_name: Option<String>,
    /// Breadcrumb path: "display_name > topic > concept" (Phase 11)
    breadcrumb: Option<String>,
}

pub fn execute(args: &SearchArgs, config: &Config, json: bool) -> i32 {
    // Validate flag combinations
    if args.flat && (args.topic.is_some() || args.concept.is_some()) {
        eprintln!("Error: --flat cannot be combined with --topic or --concept.");
        return common::ExitCode::Validation as i32;
    }
    if args.topic.is_some() && args.concept.is_some() {
        eprintln!("Error: --topic and --concept cannot be used together.");
        return common::ExitCode::Validation as i32;
    }

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

    let mut retrieval_config = config.retrieval.clone();
    if args.flat {
        retrieval_config.use_hierarchy = false;
    }

    let topic_filter = args.topic.as_deref();
    let concept_filter = args.concept.as_deref();

    match retrieve::search(
        &db,
        provider.as_ref(),
        &retrieval_config,
        &config.rate_limit,
        &args.query,
        args.k,
        topic_filter,
        concept_filter,
    ) {
        Ok(hits) => {
            if json {
                if args.full {
                    let output = SearchOutput {
                        query: args.query.clone(),
                        top_k: args.k.unwrap_or(config.retrieval.top_k),
                        hit_count: hits.len(),
                        results: hits
                            .into_iter()
                            .map(|h| SearchResultItem {
                                chunk_id: h.chunk_id,
                                document_id: h.document_id,
                                display_name: h.display_name,
                                section_id: h.section_id,
                                chunk_index: h.chunk_index,
                                page: h.page,
                                offset_start: h.offset_start,
                                offset_end: h.offset_end,
                                score: h.score,
                                preview: h.preview,
                                topic_name: h.topic_name,
                                concept_name: h.concept_name,
                                breadcrumb: h.breadcrumb,
                            })
                            .collect(),
                    };
                    print_json(&output);
                } else {
                    print_json(&to_compact_search(&hits));
                }
            } else if hits.is_empty() {
                println!("No results found.");
            } else {
                println!("Search results ({}):", hits.len());
                for (idx, hit) in hits.iter().enumerate() {
                    println!(
                        "  {}. {:.4} {}:{} [{}]",
                        idx + 1,
                        hit.score,
                        hit.display_name,
                        hit.chunk_index,
                        hit.chunk_id
                    );
                    println!("     {}", hit.preview);
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
