use super::CommandContext;
use crate::output::print_json;
use clap::Args;
use common::ExitCode;
use config::Config;
use engine::context;

#[derive(Args)]
pub struct TraceArgs {
    /// Trace ID to look up
    pub trace_id: String,
}

pub fn execute(args: &TraceArgs, config: &Config, json: bool) -> i32 {
    let ctx = match CommandContext::open(config, json) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    let db = &ctx.db;
    let provider = match ctx.provider.as_ref() {
        Some(p) => p,
        None => {
            eprintln!("Error: embedding provider not configured");
            return ExitCode::Validation as i32;
        }
    };

    match context::get_trace(db, provider.as_ref(), &args.trace_id) {
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
