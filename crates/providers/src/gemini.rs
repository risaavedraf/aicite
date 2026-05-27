use crate::EmbeddingProvider;
use common::HarnessError;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

/// Google Gemini embedding provider.
///
/// Uses the Gemini API `embedContent` endpoint:
/// `POST https://generativelanguage.googleapis.com/v1beta/models/{model}:embedContent`
///
/// Authentication via `x-goog-api-key` header.
/// Free tier available at https://aistudio.google.com/apikey
#[derive(Debug)]
pub struct GeminiProvider {
    client: Client,
    model: String,
    endpoint: String,
}

impl GeminiProvider {
    /// Create a new Gemini embedding provider.
    ///
    /// - `model`: model ID (e.g. `"gemini-embedding-001"`, `"gemini-embedding-2"`)
    /// - `api_key`: Google AI API key (get one free at https://aistudio.google.com/apikey)
    pub fn new(model: &str, api_key: &str) -> Result<Self, HarnessError> {
        let endpoint = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:embedContent",
            model
        );

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    "x-goog-api-key",
                    reqwest::header::HeaderValue::from_str(api_key).map_err(|e| {
                        HarnessError::ConfigError {
                            message: format!("Invalid API key header value: {}", e),
                        }
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
            model: model.to_string(),
            endpoint,
        })
    }
}

// --- Request/Response types for Gemini API ---

#[derive(Serialize)]
struct GeminiRequest<'a> {
    model: String,
    content: GeminiContent<'a>,
}

#[derive(Serialize)]
struct GeminiContent<'a> {
    parts: Vec<GeminiPart<'a>>,
}

#[derive(Serialize)]
struct GeminiPart<'a> {
    text: &'a str,
}

#[derive(Deserialize)]
struct GeminiResponse {
    embeddings: Vec<GeminiEmbedding>,
}

#[derive(Deserialize)]
struct GeminiEmbedding {
    values: Vec<f32>,
}

impl EmbeddingProvider for GeminiProvider {
    fn embed(&self, text: &str) -> Result<Vec<f32>, HarnessError> {
        let request = GeminiRequest {
            model: format!("models/{}", self.model),
            content: GeminiContent {
                parts: vec![GeminiPart { text }],
            },
        };

        let response = self
            .client
            .post(&self.endpoint)
            .json(&request)
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    HarnessError::EmbeddingProviderError {
                        message: format!("Gemini embedding request timed out: {}", e),
                    }
                } else {
                    HarnessError::EmbeddingProviderError {
                        message: format!("Gemini embedding request failed: {}", e),
                    }
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(HarnessError::EmbeddingProviderError {
                message: format!("Gemini API returned HTTP {}: {}", status, body),
            });
        }

        let parsed: GeminiResponse =
            response
                .json()
                .map_err(|e| HarnessError::EmbeddingProviderError {
                    message: format!("Failed to parse Gemini embedding response: {}", e),
                })?;

        parsed
            .embeddings
            .into_iter()
            .next()
            .map(|e| e.values)
            .ok_or_else(|| HarnessError::EmbeddingProviderError {
                message: "Gemini embedding response contained no embeddings".to_string(),
            })
    }

    fn model_id(&self) -> &str {
        &self.model
    }

    fn provider_id(&self) -> &str {
        "gemini"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let result = GeminiProvider::new("gemini-embedding-001", "test-api-key");
        assert!(result.is_ok());
        let provider = result.unwrap();
        assert_eq!(provider.model_id(), "gemini-embedding-001");
        assert_eq!(provider.provider_id(), "gemini");
        assert!(provider.endpoint.contains("gemini-embedding-001"));
    }

    #[test]
    fn test_provider_endpoint_format() {
        let provider = GeminiProvider::new("gemini-embedding-2", "key").unwrap();
        assert_eq!(
            provider.endpoint,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-embedding-2:embedContent"
        );
    }

    #[test]
    fn test_embed_invalid_key_returns_error() {
        // Use an obviously invalid key to test error handling
        let provider = GeminiProvider::new("gemini-embedding-001", "invalid-key").unwrap();
        let result = provider.embed("hello world");
        assert!(result.is_err());
        match result.unwrap_err() {
            HarnessError::EmbeddingProviderError { message } => {
                assert!(
                    message.contains("HTTP")
                        || message.contains("failed")
                        || message.contains("timed out"),
                    "Unexpected message: {}",
                    message
                );
            }
            other => panic!("Expected EmbeddingProviderError, got: {:?}", other),
        }
    }
}
