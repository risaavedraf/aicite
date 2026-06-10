use common::CiteError;

pub mod eval;
pub mod gemini;
pub mod ollama;
pub mod openai;

/// An embedding vector.
pub type Embedding = Vec<f32>;

/// Strategy for batching embedding requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchStrategy {
    /// Use the provider's native batch API (single request, multiple texts).
    Native,
    /// Rate-limited parallel execution.
    RateLimited {
        max_concurrent: usize,
        delay_ms: u64,
    },
    /// Sequential one-at-a-time execution (safe default).
    Sequential,
}

impl std::fmt::Display for BatchStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Native => write!(f, "native"),
            Self::RateLimited {
                max_concurrent,
                delay_ms,
            } => write!(f, "rate_limited({max_concurrent},{delay_ms}ms)"),
            Self::Sequential => write!(f, "sequential"),
        }
    }
}

/// Embedding provider trait
pub trait EmbeddingProvider {
    fn embed(&self, text: &str) -> Result<Embedding, CiteError>;

    /// Embed multiple texts in a batch. Default implementation calls
    /// `embed` sequentially for each input, preserving order.
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, CiteError> {
        texts.iter().map(|text| self.embed(text)).collect()
    }

    /// Return the batching strategy this provider supports.
    /// Default is `Sequential` (one-at-a-time).
    fn batch_strategy(&self) -> BatchStrategy {
        BatchStrategy::Sequential
    }

    fn model_id(&self) -> &str;
    fn provider_id(&self) -> &str;
}

#[cfg(test)]
mod tests {
    // PR7 RED: these tests expect `BatchStrategy`, `embed_batch`, and
    // `batch_strategy` to exist on the `EmbeddingProvider` trait. They are
    // intentionally failing in the current (RED) state. The matching GREEN
    // phase will add the enum + default trait methods, at which point these
    // tests should compile and pass.

    use super::gemini::GeminiProvider;
    use super::openai::OpenAICompatibleProvider;
    use super::*;
    use std::cell::{Cell, RefCell};

    /// Test provider that counts `embed` calls and returns a deterministic
    /// embedding derived from the input text length. The first element of
    /// the returned vector equals `text.len() as f32`, so tests can prove
    /// that input order is preserved through `embed_batch`.
    struct CountingProvider {
        model: String,
        call_count: Cell<usize>,
        call_log: RefCell<Vec<String>>,
    }

    impl CountingProvider {
        fn new(model: &str) -> Self {
            Self {
                model: model.to_string(),
                call_count: Cell::new(0),
                call_log: RefCell::new(Vec::new()),
            }
        }

        fn call_count(&self) -> usize {
            self.call_count.get()
        }

        fn recorded_inputs(&self) -> Vec<String> {
            self.call_log.borrow().clone()
        }
    }

    impl EmbeddingProvider for CountingProvider {
        fn embed(&self, text: &str) -> Result<Embedding, CiteError> {
            self.call_count.set(self.call_count.get() + 1);
            self.call_log.borrow_mut().push(text.to_string());
            Ok(vec![text.len() as f32, 0.0, 0.0])
        }

        fn model_id(&self) -> &str {
            &self.model
        }

        fn provider_id(&self) -> &str {
            "counting"
        }
    }

    // --- BatchStrategy enum tests ---

    #[test]
    fn test_batch_strategy_variants_are_distinguishable() {
        let sequential = BatchStrategy::Sequential;
        let native = BatchStrategy::Native;
        let rate_limited = BatchStrategy::RateLimited {
            max_concurrent: 2,
            delay_ms: 100,
        };

        assert_ne!(sequential, native, "Sequential must differ from Native");
        assert_ne!(
            sequential, rate_limited,
            "Sequential must differ from RateLimited"
        );
        assert_ne!(native, rate_limited, "Native must differ from RateLimited");
    }

    #[test]
    fn test_batch_strategy_derives_debug() {
        let strategy = BatchStrategy::RateLimited {
            max_concurrent: 4,
            delay_ms: 50,
        };
        let dbg = format!("{:?}", strategy);
        assert!(
            dbg.contains("RateLimited"),
            "Debug output missing variant name: {}",
            dbg
        );
        assert!(
            dbg.contains("max_concurrent"),
            "Debug output missing field name: {}",
            dbg
        );
        assert!(
            dbg.contains("delay_ms"),
            "Debug output missing field name: {}",
            dbg
        );
    }

