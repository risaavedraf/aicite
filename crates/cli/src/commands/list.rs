use clap::Args;
use common::ExitCode;
use config::Config;
use engine::ingest;
use serde::Serialize;
use storage::tags::TagFilter;

use super::{exit_for_error, CommandContext};
use crate::output::print_json;

#[derive(Args)]
pub struct ListArgs {
    /// Filter documents by local tag. Multiple filters use AND semantics.
    #[arg(long = "tag")]
    tags: Vec<String>,
}

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

pub fn execute(args: &ListArgs, config: &Config, json: bool) -> i32 {
    let ctx = match CommandContext::open_db_only(config, json) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };

    let tag_filters = match parse_tag_filters(&args.tags) {
        Ok(filters) => filters,
        Err(e) => return exit_for_error(&e, json),
    };

    let docs_result = if tag_filters.is_empty() {
        ingest::list_documents(&ctx.db)
    } else {
        ctx.db.list_documents_by_tags(&tag_filters)
    };

    match docs_result {
        Ok(docs) => {
            let summaries: Vec<DocumentSummary> = docs
                .iter()
                .map(|d| DocumentSummary {
                    document_id: d.document_id.to_string(),
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

fn parse_tag_filters(inputs: &[String]) -> Result<Vec<TagFilter>, common::CiteError> {
    inputs.iter().map(|input| TagFilter::parse(input)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_tag_filter_parser_accepts_key_value_and_key_only() {
        let filters = parse_tag_filters(&["type:rfc".to_string(), "status".to_string()]).unwrap();

        assert_eq!(
            filters,
            vec![
                TagFilter {
                    key: "type".to_string(),
                    value: Some("rfc".to_string()),
                },
                TagFilter {
                    key: "status".to_string(),
                    value: None,
                },
            ]
        );
    }

    #[test]
    fn list_tag_filter_parser_rejects_malformed_filters() {
        assert!(parse_tag_filters(&["status:".to_string()]).is_err());
        assert!(parse_tag_filters(&[":changed".to_string()]).is_err());
        assert!(parse_tag_filters(&[" status".to_string()]).is_err());
    }
}
