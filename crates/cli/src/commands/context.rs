use super::{exit_for_error, validate_retrieval_scope, CommandContext};
use crate::output::{print_json, to_compact_context, truncate_to};
use clap::Args;
use common::ExitCode;
use config::Config;
use engine::context;

#[derive(Args)]
pub struct ContextArgs {
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

pub fn execute(args: &ContextArgs, config: &Config, json: bool) -> i32 {
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

    match context::build_context(
        db,
        provider,
        &retrieval_config,
        &config.rate_limit,
        &args.query,
        args.k,
        scope.topic_filter,
        scope.concept_filter,
    ) {
        Ok(response) => {
            if json {
                if args.full {
                    print_json(&response);
                } else {
                    print_json(&to_compact_context(&response));
                }
            } else {
                println!("Context pack ({}):", response.result_kind);
                println!("  Citations: {}", response.citations.len());
                println!("  Trace ID: {}", response.trace_id);
                println!();
                for (idx, citation) in response.citations.iter().enumerate() {
                    println!(
                        "  {}. [{:.4}] {} [{}]",
                        idx + 1,
                        citation.score.unwrap_or(0.0),
                        citation.display_name,
                        citation.citation_id
                    );
                    println!("     {}", truncate_to(&citation.text, 160));
                }
                println!();
                println!("{}", response.metadata.disclaimer);
                if let Some(caution) = &response.metadata.caution {
                    println!("⚠ {}", caution);
                }
            }

            ExitCode::Success as i32
        }
        Err(e) => exit_for_error(&e, json),
    }
}
