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

fn handle_command_error(e: &common::CiteError, json: bool) {
    if json {
        crate::output::print_json(&e.to_json_response());
    } else {
        eprintln!("Error: {e}");
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
