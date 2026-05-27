pub mod chunker;
pub mod extractor;
pub mod validator;

/// Ingest handles document extraction, chunking, and embedding
pub struct Ingest;

impl Ingest {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Ingest {
    fn default() -> Self {
        Self::new()
    }
}
