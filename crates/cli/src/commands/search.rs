use clap::Args;
use common::ExitCode;
use config::Config;
use engine::retrieve;
use serde::Serialize;

use super::{exit_for_error, validate_retrieval_scope, CommandContext};
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
    let scope =
        match validate_retrieval_scope(args.flat, args.topic.as_deref(), args.concept.as_deref()) {
            Ok(scope) => scope,
            Err(e) => return exit_for_error(&e, json),
        };

    let ctx = match CommandContext::open(config, json) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    let db = &ctx.db;
    let provider = match ctx.provider() {
        Ok(p) => p,
        Err(e) => return exit_for_error(&e, json),
    };

    let mut retrieval_config = config.retrieval.clone();
    if let Some(use_hierarchy) = scope.hierarchy_override {
        retrieval_config.use_hierarchy = use_hierarchy;
    }

    match retrieve::search(
        db,
        provider,
        &retrieval_config,
        &config.rate_limit,
        &args.query,
        args.k,
        scope.topic_filter,
        scope.concept_filter,
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
                            .map(|h| {
                                let preview = h.preview();
                                SearchResultItem {
                                    chunk_id: h.chunk_id.to_string(),
                                    document_id: h.document_id.to_string(),
                                    display_name: h.display_name,
                                    section_id: h.section_id,
                                    chunk_index: h.chunk_index,
                                    page: h.page,
                                    offset_start: h.offset_start,
                                    offset_end: h.offset_end,
                                    score: h.score,
                                    preview,
                                    topic_name: h.topic_name,
                                    concept_name: h.concept_name,
                                    breadcrumb: h.breadcrumb,
                                }
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
                    println!("     {}", hit.preview());
                }
            }

            ExitCode::Success as i32
        }
        Err(e) => exit_for_error(&e, json),
    }
}
