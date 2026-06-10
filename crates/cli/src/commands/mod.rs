pub mod check_docs;
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
pub mod tag;
pub mod trace;
pub mod workspace;

use config::Config;
use providers::gemini::GeminiProvider;
use providers::ollama::OllamaProvider;
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

/// Parse shared chunk-local tag filters for search, retrieve, and context.
pub fn parse_retrieval_tag_filters(
    tags: &[String],
) -> Result<Vec<storage::tags::TagFilter>, common::CiteError> {
    tags.iter()
        .map(|tag| storage::tags::TagFilter::parse(tag))
        .collect()
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
/// - `gemini`: Google Gemini API (requires API key)
/// - `ollama`: Local Ollama inference server (no API key required)
/// - `openai-compatible` (default): Any OpenAI-compatible API (requires API key)
pub fn create_provider(config: &Config) -> Result<Box<dyn EmbeddingProvider>, common::CiteError> {
    match config.embedding.provider.as_str() {
        "gemini" => {
            let api_key = resolve_api_key(config).ok_or_else(|| common::CiteError::ConfigError {
                message: "No API key configured for Gemini. Set CITE_EMBEDDING_API_KEY or run `cite setup`."
                    .to_string(),
            })?;
            let provider = GeminiProvider::new(
                &config.embedding.model,
                &api_key,
                config.ingest.embedding_timeout_secs,
            )?;
            Ok(Box::new(provider))
        }
        "ollama" => {
            // Ollama is a local provider — no API key required.
            // Full implementation in PR8.
            let provider = OllamaProvider::new(&config.embedding.model)?;
            Ok(Box::new(provider))
        }
        "openai-compatible" => {
            let api_key = resolve_api_key(config).ok_or_else(|| common::CiteError::ConfigError {
                message: "No API key configured. Set the CITE_API_KEY environment variable or run `cite setup`."
                    .to_string(),
            })?;
            let endpoint = config
                .embedding_endpoint()
                .unwrap_or("https://api.openai.com/v1/embeddings");

            let provider = OpenAICompatibleProvider::new(
                endpoint,
                &config.embedding.model,
                &api_key,
                config.ingest.embedding_timeout_secs,
            )?;
            Ok(Box::new(provider))
        }
        unknown => Err(common::CiteError::ConfigError {
            message: format!(
                "Unknown embedding provider '{}'. Supported providers: gemini, ollama, openai-compatible.",
                unknown
            ),
        }),
    }
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
    fn parse_retrieval_tag_filters_accepts_exact_and_key_only_filters() {
        let filters = parse_retrieval_tag_filters(&["type:rfc".into(), "status".into()]).unwrap();
        assert_eq!(filters.len(), 2);
        assert_eq!(filters[0].key, "type");
        assert_eq!(filters[0].value.as_deref(), Some("rfc"));
        assert_eq!(filters[1].key, "status");
        assert_eq!(filters[1].value, None);
    }

    #[test]
    fn parse_retrieval_tag_filters_rejects_malformed_filters() {
        let err = parse_retrieval_tag_filters(&["type:".into()]).unwrap_err();
        assert_eq!(err.exit_code(), ExitCode::Validation);
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

    // --- PR7 factory refactor: provider-before-key tests ---
    //
    // PR7 changes `create_provider` so provider selection happens BEFORE
    // API key validation, and adds an `ollama` branch that does not need
    // a key. These tests document the new contract:
    //
    //   * `ollama` provider should succeed without any API key (RED now:
    //     the factory returns a "No API key configured" ConfigError before
    //     the provider match runs).
    //   * `gemini` and the default `openai-compatible` provider still
    //     require a key (these tests document the existing contract).
    //   * An unknown provider should return a meaningful error referencing
    //     the provider name (RED now: the factory returns a misleading
    //     "No API key configured" error before the match runs).
    //
    // The factory function is NOT refactored in this PR; these tests are
    // intentionally failing for the ollama / unknown-provider cases.

    use std::sync::Mutex;

    /// Serializes tests that mutate process-global env vars so they don't
    /// race each other when cargo runs tests in parallel.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    /// Restores a removed env var to its previous value on drop.
    struct EnvVarGuard {
        name: &'static str,
        original: Option<String>,
    }

    impl EnvVarGuard {
        fn remove(name: &'static str) -> Self {
            let original = std::env::var(name).ok();
            std::env::remove_var(name);
            Self { name, original }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(value) => std::env::set_var(self.name, value),
                None => std::env::remove_var(self.name),
            }
        }
    }

    /// Remove every env var that could supply an API key or override the
    /// embedding provider/model/endpoint, so the resulting Config reflects
    /// the on-disk defaults plus our explicit per-test overrides only.
    fn clear_factory_env_vars() -> [EnvVarGuard; 7] {
        [
            EnvVarGuard::remove("CITE_EMBEDDING_API_KEY"),
            EnvVarGuard::remove("CITE_API_KEY"),
            EnvVarGuard::remove("GEMINI_API_KEY"),
            EnvVarGuard::remove("OPENAI_API_KEY"),
            EnvVarGuard::remove("CITE_EMBEDDING_PROVIDER"),
            EnvVarGuard::remove("CITE_EMBEDDING_MODEL"),
            EnvVarGuard::remove("CITE_EMBEDDING_ENDPOINT"),
        ]
    }

    /// Path that is guaranteed not to exist, so `FileConfig::load` returns
    /// `None` and the merged config reflects defaults only.
    fn isolated_config_path() -> &'static std::path::Path {
        std::path::Path::new("/nonexistent/aiharness-pr7-factory-test.toml")
    }

    /// PR7: with `provider = "ollama"` and no API key anywhere, the factory
    /// should match the ollama branch and succeed without asking for a key.
    /// This is RED today: the factory calls `resolve_api_key().ok_or(...)`
    /// before the provider match runs, so it returns
    /// `ConfigError("No API key configured...")` instead of creating the
    /// provider.
    #[test]
    fn ollama_provider_creates_without_api_key() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _env_guards = clear_factory_env_vars();

        let mut config = Config::load_from(Some(isolated_config_path()))
            .expect("isolated config load should succeed");
        config.embedding.provider = "ollama".to_string();
        // config.embedding.api_key remains None from defaults.

        let result = create_provider(&config);
        assert!(
            result.is_ok(),
            "ollama provider should be creatable without an API key, got error: {:?}",
            result.err()
        );

        let provider = result.unwrap();
        assert_eq!(
            provider.provider_id(),
            "ollama",
            "factory should have matched the ollama branch"
        );
    }

    /// PR7: provider selection should happen before key validation. With
    /// `provider = "ollama"` and no key, the factory must not return a
    /// ConfigError complaining about a missing API key. The weaker form of
    /// this contract: the error path, if any, must mention the provider —
    /// not the key.
    #[test]
    fn ollama_provider_does_not_fail_with_missing_api_key_error() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _env_guards = clear_factory_env_vars();

        let mut config = Config::load_from(Some(isolated_config_path()))
            .expect("isolated config load should succeed");
        config.embedding.provider = "ollama".to_string();

        let result = create_provider(&config);
        if let Err(e) = &result {
            let msg = format!("{:?}", e).to_lowercase();
            assert!(
                !msg.contains("api key") && !msg.contains("cite_api_key"),
                "factory should match provider before validating key, got key error: {:?}",
                e
            );
        }
        // Ok(...) is fine — the strong contract is covered by
        // `ollama_provider_creates_without_api_key`. This test pins the
        // weaker "must not mention api key" contract for documentation.
    }

    /// PR7 contract (preserved): `gemini` requires an API key. This
    /// documents existing behavior — the factory returns a ConfigError when
    /// no key is configured for the gemini branch.
    #[test]
    fn gemini_provider_requires_api_key() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _env_guards = clear_factory_env_vars();

        let mut config = Config::load_from(Some(isolated_config_path()))
            .expect("isolated config load should succeed");
        config.embedding.provider = "gemini".to_string();

        let result = create_provider(&config);
        match result {
            Err(common::CiteError::ConfigError { message }) => {
                let lower = message.to_lowercase();
                assert!(
                    lower.contains("api key") || lower.contains("must not be empty"),
                    "expected config error about API key, got: {}",
                    message
                );
            }
            Err(other) => panic!("Expected ConfigError, got: {:?}", other),
            Ok(_) => panic!("gemini without a key should fail, got Ok"),
        }
    }

    /// PR7 contract (preserved): the default `openai-compatible` provider
    /// requires an API key. This documents existing behavior — the
    /// factory returns a ConfigError when no key is configured.
    #[test]
    fn openai_compatible_provider_requires_api_key() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _env_guards = clear_factory_env_vars();

        let config = Config::load_from(Some(isolated_config_path()))
            .expect("isolated config load should succeed");
        assert_eq!(
            config.embedding.provider, "openai-compatible",
            "default provider should be openai-compatible"
        );

        let result = create_provider(&config);
        match result {
            Err(common::CiteError::ConfigError { message }) => {
                let lower = message.to_lowercase();
                assert!(
                    lower.contains("api key") || lower.contains("must not be empty"),
                    "expected config error about API key, got: {}",
                    message
                );
            }
            Err(other) => panic!("Expected ConfigError, got: {:?}", other),
            Ok(_) => panic!("openai-compatible without a key should fail, got Ok"),
        }
    }

    /// PR7 contract: an unrecognized provider name should produce a
    /// meaningful error that references the provider, not a misleading
    /// "no API key configured" error. This is RED today: the factory
    /// checks the key first and never reaches the provider match, so the
    /// error is about the missing key instead of the unknown provider.
    #[test]
    fn unknown_provider_returns_meaningful_error() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _env_guards = clear_factory_env_vars();

        let mut config = Config::load_from(Some(isolated_config_path()))
            .expect("isolated config load should succeed");
        config.embedding.provider = "nonexistent-provider".to_string();

        let err = match create_provider(&config) {
            Ok(_) => panic!("unknown provider should fail, got Ok"),
            Err(e) => e,
        };
        let msg = format!("{:?}", err);
        let lower = msg.to_lowercase();

        assert!(
            msg.contains("nonexistent-provider")
                || lower.contains("unknown provider")
                || lower.contains("unsupported provider")
                || lower.contains("invalid provider")
                || lower.contains("unrecognized provider"),
            "expected error to mention the unknown provider name, got: {}",
            msg
        );
    }
}
