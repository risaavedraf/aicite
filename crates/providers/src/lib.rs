use common::CiteError;

pub mod gemini;
pub mod openai;

/// Embedding provider trait
pub trait EmbeddingProvider {
    fn embed(&self, text: &str) -> Result<Vec<f32>, CiteError>;
    fn model_id(&self) -> &str;
    fn provider_id(&self) -> &str;
}
