use common::CiteError;
use providers::eval::EvalProvider;
use providers::EmbeddingProvider;
use std::collections::HashMap;

/// Topic-based mock embedding provider for deterministic evaluation.
///
/// Returns 8-dimensional vectors where dimensions map to semantic topics:
/// - dim 0: API/gateway
/// - dim 1: database/storage
/// - dim 2: auth/security/passwords
/// - dim 3: logging/monitoring
/// - dim 4: users/CRUD
/// - dim 5: error handling/rate limiting
/// - dim 6: compliance/policy
/// - dim 7: general/noise (low similarity to everything)
pub struct GoldenProvider {
    cache: HashMap<String, Vec<f32>>,
}

impl GoldenProvider {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Pre-populate cache with known chunk embeddings
    #[allow(dead_code)]
    pub fn with_embeddings(embeddings: Vec<(String, Vec<f32>)>) -> Self {
        let mut cache = HashMap::new();
        for (text, vec) in embeddings {
            cache.insert(normalize_key(&text), vec);
        }
        Self { cache }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_golden_provider_deterministic() {
        let provider = GoldenProvider::new();
        let v1 = provider.embed("test text").unwrap();
        let v2 = provider.embed("test text").unwrap();
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_golden_provider_api_topic() {
        let provider = GoldenProvider::new();
        let v = provider
            .embed("The API gateway routes all external requests")
            .unwrap();
        // dim 0 (API) should be dominant
        assert!(v[0] > v[1]);
        assert!(v[0] > v[7]);
    }

    #[test]
    fn test_golden_provider_db_topic() {
        let provider = GoldenProvider::new();
        let v = provider.embed("PostgreSQL with read replicas").unwrap();
        assert!(v[1] > v[0]);
    }

    #[test]
    fn test_golden_provider_unknown_text() {
        let provider = GoldenProvider::new();
        let v = provider.embed("quantum computing relativity").unwrap();
        // Most dimensions should be zero (only noise dim might be non-zero)
        let non_zero = v.iter().filter(|x| **x > 0.0).count();
        assert!(non_zero <= 1); // at most noise dimension
    }

    #[test]
    fn test_golden_provider_cosine_similarity() {
        let provider = GoldenProvider::new();
        let api_chunk = provider
            .embed("The API gateway routes external requests to microservices")
            .unwrap();
        let api_query = provider.embed("What does the API gateway do?").unwrap();
        let unrelated = provider.embed("quantum computing").unwrap();

        // API query should be similar to API chunk
        let sim_related = cosine(&api_chunk, &api_query);
        let sim_unrelated = cosine(&api_chunk, &unrelated);

        assert!(sim_related > sim_unrelated);
        assert!(sim_related > 0.5);
        assert!(sim_unrelated < 0.3);
    }

    fn cosine(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if na == 0.0 || nb == 0.0 {
            0.0
        } else {
            dot / (na * nb)
        }
    }
}
