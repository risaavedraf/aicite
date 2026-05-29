use common::ExitCode;
use config::Config;
use serde::Serialize;

use super::CommandContext;
use crate::output::print_json;

#[derive(Serialize)]
struct RefreshOutput {
    status: String,
    snapshot_id: String,
    document_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    previous_snapshot_id: Option<String>,
}

pub fn execute(config: &Config, json: bool) -> i32 {
    let ctx = match CommandContext::open_db_only(config, json) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };

    match engine::refresh::refresh_corpus(&ctx.db) {
        Ok(result) => {
            let output = RefreshOutput {
                status: "refreshed".to_string(),
                snapshot_id: result.snapshot_id,
                document_count: result.document_count,
                previous_snapshot_id: result.previous_snapshot_id,
            };
            if json {
                print_json(&output);
            } else {
                println!("✓ Corpus refreshed");
                println!("  Snapshot ID: {}", output.snapshot_id);
                println!("  Documents: {}", output.document_count);
                if let Some(prev) = &output.previous_snapshot_id {
                    println!("  Previous snapshot: {prev}");
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
