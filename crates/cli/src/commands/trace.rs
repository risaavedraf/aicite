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
    let ctx = match CommandContext::open_db_only(config, json) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };

    match context::get_trace(&ctx.db, &args.trace_id) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use common::types::TraceHeaderInput;
    use std::path::PathBuf;
    use storage::Database;

    fn unique_temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "cite-{name}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn trace_command_uses_db_only_context() {
        let data_dir = unique_temp_dir("trace-db-only");
        std::fs::create_dir_all(&data_dir).unwrap();
        let db = Database::open(&data_dir).unwrap();
        db.persist_trace_with_citations(
            &TraceHeaderInput {
                trace_id: "trace-offline".into(),
                query_id: Some("qry-offline".into()),
                context_pack_id: Some("ctx-offline".into()),
                request_type: "context".into(),
                document_ids: Some("doc-1".into()),
                citation_ids: None,
                top_k: Some(3),
                evidence_floor: Some(0.5),
                confidence_threshold: Some(0.7),
                ranking_method: Some("vector_cosine_v1".into()),
                embedding_model_registry_id: Some("historic-model".into()),
                provider: Some("historic-provider".into()),
                latency_ms: Some(42),
            },
            &[],
        )
        .unwrap();
        drop(db);

        let mut config = Config::load_from(None).unwrap();
        config.paths.data_dir = Some(data_dir.clone());
        config.embedding.provider = "openai-compatible".into();
        config.ingest.embedding_endpoint = Some("http://invalid-provider-endpoint".into());

        let args = TraceArgs {
            trace_id: "trace-offline".into(),
        };

        assert_eq!(execute(&args, &config, true), ExitCode::Success as i32);

        let _ = std::fs::remove_dir_all(data_dir);
    }
}
