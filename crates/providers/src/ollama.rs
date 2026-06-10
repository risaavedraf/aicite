use crate::{BatchStrategy, EmbeddingProvider};
use common::CiteError;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

/// Ollama embedding provider.
///
/// Connects to a local Ollama server (default `http://localhost:11434`).
/// No API key required — Ollama runs locally.
///
/// Supports native batch via Ollama's `/api/embed` endpoint
/// which accepts an array of input texts.
#[derive(Debug)]
pub struct OllamaProvider {
    client: Client,
    model: String,
    endpoint: String,
    dimensions: usize,
}

impl OllamaProvider {
    /// Create a new Ollama embedding provider.
    ///
    /// - `model`: Ollama model name (e.g. "nomic-embed-text", "qwen3-embedding:4b")
    /// - `endpoint`: Ollama server URL (e.g. "http://localhost:11434")
    /// - `dimensions`: embedding dimensions (used for validation/info)
    pub fn new(model: &str, endpoint: &str, dimensions: usize) -> Result<Self, CiteError> {
        Ok(Self {
            client: Client::new(),
            model: model.to_string(),
            endpoint: endpoint.to_string(),
            dimensions,
        })
    }

    fn embed_url(&self) -> String {
        format!("{}/api/embed", self.endpoint)
    }

    /// Return the configured embedding dimensions.
    pub fn dimensions(&self) -> usize {
        self.dimensions
    }
}

#[derive(Serialize)]
struct OllamaEmbedRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Deserialize)]
struct OllamaEmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

