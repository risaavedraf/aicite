use clap::Args;
use common::{CiteError, ExitCode, ReadSelector};
use config::Config;
use engine::context;
use std::path::PathBuf;

use crate::output::print_json;

#[derive(Args)]
pub struct ReadArgs {
    /// Citation ID (requires --trace-id)
    #[arg(long)]
    pub citation_id: Option<String>,

    /// Trace ID (required with --citation-id)
    #[arg(long)]
    pub trace_id: Option<String>,

    /// Chunk ID (requires --document-id)
    #[arg(long)]
    pub chunk_id: Option<String>,

    /// Document ID (required with --chunk-id)
    #[arg(long)]
    pub document_id: Option<String>,
}

pub fn execute(args: &ReadArgs, config: &Config, json: bool) -> i32 {
    let selector = match build_selector(args) {
        Ok(s) => s,
        Err(e) => {
            if json {
                print_json(&e.to_json_response());
            } else {
                eprintln!("Error: {e}");
            }
            return e.exit_code() as i32;
        }
    };

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

    match context::read_context(&db, selector) {
        Ok(response) => {
            if json {
                print_json(&response);
            } else {
                let label = response
                    .display_name
                    .unwrap_or_else(|| response.document_id.clone());
                println!("Document: {}", label);
                println!("Chunk: {}", response.chunk_id);
                if let Some(page) = response.page {
                    println!("Page: {}", page);
                }
                if let Some(trace_id) = &response.trace_id {
                    println!("Trace: {}", trace_id);
                }
                if let Some(score) = response.score {
                    println!("Score: {:.4}", score);
                }
                if let Some(label) = &response.confidence_label {
                    println!("Confidence: {}", label);
                }
                println!();
                println!("{}", response.text);
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

fn build_selector(args: &ReadArgs) -> Result<ReadSelector, CiteError> {
    let has_citation = args.citation_id.is_some();
    let has_chunk = args.chunk_id.is_some();

    if has_citation && has_chunk {
        return Err(CiteError::InvalidParameter {
            message: "Cannot specify both --citation-id and --chunk-id".into(),
        });
    }

    if !has_citation && !has_chunk {
        return Err(CiteError::InvalidParameter {
            message: "Must specify either --citation-id --trace-id or --chunk-id --document-id"
                .into(),
        });
    }

    if has_citation {
        let citation_id = args.citation_id.clone().unwrap();
        let trace_id = args
            .trace_id
            .clone()
            .ok_or_else(|| CiteError::InvalidParameter {
                message: "--trace-id is required when using --citation-id".into(),
            })?;
        Ok(ReadSelector::Citation {
            trace_id,
            citation_id,
        })
    } else {
        let chunk_id = args.chunk_id.clone().unwrap();
        let document_id = args
            .document_id
            .clone()
            .ok_or_else(|| CiteError::InvalidParameter {
                message: "--document-id is required when using --chunk-id".into(),
            })?;
        Ok(ReadSelector::Chunk {
            document_id,
            chunk_id,
        })
    }
}

fn resolve_data_dir(config: &Config) -> PathBuf {
    config.paths.data_dir.clone().unwrap_or_else(|| {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("cite")
    })
}
