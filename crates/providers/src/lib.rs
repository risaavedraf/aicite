use common::CiteError;

pub mod eval;
pub mod gemini;
pub mod openai;

/// An embedding vector.
pub type Embedding = Vec<f32>;

/// Embedding provider trait
pub trait EmbeddingProvider {
    fn embed(&self, text: &str) -> Result<Embedding, CiteError>;
    fn model_id(&self) -> &str;
    fn provider_id(&self) -> &str;
}
