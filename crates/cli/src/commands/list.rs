use common::ExitCode;
use config::Config;
use engine::ingest;
use serde::Serialize;

use super::resolve_data_dir;
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

    match ingest::list_documents(&db) {
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
