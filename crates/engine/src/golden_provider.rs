//! Topic-based mock embedding provider for deterministic evaluation.
//!
//! Returns 8-dimensional vectors where dimensions map to semantic topics.

use common::CiteError;
use providers::EmbeddingProvider;
use std::collections::HashMap;

const DIM: usize = 8;

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

    fn compute_vector(text: &str) -> Vec<f32> {
        let lower = text.to_lowercase();
        let mut vec = vec![0.0f32; DIM];

        // dim 0: API/gateway/architecture
        if contains_any(
            &lower,
            &[
                "api gateway",
                "routes",
                "external requests",
                "microservices",
                "endpoint",
                "architecture",
                "system design",
            ],
        ) {
            vec[0] = 0.9;
        }

        // dim 1: database/storage
        if contains_any(
            &lower,
            &[
                "postgresql",
                "database",
                "read replicas",
                "storage",
                "data layer",
            ],
        ) {
            vec[1] = 0.9;
        }

        // dim 2: auth/security/passwords/encryption
        if contains_any(
            &lower,
            &[
                "jwt",
                "authentication",
                "password",
                "token",
                "encrypt",
                "aes-256",
                "tls",
                "security",
                "secure",
                "credential",
            ],
        ) {
            vec[2] = 0.9;
        }

        // dim 3: logging/monitoring
        if contains_any(
            &lower,
            &[
                "logging",
                "audit",
                "monitor",
                "structured json logs",
                "elk",
                "retained",
            ],
        ) {
            vec[3] = 0.9;
        }

        // dim 4: users/CRUD
        if contains_any(
            &lower,
            &[
                "users",
                "get /users",
                "post /users",
                "create user",
                "paginated list",
            ],
        ) {
            vec[4] = 0.9;
        }

        // dim 5: error handling/rate limiting
        if contains_any(
            &lower,
            &[
                "rate limit",
                "429",
                "retry-after",
                "error code",
                "too many requests",
            ],
        ) {
            vec[5] = 0.9;
        }

        // dim 6: compliance/policy/injection
        if contains_any(
            &lower,
            &[
                "policy",
                "compliance",
                "security policy",
                "data classification",
                "incident",
                "ignore",
                "instructions",
                "prompt",
                "injection",
            ],
        ) {
            vec[6] = 0.85;
        }

        // Add small noise to non-zero dimensions for realism
        for v in vec.iter_mut().take(DIM) {
            if *v > 0.0 {
                *v += 0.05;
            }
        }

        // Normalize to unit vector
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut vec {
                *x /= norm;
            }
        }

        vec
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
        Ok(Self::compute_vector(text))
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

fn contains_any(text: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|kw| text.contains(kw))
}
