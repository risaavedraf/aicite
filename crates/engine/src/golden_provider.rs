//! Topic-based mock embedding provider for deterministic evaluation.
//!
//! Returns 8-dimensional vectors where dimensions map to semantic topics.

use common::CiteError;
use providers::eval::EvalProvider;
use providers::EmbeddingProvider;
use std::collections::HashMap;

/// Deterministic embedding provider that maps text to topic-based vectors.
///
/// Used by the golden-dataset evaluation and the `cite evaluate` CLI command.
pub struct GoldenProvider {
    cache: HashMap<String, Vec<f32>>,
}

impl GoldenProvider {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }


}

impl Default for GoldenProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddingProvider for GoldenProvider {
    fn embed(&self, text: &str) -> Result<Vec<f32>, CiteError> {
        let key = normalize_key(text);
        if let Some(cached) = self.cache.get(&key) {
            return Ok(cached.clone());
        }
        Ok(EvalProvider::compute_vector(text))
    }

    fn model_id(&self) -> &str {
        "golden-eval-v1"
    }

    fn provider_id(&self) -> &str {
        "golden"
    }
}

fn normalize_key(text: &str) -> String {
    text.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}