impl OllamaProvider {
    fn request_embeddings(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, CiteError> {
        let request = OllamaEmbedRequest {
            model: self.model.clone(),
            input: texts.iter().map(|text| (*text).to_string()).collect(),
        };

        let response = self
            .client
            .post(self.embed_url())
            .json(&request)
            .send()
            .map_err(|_| CiteError::EmbeddingProviderError {
                message: format!(
                    "Cannot connect to Ollama at {}. Is Ollama running?",
                    self.endpoint
                ),
            })?;

        let status = response.status();
        let body = response
            .text()
            .map_err(|e| CiteError::EmbeddingProviderError {
                message: format!("Unexpected Ollama response: {}", e),
            })?;

        if !status.is_success() {
            if status == reqwest::StatusCode::NOT_FOUND
                || body.to_lowercase().contains("not found")
            {
                return Err(CiteError::EmbeddingProviderError {
                    message: format!(
                        "Model '{}' not found in Ollama. Pull it with: ollama pull {}",
                        self.model, self.model
                    ),
                });
            }

            return Err(CiteError::EmbeddingProviderError {
                message: format!("Ollama HTTP error: {}", status),
            });
        }

        let parsed: OllamaEmbedResponse =
            serde_json::from_str(&body).map_err(|e| CiteError::EmbeddingProviderError {
                message: format!("Unexpected Ollama response: {}", e),
            })?;

        Ok(parsed.embeddings)
    }
}

impl EmbeddingProvider for OllamaProvider {
    fn embed(&self, text: &str) -> Result<Vec<f32>, CiteError> {
        self.request_embeddings(&[text])?
            .into_iter()
            .next()
            .ok_or_else(|| CiteError::EmbeddingProviderError {
                message: "Unexpected Ollama response: missing embedding".to_string(),
            })
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, CiteError> {
        self.request_embeddings(texts)
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
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    /// Read timeout for mock servers — prevents hanging.
    const MOCK_TIMEOUT: Duration = Duration::from_secs(2);

    /// Start a mock Ollama server on a random port.
    /// Returns (port, handle). The server handles one request then shuts down.
    fn mock_ollama_server(
        response_body: String,
        expected_path: &str,
    ) -> (u16, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let path = expected_path.to_string();
        let handle = thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                stream.set_read_timeout(Some(MOCK_TIMEOUT)).ok();
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                let request = String::from_utf8_lossy(&buf);
                assert!(
                    request.contains(&path),
                    "Expected request to {}, got: {}",
                    path,
                    request.lines().next().unwrap_or("")
                );
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        (port, handle)
    }

    /// Start a mock server that captures the request body.
    fn mock_ollama_server_capture(response_body: String) -> (u16, thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = thread::spawn(move || {
            let mut request_body = String::new();
            if let Ok((mut stream, _)) = listener.accept() {
                stream.set_read_timeout(Some(MOCK_TIMEOUT)).ok();
                let mut buf = [0u8; 8192];
                let n = stream.read(&mut buf).unwrap_or(0);
                let request = String::from_utf8_lossy(&buf[..n]);

                if let Some(body_start) = request.find("\r\n\r\n") {
                    request_body = request[body_start + 4..].to_string();
                    request_body = request_body.trim_end_matches('\0').to_string();
                }

                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );
                let _ = stream.write_all(response.as_bytes());
            }
            request_body
        });
        (port, handle)
    }

    // ── Construction and error handling tests ───────────────────────────

    #[test]
    fn test_ollama_provider_creation() {
        let result = OllamaProvider::new("nomic-embed-text", "http://localhost:11434", 768);
        assert!(result.is_ok());
        let provider = result.unwrap();
        assert_eq!(provider.model_id(), "nomic-embed-text");
        assert_eq!(provider.provider_id(), "ollama");
    }

    #[test]
    fn test_ollama_provider_batch_strategy_is_native() {
        let provider =
            OllamaProvider::new("nomic-embed-text", "http://localhost:11434", 768).unwrap();
        assert_eq!(provider.batch_strategy(), BatchStrategy::Native);
    }

    #[test]
    fn test_ollama_provider_endpoint_and_dimensions() {
        let provider =
            OllamaProvider::new("qwen3-embedding:4b", "http://localhost:9999", 1024).unwrap();
        assert_eq!(provider.model_id(), "qwen3-embedding:4b");
        assert_eq!(provider.dimensions, 1024);
        assert_eq!(provider.endpoint, "http://localhost:9999");
    }

    #[test]
    fn test_ollama_embed_rejects_invalid_json_response() {
        let (port, handle) = mock_ollama_server("not json".to_string(), "/api/embed");
        let endpoint = format!("http://127.0.0.1:{}", port);
        let provider = OllamaProvider::new("test-model", &endpoint, 768).unwrap();

        let result = provider.embed("test text");
        let _ = handle.join();

        assert!(result.is_err(), "invalid JSON should return error");
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.contains("Unexpected Ollama response"),
            "Error should mention unexpected response, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_ollama_embed_batch_rejects_invalid_json_response() {
        let (port, handle) = mock_ollama_server("not json".to_string(), "/api/embed");
        let endpoint = format!("http://127.0.0.1:{}", port);
        let provider = OllamaProvider::new("test-model", &endpoint, 768).unwrap();

        let result = provider.embed_batch(&["text1", "text2"]);
        let _ = handle.join();

        assert!(result.is_err(), "invalid JSON should return error");
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.contains("Unexpected Ollama response"),
            "Error should mention unexpected response, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_ollama_embed_connection_refused() {
        // Port 1 is almost certainly not listening
        let provider = OllamaProvider::new("test-model", "http://127.0.0.1:1", 768).unwrap();
        let result = provider.embed("test");
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.to_lowercase().contains("ollama") || err_msg.to_lowercase().contains("connect"),
            "Error should mention Ollama or connection, got: {}",
            err_msg
        );
    }

    // ── HTTP request/response tests ─────────────────────────────────────

    #[test]
    fn test_ollama_embed_sends_correct_payload() {
        let response = r#"{"embeddings":[[0.1,0.2,0.3]]}"#;
        let (port, handle) = mock_ollama_server_capture(response.to_string());
        let endpoint = format!("http://127.0.0.1:{}", port);
        let provider = OllamaProvider::new("test-model", &endpoint, 3).unwrap();

        let result = provider.embed("hello world");
        let request_body = handle.join().unwrap();

        assert!(
            result.is_ok(),
            "embed should succeed, got: {:?}",
            result.err()
        );
        let parsed: serde_json::Value =
            serde_json::from_str(&request_body).expect("request body should be valid JSON");
        assert_eq!(parsed["model"], "test-model");
        assert_eq!(parsed["input"], serde_json::json!(["hello world"]));

        let embedding = result.unwrap();
        assert_eq!(embedding, vec![0.1, 0.2, 0.3]);
    }

