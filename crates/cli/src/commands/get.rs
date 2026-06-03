use clap::Args;
use common::ExitCode;
use config::Config;
use engine::ingest;
use serde::Serialize;

use super::{exit_for_error, CommandContext};
use crate::output::print_json;

#[derive(Args)]
pub struct GetArgs {
    /// Document ID
    pub document_id: String,
}

#[derive(Serialize)]
struct GetOutput {
    document_id: String,
    display_name: String,
    status: String,
    chunk_count: u32,
    retry_count: u32,
    max_retry_count: u32,
    next_retry_at: Option<String>,
    error: Option<ErrorOutput>,
}

#[derive(Serialize)]
struct ErrorOutput {
    code: String,
    message: String,
}

pub fn execute(args: &GetArgs, config: &Config, json: bool) -> i32 {
    let ctx = match CommandContext::open_db_only(config, json) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };

    match ingest::get_document(&ctx.db, &args.document_id) {
        Ok(doc) => {
            let output = GetOutput {
                document_id: doc.document_id,
                display_name: doc.display_name,
                status: doc.status.to_string(),
                chunk_count: doc.chunk_count,
                retry_count: doc.retry_count,
                max_retry_count: doc.max_retry_count,
                next_retry_at: doc.next_retry_at.map(|dt| dt.to_rfc3339()),
                error: doc.error.map(|e| ErrorOutput {
                    code: e.code,
                    message: e.message,
                }),
            };
            if json {
                print_json(&output);
            } else {
                println!("Document: {}", output.document_id);
                println!("  Display name: {}", output.display_name);
                println!("  Status: {}", output.status);
                println!("  Chunks: {}", output.chunk_count);
                println!(
                    "  Retries: {}/{}",
                    output.retry_count, output.max_retry_count
                );
                if let Some(err) = &output.error {
                    println!("  Error [{}]: {}", err.code, err.message);
                }
            }
            ExitCode::Success as i32
        }
        Err(e) => exit_for_error(&e, json),
    }
}
