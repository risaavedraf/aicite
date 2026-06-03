use crate::EmbeddingProvider;
use common::CiteError;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

/// OpenAI-compatible embedding provider.
///
/// Works with any API that follows the OpenAI embeddings endpoint contract:
/// `POST /v1/embeddings` with `{"input": "...", "model": "..."}` and
/// `Authorization: Bearer <key>`.
#[derive(Debug)]
pub struct OpenAICompatibleProvider {
    client: Client,
    endpoint: String,
    model: String,
    provider_id: String,
}

impl OpenAICompatibleProvider {
    /// Create a new provider.
    ///
    /// - `endpoint`: API URL (must be HTTPS)
    /// - `model`: model ID (e.g. `"text-embedding-3-small"`)
    /// - `api_key`: API key for bearer authentication
    pub fn new(
        endpoint: &str,
        model: &str,
        api_key: &str,
        timeout_secs: u64,
    ) -> Result<Self, CiteError> {
        if api_key.is_empty() {
            return Err(CiteError::ConfigError {
                message: "API key must not be empty. Set the CITE_API_KEY environment variable or add api_key to config.".to_string(),
            });
        }

        if !endpoint.starts_with("https://") {
            return Err(CiteError::ConfigError {
                message: format!("Embedding endpoint must use HTTPS, got: {}", endpoint),
            });
        }

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&format!("Bearer {}", api_key))
                        .map_err(|e| CiteError::ConfigError {
                            message: format!("Invalid API key header value: {}", e),
                        })?,
                );
                headers
            })
            .build()
            .map_err(|e| CiteError::ConfigError {
                message: format!("Failed to build HTTP client: {}", e),
            })?;

        Ok(Self {
            client,
            endpoint: endpoint.to_string(),
            model: model.to_string(),
            provider_id: "openai-compatible".to_string(),
        })
    }
}

#[derive(Serialize)]
struct EmbeddingRequest<'a> {
    input: &'a str,
    model: &'a str,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

impl EmbeddingProvider for OpenAICompatibleProvider {
    fn embed(&self, text: &str) -> Result<Vec<f32>, CiteError> {
        let request = EmbeddingRequest {
            input: text,
            model: &self.model,
        };

        let response = self
            .client
            .post(&self.endpoint)
            .json(&request)
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    CiteError::EmbeddingProviderError {
                        message: format!("Request to embedding provider timed out: {}", e),
                    }
                } else {
                    CiteError::EmbeddingProviderError {
                        message: format!("Embedding request failed: {}", e),
                    }
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(CiteError::EmbeddingProviderError {
                message: format!("Embedding provider returned HTTP {}: {}", status, body),
            });
        }

        let parsed: EmbeddingResponse =
            response
                .json()
                .map_err(|e| CiteError::EmbeddingProviderError {
                    message: format!("Failed to parse embedding response: {}", e),
                })?;

        parsed
            .data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| CiteError::EmbeddingProviderError {
                message: "Embedding response contained no data".to_string(),
            })
    }

    fn model_id(&self) -> &str {
        &self.model
    }

    fn provider_id(&self) -> &str {
        &self.provider_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation_valid() {
        let result = OpenAICompatibleProvider::new(
            "https://api.openai.com/v1/embeddings",
            "text-embedding-3-small",
            "sk-test-key",
            30,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_provider_creation_rejects_http() {
        let result = OpenAICompatibleProvider::new(
            "http://api.openai.com/v1/embeddings",
            "text-embedding-3-small",
            "sk-test-key",
            30,
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            CiteError::ConfigError { message } => {
                assert!(message.contains("HTTPS"));
            }
            other => panic!("Expected ConfigError, got: {:?}", other),
        }
    }

    #[test]
    fn test_provider_model_id() {
        let provider = OpenAICompatibleProvider::new(
            "https://api.openai.com/v1/embeddings",
            "text-embedding-3-small",
            "sk-test-key",
            30,
        )
        .unwrap();
        assert_eq!(provider.model_id(), "text-embedding-3-small");
    }

    #[test]
    fn test_provider_provider_id() {
        let provider = OpenAICompatibleProvider::new(
            "https://api.openai.com/v1/embeddings",
            "text-embedding-3-small",
            "sk-test-key",
            30,
        )
        .unwrap();
        assert_eq!(provider.provider_id(), "openai-compatible");
    }

    #[test]
    fn test_provider_rejects_empty_key() {
        let result = OpenAICompatibleProvider::new(
            "https://api.openai.com/v1/embeddings",
            "text-embedding-3-small",
            "",
            30,
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            CiteError::ConfigError { message } => {
                assert!(
                    message.contains("must not be empty"),
                    "Unexpected message: {}",
                    message
                );
            }
            other => panic!("Expected ConfigError, got: {:?}", other),
        }
    }

    #[test]
    fn test_embed_invalid_endpoint_returns_error() {
        // Use a non-existent HTTPS endpoint to test error handling
        let provider = OpenAICompatibleProvider::new(
            "https://localhost:1/nonexistent",
            "test-model",
            "test-key",
            30,
        )
        .unwrap();

        let result = provider.embed("hello world");
        assert!(result.is_err());
        match result.unwrap_err() {
            CiteError::EmbeddingProviderError { message } => {
                assert!(
                    message.contains("failed") || message.contains("timed out"),
                    "Unexpected message: {}",
                    message
                );
            }
            other => panic!("Expected EmbeddingProviderError, got: {:?}", other),
        }
    }
}