    #[test]
    fn test_ollama_embed_parses_single_embedding() {
        let response = r#"{"embeddings":[[0.5, -0.3, 0.8, 0.0]]}"#;
        let (port, handle) = mock_ollama_server(response.to_string(), "/api/embed");
        let endpoint = format!("http://127.0.0.1:{}", port);
        let provider = OllamaProvider::new("test-model", &endpoint, 4).unwrap();

        let result = provider.embed("test text");
        let _ = handle.join();

        let embedding = result.unwrap();
        assert_eq!(embedding.len(), 4);
        assert!((embedding[0] - 0.5).abs() < 1e-6);
        assert!((embedding[1] - (-0.3)).abs() < 1e-6);
    }

    #[test]
    fn test_ollama_embed_batch_sends_array_input() {
        let response = r#"{"embeddings":[[0.1,0.2],[0.3,0.4]]}"#;
        let (port, handle) = mock_ollama_server_capture(response.to_string());
        let endpoint = format!("http://127.0.0.1:{}", port);
        let provider = OllamaProvider::new("test-model", &endpoint, 2).unwrap();

        let result = provider.embed_batch(&["text one", "text two"]);
        let request_body = handle.join().unwrap();

        assert!(
            result.is_ok(),
            "embed_batch should succeed, got: {:?}",
            result.err()
        );

        let parsed: serde_json::Value =
            serde_json::from_str(&request_body).expect("request body should be valid JSON");
        assert_eq!(parsed["model"], "test-model");
        let input = parsed["input"].as_array().expect("input should be array");
        assert_eq!(input.len(), 2, "should send 2 texts in batch");
        assert_eq!(parsed["input"], serde_json::json!(["text one", "text two"]));

        let embeddings = result.unwrap();
        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0], vec![0.1, 0.2]);
        assert_eq!(embeddings[1], vec![0.3, 0.4]);
    }

    #[test]
    fn test_ollama_embed_batch_parses_multiple_embeddings() {
        let response = r#"{"embeddings":[[1.0,0.0],[0.0,1.0],[0.5,0.5]]}"#;
        let (port, handle) = mock_ollama_server(response.to_string(), "/api/embed");
        let endpoint = format!("http://127.0.0.1:{}", port);
        let provider = OllamaProvider::new("test-model", &endpoint, 2).unwrap();

        let result = provider.embed_batch(&["a", "b", "c"]);
        let _ = handle.join();

        let embeddings = result.unwrap();
        assert_eq!(embeddings.len(), 3);
        assert_eq!(embeddings[0], vec![1.0, 0.0]);
        assert_eq!(embeddings[1], vec![0.0, 1.0]);
        assert_eq!(embeddings[2], vec![0.5, 0.5]);
    }

    #[test]
    fn test_ollama_embed_model_not_found() {
        let response_body = r#"{"error":"model 'nonexistent' not found"}"#;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                stream.set_read_timeout(Some(MOCK_TIMEOUT)).ok();
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                let response = format!(
                    "HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        let endpoint = format!("http://127.0.0.1:{}", port);
        let provider = OllamaProvider::new("nonexistent", &endpoint, 768).unwrap();

        let result = provider.embed("test");
        let _ = handle.join();

        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.to_lowercase().contains("model")
                || err_msg.to_lowercase().contains("not found"),
            "Error should mention model not found, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_ollama_embed_empty_input() {
        let response = r#"{"embeddings":[[]]}"#;
        let (port, handle) = mock_ollama_server(response.to_string(), "/api/embed");
        let endpoint = format!("http://127.0.0.1:{}", port);
        let provider = OllamaProvider::new("test-model", &endpoint, 0).unwrap();

        let result = provider.embed("");
        let _ = handle.join();

        assert!(
            result.is_ok(),
            "empty input should succeed, got: {:?}",
            result.err()
        );
    }
}
