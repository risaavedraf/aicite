//! Shared evaluation embedding provider for deterministic testing.
//!
//! Returns 8-dimensional topic-based vectors where dimensions map to semantic topics:
//! - dim 0: API/gateway
//! - dim 1: database/storage
//! - dim 2: auth/security/passwords
//! - dim 3: logging/monitoring
//! - dim 4: users/CRUD
//! - dim 5: error handling/rate limiting
//! - dim 6: compliance/policy
//! - dim 7: general/noise

use crate::EmbeddingProvider;
use common::CiteError;

const DIM: usize = 8;

/// Topic-based mock embedding provider for deterministic evaluation.
///
/// Shared between CLI evaluate command and engine golden tests.
pub struct EvalProvider;

impl EvalProvider {
    /// Compute a topic-based vector from text content.
    /// Uses keyword detection to assign weights to each dimension.
    pub(crate) fn compute_vector(text: &str) -> Vec<f32> {
        let lower = text.to_lowercase();
        let mut vec = vec![0.0f32; DIM];

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

        // Add small noise to non-zero dimensions
        for v in vec.iter_mut() {
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

impl EmbeddingProvider for EvalProvider {
    fn embed(&self, text: &str) -> Result<Vec<f32>, CiteError> {
        Ok(Self::compute_vector(text))
    }
    fn model_id(&self) -> &str {
        "eval-v1"
    }
    fn provider_id(&self) -> &str {
        "eval"
    }
}

fn contains_any(text: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|kw| text.contains(kw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_provider_deterministic() {
        let provider = EvalProvider;
        let v1 = provider.embed("test query").unwrap();
        let v2 = provider.embed("test query").unwrap();
        assert_eq!(v1, v2);
        assert_eq!(v1.len(), DIM);
    }

    #[test]
    fn test_eval_provider_api_topic() {
        let provider = EvalProvider;
        let v = provider
            .embed("The API gateway routes all external requests")
            .unwrap();
        assert!(v[0] > v[1]);
        assert!(v[0] > v[7]);
    }

    #[test]
    fn test_eval_provider_db_topic() {
        let provider = EvalProvider;
        let v = provider.embed("PostgreSQL with read replicas").unwrap();
        assert!(v[1] > v[0]);
    }

    #[test]
    fn test_eval_provider_unknown_text() {
        let provider = EvalProvider;
        let v = provider.embed("quantum computing relativity").unwrap();
        let non_zero = v.iter().filter(|x| **x > 0.0).count();
        assert!(non_zero <= 1);
    }
}
