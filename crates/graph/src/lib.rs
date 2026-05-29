pub mod heading_parser;
pub mod hierarchy;
pub mod types;
pub use hierarchy::{ConceptWithChunks, HierarchyResult, TopicWithConcepts};
pub use types::{Concept, HeadingSpan, SemanticLink, Topic};

/// Graph manages document/section/chunk relationships
pub struct Graph;

impl Graph {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}
