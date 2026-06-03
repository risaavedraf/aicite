pub mod context;
pub mod evaluate;
pub mod get;
pub mod health;
pub mod ingest;
pub mod list;
pub mod read;
pub mod refresh;
pub mod retrieve;
pub mod retry;
pub mod search;
pub mod setup;
pub mod trace;

use config::Config;
use providers::gemini::GeminiProvider;
use providers::openai::OpenAICompatibleProvider;
use providers::EmbeddingProvider;
use std::path::PathBuf;

/// Shared context for CLI commands that need database and/or provider access.
pub struct CommandContext {
    pub db: storage::Database,
    pub provider: Option<Box<dyn EmbeddingProvider>>,
}

impl CommandContext {
    /// Open database and create an embedding provider from config.
    ///
    /// Use this for commands that need vector search (search, retrieve, context, ingest).
    pub fn open(config: &Config, json: bool) -> Result<Self, i32> {
        let data_dir = resolve_data_dir(config);
        let db = storage::Database::open(&data_dir).map_err(|e| {
            handle_command_error(&e, json);
            e.exit_code() as i32
        })?;
        let provider = create_provider(config).map_err(|e| {
            handle_command_error(&e, json);
            e.exit_code() as i32
        })?;
        Ok(Self {
            db,
            provider: Some(provider),
        })
    }

    /// Open database only, without creating a provider.
    ///
    /// Use this for commands that only need database access (get, list, retry, refresh, read, trace).
    pub fn open_db_only(config: &Config, json: bool) -> Result<Self, i32> {
        let data_dir = resolve_data_dir(config);
        let db = storage::Database::open(&data_dir).map_err(|e| {
            handle_command_error(&e, json);
            e.exit_code() as i32
        })?;
        Ok(Self { db, provider: None })
    }

    /// Get the embedding provider, returning an error if none was configured.
    pub fn provider(&self) -> Result<&dyn EmbeddingProvider, common::CiteError> {
        self.provider
            .as_deref()
            .ok_or_else(|| common::CiteError::ConfigError {
                message: "No embedding provider configured".to_string(),
            })
    }
}

/// Render a command error using the canonical CLI JSON/text shape and return its exit code.
pub fn exit_for_error(e: &common::CiteError, json: bool) -> i32 {
    handle_command_error(e, json);
    e.exit_code() as i32
}

fn handle_command_error(e: &common::CiteError, json: bool) {
    if json {
        crate::output::print_json(&e.to_json_response());
    } else {
        eprintln!("Error: {e}");
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RetrievalScopeArgs<'a> {
    pub hierarchy_override: Option<bool>,
    pub topic_filter: Option<&'a str>,
    pub concept_filter: Option<&'a str>,
}

/// Validate shared retrieval-scope flags for search, retrieve, and context commands.
pub fn validate_retrieval_scope<'a>(
    flat: bool,
    topic: Option<&'a str>,
    concept: Option<&'a str>,
) -> Result<RetrievalScopeArgs<'a>, common::CiteError> {
    if flat && (topic.is_some() || concept.is_some()) {
        return Err(common::CiteError::InvalidParameter {
            message: "--flat cannot be combined with --topic or --concept.".to_string(),
        });
    }

    if topic.is_some() && concept.is_some() {
        return Err(common::CiteError::InvalidParameter {
            message: "--topic and --concept cannot be used together.".to_string(),
        });
    }

    Ok(RetrievalScopeArgs {
        hierarchy_override: flat.then_some(false),
        topic_filter: topic,
        concept_filter: concept,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::ExitCode;

    #[test]
    fn invalid_retrieval_scope_rejects_flat_with_topic() {
        let err = validate_retrieval_scope(true, Some("security"), None).unwrap_err();
        assert_eq!(err.exit_code(), ExitCode::Validation);
        assert_eq!(
            err,
            common::CiteError::InvalidParameter {
                message: "--flat cannot be combined with --topic or --concept.".to_string(),
            }
        );
    }

    #[test]
    fn invalid_retrieval_scope_rejects_flat_with_concept() {
        let err = validate_retrieval_scope(true, None, Some("auth")).unwrap_err();
        assert_eq!(err.exit_code(), ExitCode::Validation);
        assert_eq!(
            err,
            common::CiteError::InvalidParameter {
                message: "--flat cannot be combined with --topic or --concept.".to_string(),
            }
        );
    }

    #[test]
    fn invalid_retrieval_scope_rejects_topic_with_concept() {
        let err = validate_retrieval_scope(false, Some("security"), Some("auth")).unwrap_err();
        assert_eq!(err.exit_code(), ExitCode::Validation);
        assert_eq!(
            err,
            common::CiteError::InvalidParameter {
                message: "--topic and --concept cannot be used together.".to_string(),
            }
        );
    }

    #[test]
    fn valid_retrieval_scope_preserves_filters() {
        let scope = validate_retrieval_scope(false, Some("security"), None).unwrap();
        assert_eq!(scope.hierarchy_override, None);
        assert_eq!(scope.topic_filter, Some("security"));
        assert_eq!(scope.concept_filter, None);
    }

    #[test]
    fn valid_retrieval_scope_flat_disables_hierarchy() {
        let scope = validate_retrieval_scope(true, None, None).unwrap();
        assert_eq!(scope.hierarchy_override, Some(false));
        assert_eq!(scope.topic_filter, None);
        assert_eq!(scope.concept_filter, None);
    }

    #[test]
    fn exit_for_error_returns_error_exit_code() {
        let err = common::CiteError::InvalidParameter {
            message: "bad flags".to_string(),
        };
        assert_eq!(exit_for_error(&err, true), ExitCode::Validation as i32);
    }
}

/// Resolve the data directory from config or platform default.
pub fn resolve_data_dir(config: &Config) -> PathBuf {
    config.paths.data_dir.clone().unwrap_or_else(|| {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("cite")
    })
}

/// Resolve the API key from environment variables and config.
///
/// Precedence: CITE_EMBEDDING_API_KEY > GEMINI_API_KEY > OPENAI_API_KEY > config file api_key
pub fn resolve_api_key(config: &Config) -> Option<String> {
    std::env::var("CITE_EMBEDDING_API_KEY")
        .or_else(|_| std::env::var("GEMINI_API_KEY"))
        .or_else(|_| std::env::var("OPENAI_API_KEY"))
        .ok()
        .or_else(|| config.embedding.api_key.clone())
}

/// Create an embedding provider based on config.
///
/// Supported providers:
/// - `gemini`: Google Gemini API
/// - `openai-compatible` (default): Any OpenAI-compatible API
pub fn create_provider(config: &Config) -> Result<Box<dyn EmbeddingProvider>, common::CiteError> {
    let api_key = resolve_api_key(config).ok_or_else(|| common::CiteError::ConfigError {
        message:
            "No API key configured. Set the CITE_API_KEY environment variable or run `cite setup`."
                .to_string(),
    })?;

    match config.embedding.provider.as_str() {
        "gemini" => {
            let provider = GeminiProvider::new(
                &config.embedding.model,
                &api_key,
                config.ingest.embedding_timeout_secs,
            )?;
            Ok(Box::new(provider))
        }
        _ => {
            let endpoint = config
                .ingest
                .embedding_endpoint
                .as_deref()
                .unwrap_or("https://api.openai.com/v1/embeddings");

            let provider = OpenAICompatibleProvider::new(
                endpoint,
                &config.embedding.model,
                &api_key,
                config.ingest.embedding_timeout_secs,
            )?;
            Ok(Box::new(provider))
        }
    }
}