    #[test]
    fn test_batch_strategy_derives_clone() {
        let rate_limited = BatchStrategy::RateLimited {
            max_concurrent: 8,
            delay_ms: 250,
        };
        let rate_limited_clone = rate_limited.clone();
        assert_eq!(rate_limited, rate_limited_clone);

        let native = BatchStrategy::Native;
        let native_clone = native.clone();
        assert_eq!(native, native_clone);

        let sequential = BatchStrategy::Sequential;
        let sequential_clone = sequential.clone();
        assert_eq!(sequential, sequential_clone);
    }

    #[test]
    fn test_batch_strategy_display() {
        assert_eq!(BatchStrategy::Native.to_string(), "native");
        assert_eq!(BatchStrategy::Sequential.to_string(), "sequential");
        assert_eq!(
            BatchStrategy::RateLimited {
                max_concurrent: 2,
                delay_ms: 100,
            }
            .to_string(),
            "rate_limited(2,100ms)"
        );
    }

    #[test]
    fn test_batch_strategy_derives_partial_eq_field_sensitive() {
        let baseline = BatchStrategy::RateLimited {
            max_concurrent: 2,
            delay_ms: 100,
        };
        let same = BatchStrategy::RateLimited {
            max_concurrent: 2,
            delay_ms: 100,
        };
        let different_concurrency = BatchStrategy::RateLimited {
            max_concurrent: 3,
            delay_ms: 100,
        };
        let different_delay = BatchStrategy::RateLimited {
            max_concurrent: 2,
            delay_ms: 200,
        };

        assert_eq!(baseline, same);
        assert_ne!(baseline, different_concurrency);
        assert_ne!(baseline, different_delay);
    }

    // --- Default embed_batch behavior tests ---

    #[test]
    fn test_default_embed_batch_returns_one_embedding_per_input() {
        let provider = CountingProvider::new("test-model");
        let result = provider
            .embed_batch(&["a", "b", "c"])
            .expect("default embed_batch should succeed");
        assert_eq!(
            result.len(),
            3,
            "expected 3 embeddings, got {}",
            result.len()
        );
    }

    #[test]
    fn test_default_embed_batch_invokes_embed_once_per_input() {
        let provider = CountingProvider::new("test-model");
        let _ = provider
            .embed_batch(&["a", "b", "c"])
            .expect("default embed_batch should succeed");
        assert_eq!(
            provider.call_count(),
            3,
            "default embed_batch must call embed for every input"
        );
    }

    #[test]
    fn test_default_embed_batch_preserves_input_order() {
        let provider = CountingProvider::new("test-model");
        let inputs = ["alpha", "beta", "gamma", "delta"];
        let result = provider
            .embed_batch(&inputs)
            .expect("default embed_batch should succeed");

        // First element of each embedding encodes the input length,
        // so we can prove order by checking that the encoded length
        // matches the corresponding input.
        for (input, embedding) in inputs.iter().zip(result.iter()) {
            assert_eq!(
                embedding[0],
                input.len() as f32,
                "embedding at index does not match input order"
            );
        }

        // The recorded call order must match the input order
        let recorded = provider.recorded_inputs();
        assert_eq!(
            recorded,
            vec!["alpha", "beta", "gamma", "delta"],
            "embed must be called in input order"
        );
    }

    #[test]
    fn test_default_embed_batch_with_empty_input_does_not_invoke_embed() {
        let provider = CountingProvider::new("test-model");
        let result = provider
            .embed_batch(&[])
            .expect("empty batch should succeed");
        assert!(result.is_empty(), "empty input must produce empty result");
        assert_eq!(
            provider.call_count(),
            0,
            "empty input must not invoke embed"
        );
    }

    // --- Default batch_strategy behavior test ---

    #[test]
    fn test_default_batch_strategy_is_sequential() {
        let provider = CountingProvider::new("test-model");
        assert_eq!(
            provider.batch_strategy(),
            BatchStrategy::Sequential,
            "default batch_strategy must be Sequential"
        );
    }

    // --- Existing providers compile and report expected strategy ---

    #[test]
    fn test_gemini_provider_reports_rate_limited_batch_strategy() {
        let provider = GeminiProvider::new("gemini-embedding-001", "test-key", 30)
            .expect("gemini provider should build with valid key");
        assert_eq!(
            provider.batch_strategy(),
            BatchStrategy::RateLimited {
                max_concurrent: 1,
                delay_ms: 0,
            }
        );
    }

    #[test]
    fn test_openai_compatible_provider_reports_sequential_batch_strategy() {
        let provider = OpenAICompatibleProvider::new(
            "https://api.openai.com/v1/embeddings",
            "text-embedding-3-small",
            "sk-test-key",
            30,
        )
        .expect("openai-compatible provider should build with valid args");
        assert_eq!(provider.batch_strategy(), BatchStrategy::Sequential);
    }
}
