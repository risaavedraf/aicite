use crate::EmbeddingProvider;
use common::HarnessError;
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
    pub fn new(endpoint: &str, model: &str, api_key: &str) -> Result<Self, HarnessError> {
        if !endpoint.starts_with("https://") {
            return Err(HarnessError::ConfigError {
                message: format!("Embedding endpoint must use HTTPS, got: {}", endpoint),
            });
        }

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&format!("Bearer {}", api_key))
                        .map_err(|e| HarnessError::ConfigError {
                            message: format!("Invalid API key header value: {}", e),
                        })?,
                );
                headers
            })
            .build()
            .map_err(|e| HarnessError::ConfigError {
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
    fn embed(&self, text: &str) -> Result<Vec<f32>, HarnessError> {
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
                    HarnessError::EmbeddingProviderError {
                        message: format!("Request to embedding provider timed out: {}", e),
                    }
                } else {
                    HarnessError::EmbeddingProviderError {
                        message: format!("Embedding request failed: {}", e),
                    }
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(HarnessError::EmbeddingProviderError {
                message: format!("Embedding provider returned HTTP {}: {}", status, body),
            });
        }

        let parsed: EmbeddingResponse =
            response
                .json()
                .map_err(|e| HarnessError::EmbeddingProviderError {
                    message: format!("Failed to parse embedding response: {}", e),
                })?;

        parsed
            .data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| HarnessError::EmbeddingProviderError {
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
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_provider_creation_rejects_http() {
        let result = OpenAICompatibleProvider::new(
            "http://api.openai.com/v1/embeddings",
            "text-embedding-3-small",
            "sk-test-key",
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            HarnessError::ConfigError { message } => {
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
        )
        .unwrap();
        assert_eq!(provider.provider_id(), "openai-compatible");
    }

    #[test]
    fn test_embed_invalid_endpoint_returns_error() {
        // Use a non-existent HTTPS endpoint to test error handling
        let provider = OpenAICompatibleProvider::new(
            "https://localhost:1/nonexistent",
            "test-model",
            "test-key",
        )
        .unwrap();

        let result = provider.embed("hello world");
        assert!(result.is_err());
        match result.unwrap_err() {
            HarnessError::EmbeddingProviderError { message } => {
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
