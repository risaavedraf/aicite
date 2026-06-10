use crate::{BatchStrategy, EmbeddingProvider};
use common::CiteError;

/// Ollama embedding provider.
///
/// Connects to a local Ollama server (default `http://localhost:11434`).
/// No API key required — Ollama runs locally.
///
/// Full implementation in PR8. This is a minimal placeholder that
/// satisfies the PR7 factory refactor contract:
/// - `provider_id()` returns `"ollama"`
/// - `batch_strategy()` returns `BatchStrategy::Native`
/// - No API key is required for construction
#[derive(Debug)]
pub struct OllamaProvider {
    model: String,
}

impl OllamaProvider {
    /// Create a new Ollama embedding provider.
    ///
    /// No API key required — Ollama is a local inference server.
    pub fn new(model: &str) -> Result<Self, CiteError> {
        Ok(Self {
            model: model.to_string(),
        })
    }
}

impl EmbeddingProvider for OllamaProvider {
    fn embed(&self, _text: &str) -> Result<Vec<f32>, CiteError> {
        // Placeholder — full implementation in PR8.
        Err(CiteError::EmbeddingProviderError {
            message: "Ollama provider not yet fully implemented. Full support coming in PR8."
                .to_string(),
        })
    }

    fn batch_strategy(&self) -> BatchStrategy {
        BatchStrategy::Native
    }

    fn model_id(&self) -> &str {
        &self.model
    }

    fn provider_id(&self) -> &str {
        "ollama"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_provider_creation() {
        let result = OllamaProvider::new("nomic-embed-text");
        assert!(result.is_ok());
        let provider = result.unwrap();
        assert_eq!(provider.model_id(), "nomic-embed-text");
        assert_eq!(provider.provider_id(), "ollama");
    }

    #[test]
    fn test_ollama_provider_batch_strategy_is_native() {
        let provider = OllamaProvider::new("nomic-embed-text").unwrap();
        assert_eq!(provider.batch_strategy(), BatchStrategy::Native);
    }

    #[test]
    fn test_ollama_provider_embed_returns_placeholder_error() {
        let provider = OllamaProvider::new("nomic-embed-text").unwrap();
        let result = provider.embed("test text");
        assert!(result.is_err());
    }
}
