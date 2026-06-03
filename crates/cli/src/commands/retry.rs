use clap::Args;
use common::ExitCode;
use config::Config;
use engine::ingest;
use serde::Serialize;

use super::{exit_for_error, CommandContext};
use crate::output::print_json;

#[derive(Args)]
pub struct RetryArgs {
    /// Document ID to retry
    pub document_id: String,
}

#[derive(Serialize)]
struct RetryOutput {
    document_id: String,
    display_name: String,
    status: String,
    retry_count: u32,
    max_retry_count: u32,
    next_retry_at: Option<String>,
}

pub fn execute(args: &RetryArgs, config: &Config, json: bool) -> i32 {
    let ctx = match CommandContext::open_db_only(config, json) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };

    match ingest::retry_document(&ctx.db, &args.document_id) {
        Ok(doc) => {
            let output = RetryOutput {
                document_id: doc.document_id,
                display_name: doc.display_name,
                status: doc.status.to_string(),
                retry_count: doc.retry_count,
                max_retry_count: doc.max_retry_count,
                next_retry_at: doc.next_retry_at.map(|dt| dt.to_rfc3339()),
            };
            if json {
                print_json(&output);
            } else {
                println!("✓ Retried: {}", output.document_id);
                println!("  Status: {}", output.status);
                println!("  Retries reset to: {}", output.retry_count);
            }
            ExitCode::Success as i32
        }
        Err(e) => exit_for_error(&e, json),
    }
}
