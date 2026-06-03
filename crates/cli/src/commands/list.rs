use common::ExitCode;
use config::Config;
use engine::ingest;
use serde::Serialize;

use super::{exit_for_error, CommandContext};
use crate::output::print_json;

#[derive(Serialize)]
struct ListOutput {
    documents: Vec<DocumentSummary>,
}

#[derive(Serialize)]
struct DocumentSummary {
    document_id: String,
    display_name: String,
    status: String,
    chunk_count: u32,
    retry_count: u32,
    created_at: String,
}

pub fn execute(config: &Config, json: bool) -> i32 {
    let ctx = match CommandContext::open_db_only(config, json) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };

    match ingest::list_documents(&ctx.db) {
        Ok(docs) => {
            let summaries: Vec<DocumentSummary> = docs
                .iter()
                .map(|d| DocumentSummary {
                    document_id: d.document_id.clone(),
                    display_name: d.display_name.clone(),
                    status: d.status.to_string(),
                    chunk_count: d.chunk_count,
                    retry_count: d.retry_count,
                    created_at: d.created_at.to_rfc3339(),
                })
                .collect();

            if json {
                print_json(&ListOutput {
                    documents: summaries,
                });
            } else if summaries.is_empty() {
                println!("No documents found.");
            } else {
                println!("Documents ({}):", summaries.len());
                for d in &summaries {
                    println!(
                        "  {} [{}] — {} chunks",
                        d.document_id, d.status, d.chunk_count
                    );
                    println!("    {}", d.display_name);
                }
            }
            ExitCode::Success as i32
        }
        Err(e) => exit_for_error(&e, json),
    }
}
