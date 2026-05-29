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

/// Resolve the data directory from config or platform default.
pub fn resolve_data_dir(config: &Config) -> PathBuf {
    config.paths.data_dir.clone().unwrap_or_else(|| {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("cite")
    })
}

/// Create an embedding provider based on config.
///
/// Supported providers:
/// - `gemini`: Google Gemini API
/// - `openai-compatible` (default): Any OpenAI-compatible API
pub fn create_provider(config: &Config) -> Result<Box<dyn EmbeddingProvider>, common::CiteError> {
    // Precedence: CITE_EMBEDDING_API_KEY > GEMINI_API_KEY > OPENAI_API_KEY > config file api_key
    let api_key = std::env::var("CITE_EMBEDDING_API_KEY")
        .or_else(|_| std::env::var("GEMINI_API_KEY"))
        .or_else(|_| std::env::var("OPENAI_API_KEY"))
        .or_else(|_| {
            config
                .embedding
                .api_key
                .clone()
                .ok_or(std::env::VarError::NotPresent)
        })
        .unwrap_or_default();

    match config.embedding.provider.as_str() {
        "gemini" => {
            let provider = GeminiProvider::new(&config.embedding.model, &api_key)?;
            Ok(Box::new(provider))
        }
        _ => {
            let endpoint = config
                .ingest
                .embedding_endpoint
                .as_deref()
                .unwrap_or("https://api.openai.com/v1/embeddings");

            let provider =
                OpenAICompatibleProvider::new(endpoint, &config.embedding.model, &api_key)?;
            Ok(Box::new(provider))
        }
    }
}
